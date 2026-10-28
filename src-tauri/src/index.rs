use std::{
    collections::{HashSet, VecDeque},
    sync::{atomic::Ordering, Arc},
    time::Instant,
};

use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::{catalog, db, norm, state::AppState};

#[derive(Debug, Serialize)]
pub struct IndexStats {
    pub dirs: i64,
    pub files: i64,
    pub cache_bytes: i64,
    pub cached_dirs: i64,
    pub last_bootstrap: Option<String>,
    pub last_crawl: Option<String>,
    pub mode: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Hit {
    pub kind: String,
    pub path: String,
    pub name: String,
    pub parent: String,
    pub section: Option<String>,
    pub year: Option<i64>,
    pub ext: Option<String>,
    pub size: Option<i64>,
    pub mtime: Option<String>,
    pub score: i64,
}

#[derive(Debug, Serialize)]
pub struct SearchPage {
    pub hits: Vec<Hit>,
    pub total: i64,
    pub took_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub name: String,
    pub path: String,
    pub kind: String,
    pub ext: Option<String>,
    pub size: Option<i64>,
    pub mtime: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Listing {
    pub path: String,
    pub entries: Vec<Entry>,
    pub fetched_at: String,
    pub cached: bool,
}

#[derive(Debug, Default, Deserialize)]
pub struct SearchFilter {
    pub section: Option<String>,
    pub year: Option<i64>,
    pub kind: Option<String>,
    pub ext: Option<String>,
}

fn ttl_hours(state: &AppState, key: &str, fallback: i64) -> i64 {
    let conn = match state.app.lock() {
        Ok(conn) => conn,
        Err(_) => return fallback,
    };
    db::get_setting(&conn, key)
        .and_then(|value| value.parse().ok())
        .unwrap_or(fallback)
}

fn max_cache_bytes(state: &AppState) -> i64 {
    ttl_hours(state, "cache_max_mb", 64) * 1024 * 1024
}

fn extension_of(name: &str) -> Option<String> {
    name.rsplit_once('.')
        .map(|(_, ext)| ext.to_lowercase())
        .filter(|ext| ext.len() <= 5 && !ext.contains(' '))
}

pub fn stats(state: &AppState) -> Result<IndexStats, String> {
    let conn = state
        .cache
        .lock()
        .map_err(|_| "cache bloqueada".to_string())?;
    let dirs: i64 = conn
        .query_row("SELECT COUNT(*) FROM dir_index", [], |row| row.get(0))
        .unwrap_or(0);
    let files: i64 = conn
        .query_row("SELECT COUNT(*) FROM file_index", [], |row| row.get(0))
        .unwrap_or(0);
    let cache_bytes: i64 = conn
        .query_row("SELECT COALESCE(SUM(bytes), 0) FROM dir_cache", [], |row| {
            row.get(0)
        })
        .unwrap_or(0);
    let cached_dirs: i64 = conn
        .query_row("SELECT COUNT(*) FROM dir_cache", [], |row| row.get(0))
        .unwrap_or(0);
    let last_bootstrap = db::get_meta(&conn, "last_bootstrap");
    let last_crawl = db::get_meta(&conn, "last_crawl");
    drop(conn);

    let mode = state
        .app
        .lock()
        .ok()
        .and_then(|conn| db::get_setting(&conn, "index_mode"))
        .unwrap_or_else(|| "light".to_string());

    Ok(IndexStats {
        dirs,
        files,
        cache_bytes,
        cached_dirs,
        last_bootstrap,
        last_crawl,
        mode,
    })
}

/// Carga el arbol publicado por el sitio (`listado.html`) en el indice de directorios.
pub async fn bootstrap(
    app: &AppHandle,
    state: &Arc<AppState>,
    force: bool,
) -> Result<IndexStats, String> {
    let ttl = ttl_hours(state, "bootstrap_ttl_hours", 24);

    if !force {
        let fresh = {
            let conn = state
                .cache
                .lock()
                .map_err(|_| "cache bloqueada".to_string())?;
            let dirs: i64 = conn
                .query_row("SELECT COUNT(*) FROM dir_index", [], |row| row.get(0))
                .unwrap_or(0);
            let last = db::get_meta(&conn, "last_bootstrap");
            let fresh = last
                .as_deref()
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                .map(|dt| (chrono::Utc::now() - dt.with_timezone(&chrono::Utc)).num_hours() < ttl)
                .unwrap_or(false);
            dirs > 1000 && fresh
        };
        if fresh {
            return stats(state);
        }
    }

    let _ = app.emit(
        "index://progress",
        serde_json::json!({ "phase": "bootstrap", "state": "start" }),
    );
    let html = catalog::fetch_text(&state.http, &format!("{}/listado.html", catalog::BASE)).await?;
    let paths = catalog::parse_listado(&html);
    let total = paths.len();

    {
        let mut conn = state
            .cache
            .lock()
            .map_err(|_| "cache bloqueada".to_string())?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        for path in &paths {
            let name = catalog::decode_name(&db::name_of(path));
            let parent = db::parent_of(path);
            let section = db::section_of(path);
            let year = norm::extract_year(&name);
            db::upsert_dir_row(&tx, path, &name, &parent, &section, year, "bootstrap")?;
        }
        db::rebuild_dir_fts(&tx)?;
        db::set_meta(&tx, "last_bootstrap", &db::now_iso());
        tx.commit().map_err(|e| e.to_string())?;
    }

    let _ = app.emit(
        "index://progress",
        serde_json::json!({ "phase": "bootstrap", "state": "done", "dirs": total }),
    );

    stats(state)
}

pub fn search(
    state: &Arc<AppState>,
    query: &str,
    filter: &SearchFilter,
    limit: i64,
    offset: i64,
) -> Result<SearchPage, String> {
    let started = Instant::now();
    let normalized = norm::normalize(query);
    if normalized.is_empty() {
        return Ok(SearchPage {
            hits: Vec::new(),
            total: 0,
            took_ms: 0,
        });
    }
    let Some(fts) = norm::fts_query(query) else {
        return Ok(SearchPage {
            hits: Vec::new(),
            total: 0,
            took_ms: 0,
        });
    };

    let conn = state
        .cache
        .lock()
        .map_err(|_| "cache bloqueada".to_string())?;
    let fetch_limit = (offset + limit).clamp(50, 400);
    let mut hits: Vec<Hit> = Vec::new();
    let mut total = 0_i64;

    let include_files = filter.kind.as_deref() != Some("dir");
    let include_dirs = filter.kind.as_deref() != Some("file") && filter.ext.is_none();

    if include_files {
        let mut statement = conn
            .prepare(
                "SELECT i.path, i.name, i.parent, i.section, i.year, i.ext, i.size, i.mtime
                 FROM file_fts f JOIN file_index i ON i.path = f.path
                 WHERE file_fts MATCH ?1
                   AND (?2 IS NULL OR i.section = ?2)
                   AND (?3 IS NULL OR i.year = ?3)
                   AND (?4 IS NULL OR i.ext = ?4)
                 ORDER BY bm25(file_fts)
                 LIMIT ?5",
            )
            .map_err(|e| e.to_string())?;

        let rows = statement
            .query_map(
                params![fts, filter.section, filter.year, filter.ext, fetch_limit],
                |row| {
                    Ok(Hit {
                        kind: "file".to_string(),
                        path: row.get(0)?,
                        name: row.get(1)?,
                        parent: row.get(2)?,
                        section: row.get(3)?,
                        year: row.get(4)?,
                        ext: row.get(5)?,
                        size: row.get(6)?,
                        mtime: row.get(7)?,
                        score: 0,
                    })
                },
            )
            .map_err(|e| e.to_string())?;

        for (index, row) in rows.enumerate() {
            let mut hit = row.map_err(|e| e.to_string())?;
            hit.score = index as i64;
            hits.push(hit);
        }

        total += conn
            .query_row(
                "SELECT COUNT(*) FROM file_fts f JOIN file_index i ON i.path = f.path
                 WHERE file_fts MATCH ?1
                   AND (?2 IS NULL OR i.section = ?2)
                   AND (?3 IS NULL OR i.year = ?3)
                   AND (?4 IS NULL OR i.ext = ?4)",
                params![fts, filter.section, filter.year, filter.ext],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0);
    }

    if include_dirs {
        let mut statement = conn
            .prepare(
                "SELECT d.path, d.name, d.parent, d.section, d.year
                 FROM dir_fts f JOIN dir_index d ON d.path = f.path
                 WHERE dir_fts MATCH ?1
                   AND (?2 IS NULL OR d.section = ?2)
                   AND (?3 IS NULL OR d.year = ?3)
                 ORDER BY bm25(dir_fts)
                 LIMIT ?4",
            )
            .map_err(|e| e.to_string())?;

        let rows = statement
            .query_map(
                params![fts, filter.section, filter.year, fetch_limit],
                |row| {
                    Ok(Hit {
                        kind: "dir".to_string(),
                        path: row.get(0)?,
                        name: row.get(1)?,
                        parent: row.get(2)?,
                        section: row.get(3)?,
                        year: row.get(4)?,
                        ext: None,
                        size: None,
                        mtime: None,
                        score: 0,
                    })
                },
            )
            .map_err(|e| e.to_string())?;

        for (index, row) in rows.enumerate() {
            let mut hit = row.map_err(|e| e.to_string())?;
            hit.score = 10_000 + index as i64;
            hits.push(hit);
        }

        total += conn
            .query_row(
                "SELECT COUNT(*) FROM dir_fts f JOIN dir_index d ON d.path = f.path
                 WHERE dir_fts MATCH ?1
                   AND (?2 IS NULL OR d.section = ?2)
                   AND (?3 IS NULL OR d.year = ?3)",
                params![fts, filter.section, filter.year],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0);
    }

    hits.sort_by_key(|hit| hit.score);
    let start = offset.max(0) as usize;
    let end = (start + limit.max(1) as usize).min(hits.len());
    let page = if start < hits.len() {
        hits[start..end].to_vec()
    } else {
        Vec::new()
    };

    drop(conn);

    if let Ok(conn) = state.app.lock() {
        let _ = conn.execute(
            "INSERT INTO searches(query, hits, at) VALUES(?1, ?2, ?3)",
            params![query, total, db::now_iso()],
        );
    }

    Ok(SearchPage {
        hits: page,
        total,
        took_ms: started.elapsed().as_millis() as i64,
    })
}

fn prune_cache(conn: &rusqlite::Connection, max_bytes: i64) -> Result<(), String> {
    let total: i64 = conn
        .query_row("SELECT COALESCE(SUM(bytes), 0) FROM dir_cache", [], |row| {
            row.get(0)
        })
        .unwrap_or(0);
    if total <= max_bytes {
        return Ok(());
    }
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM dir_cache", [], |row| row.get(0))
        .unwrap_or(0);
    let victims = (count / 4).max(1);
    conn.execute(
        "DELETE FROM dir_cache WHERE path IN (
            SELECT path FROM dir_cache ORDER BY fetched_at ASC LIMIT ?1
         )",
        [victims],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn browse(state: &Arc<AppState>, path: &str, force: bool) -> Result<Listing, String> {
    let ttl = ttl_hours(state, "cache_ttl_hours", 12);
    let mut target = path.to_string();
    if target.is_empty() {
        target = "/".to_string();
    }
    if !target.starts_with('/') {
        target.insert(0, '/');
    }
    if !target.ends_with('/') {
        target.push('/');
    }

    if !force {
        let cached = {
            let conn = state
                .cache
                .lock()
                .map_err(|_| "cache bloqueada".to_string())?;
            conn.query_row(
                "SELECT payload, fetched_at FROM dir_cache WHERE path = ?1",
                [&target],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .ok()
        };

        if let Some((payload, fetched_at)) = cached {
            let fresh = chrono::DateTime::parse_from_rfc3339(&fetched_at)
                .map(|dt| (chrono::Utc::now() - dt.with_timezone(&chrono::Utc)).num_hours() < ttl)
                .unwrap_or(false);
            if fresh {
                if let Ok(entries) = serde_json::from_str::<Vec<Entry>>(&payload) {
                    return Ok(Listing {
                        path: target,
                        entries,
                        fetched_at,
                        cached: true,
                    });
                }
            }
        }
    }

    let raw = catalog::fetch_listing(&state.http, &target).await?;
    let now = db::now_iso();
    let section = db::section_of(&target);
    let year = norm::extract_year(&catalog::decode_name(&target));

    let mut entries: Vec<Entry> = Vec::with_capacity(raw.len());
    for item in &raw {
        let child_path = format!("{}{}", target, item.href);
        if item.is_dir {
            entries.push(Entry {
                name: item.name.clone(),
                path: child_path,
                kind: "dir".to_string(),
                ext: None,
                size: None,
                mtime: item.mtime.clone(),
            });
        } else {
            entries.push(Entry {
                name: item.name.clone(),
                path: child_path,
                kind: "file".to_string(),
                ext: extension_of(&item.name),
                size: item.size,
                mtime: item.mtime.clone(),
            });
        }
    }

    {
        let mut conn = state
            .cache
            .lock()
            .map_err(|_| "cache bloqueada".to_string())?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;

        for entry in &entries {
            if entry.kind == "dir" {
                db::upsert_dir(
                    &tx,
                    &entry.path,
                    &entry.name,
                    &target,
                    &section,
                    norm::extract_year(&entry.name).or(year),
                    "browse",
                )?;
            } else {
                db::upsert_file(
                    &tx,
                    &entry.path,
                    &entry.name,
                    &target,
                    &section,
                    year,
                    entry.ext.as_deref(),
                    entry.size,
                    entry.mtime.as_deref(),
                )?;
            }
        }

        let payload = serde_json::to_string(&entries).map_err(|e| e.to_string())?;
        let bytes = payload.len() as i64;
        tx.execute(
            "INSERT INTO dir_cache(path, payload, bytes, fetched_at) VALUES(?1, ?2, ?3, ?4)
             ON CONFLICT(path) DO UPDATE SET
                payload = excluded.payload,
                bytes = excluded.bytes,
                fetched_at = excluded.fetched_at",
            params![target, payload, bytes, now],
        )
        .map_err(|e| e.to_string())?;

        prune_cache(&tx, max_cache_bytes(state))?;
        tx.commit().map_err(|e| e.to_string())?;
    }

    Ok(Listing {
        path: target,
        entries,
        fetched_at: now,
        cached: false,
    })
}

pub fn cache_clear(state: &Arc<AppState>, keep_index: bool) -> Result<IndexStats, String> {
    let conn = state
        .cache
        .lock()
        .map_err(|_| "cache bloqueada".to_string())?;
    conn.execute("DELETE FROM dir_cache", [])
        .map_err(|e| e.to_string())?;
    if !keep_index {
        conn.execute("DELETE FROM dir_fts", [])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM file_fts", [])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM dir_index", [])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM file_index", [])
            .map_err(|e| e.to_string())?;
        let _ = conn.execute("DELETE FROM meta WHERE key LIKE 'last_%'", []);
    }
    drop(conn);
    stats(state)
}

/// Crawl completo (modo profundo). Recorre el arbol remoto y llena el indice de archivos.
pub async fn crawl(
    app: AppHandle,
    state: Arc<AppState>,
    max_dirs: usize,
) -> Result<IndexStats, String> {
    state.crawl_stop.store(false, Ordering::Relaxed);
    let mut queue: VecDeque<String> = VecDeque::from(["/".to_string()]);
    let mut visited: HashSet<String> = HashSet::new();
    let mut processed = 0_usize;
    let mut file_count = 0_usize;
    let mut errors = 0_usize;

    while !queue.is_empty() && processed < max_dirs && !state.crawl_stop.load(Ordering::Relaxed) {
        let mut batch: Vec<String> = Vec::new();
        while batch.len() < 6 {
            match queue.pop_front() {
                Some(path) => {
                    if visited.insert(path.clone()) {
                        batch.push(path);
                    }
                }
                None => break,
            }
        }
        if batch.is_empty() {
            break;
        }

        let futures = batch
            .iter()
            .map(|path| catalog::fetch_listing(&state.http, path));
        let results = futures::future::join_all(futures).await;

        {
            let mut conn = state
                .cache
                .lock()
                .map_err(|_| "cache bloqueada".to_string())?;
            let tx = conn.transaction().map_err(|e| e.to_string())?;
            for (path, result) in batch.iter().zip(results) {
                match result {
                    Ok(entries) => {
                        let section = db::section_of(path);
                        let year = norm::extract_year(&catalog::decode_name(path));
                        for entry in entries {
                            let child = format!("{}/{}", path.trim_end_matches('/'), entry.href);
                            if entry.is_dir {
                                db::upsert_dir(
                                    &tx,
                                    &child,
                                    &entry.name,
                                    path,
                                    &section,
                                    norm::extract_year(&entry.name).or(year),
                                    "crawl",
                                )?;
                                if !visited.contains(&child) {
                                    queue.push_back(child);
                                }
                            } else {
                                db::upsert_file(
                                    &tx,
                                    &child,
                                    &entry.name,
                                    path,
                                    &section,
                                    year,
                                    extension_of(&entry.name).as_deref(),
                                    entry.size,
                                    entry.mtime.as_deref(),
                                )?;
                                file_count += 1;
                            }
                        }
                    }
                    Err(_) => errors += 1,
                }
            }
            db::set_meta(&tx, "last_crawl", &db::now_iso());
            tx.commit().map_err(|e| e.to_string())?;
        }

        processed += batch.len();
        let _ = app.emit(
            "crawl://progress",
            serde_json::json!({
                "dirs": processed,
                "queued": queue.len(),
                "files": file_count,
                "errors": errors,
                "max": max_dirs,
            }),
        );
    }

    stats(&state)
}
