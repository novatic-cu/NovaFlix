//! Índice local: bootstrap desde `listado.html`, búsqueda FTS5, exploración con caché y rastreo.

use std::{
    collections::{HashSet, VecDeque},
    sync::{atomic::Ordering, Arc},
    time::Instant,
};

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::{
    catalog, db,
    error::{lock, Result, UiError},
    norm, Core, Event, Level, Progress,
};

#[derive(Debug, Clone, Serialize)]
pub struct IndexStats {
    pub dirs: i64,
    pub files: i64,
    pub cache_bytes: i64,
    pub cached_dirs: i64,
    pub last_bootstrap: Option<String>,
    pub last_crawl: Option<String>,
    pub mode: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Dir,
    File,
}

#[derive(Debug, Clone, Serialize)]
pub struct Hit {
    pub kind: Kind,
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

#[derive(Debug, Clone, Serialize)]
pub struct SearchPage {
    pub hits: Vec<Hit>,
    pub total: i64,
    pub took_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub name: String,
    pub path: String,
    pub kind: Kind,
    pub ext: Option<String>,
    pub size: Option<i64>,
    pub mtime: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Listing {
    pub path: String,
    pub entries: Vec<Entry>,
    pub fetched_at: String,
    pub cached: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SectionInfo {
    pub name: String,
    pub path: String,
    pub dirs: i64,
}

#[derive(Debug, Clone, Default)]
pub struct SearchFilter {
    pub section: Option<String>,
    pub year: Option<i64>,
    pub kind: Option<Kind>,
    pub ext: Option<String>,
}

fn extension_of(name: &str) -> Option<String> {
    name.rsplit_once('.')
        .map(|(_, ext)| ext.to_lowercase())
        .filter(|ext| ext.len() <= 5 && !ext.contains(' '))
}

fn canonical_dir(path: &str) -> String {
    let mut target = if path.is_empty() {
        "/".to_string()
    } else {
        path.to_string()
    };
    if !target.starts_with('/') {
        target.insert(0, '/');
    }
    if !target.ends_with('/') {
        target.push('/');
    }
    target
}

fn is_fresh(fetched_at: &str, ttl_hours: i64) -> bool {
    chrono::DateTime::parse_from_rfc3339(fetched_at)
        .map(|dt| (chrono::Utc::now() - dt.with_timezone(&chrono::Utc)).num_hours() < ttl_hours)
        .unwrap_or(false)
}

fn prune_cache(conn: &rusqlite::Connection, max_bytes: i64) -> Result<()> {
    let total: i64 = conn
        .query_row("SELECT COALESCE(SUM(bytes), 0) FROM dir_cache", [], |r| {
            r.get(0)
        })
        .unwrap_or(0);
    if total <= max_bytes {
        return Ok(());
    }
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM dir_cache", [], |r| r.get(0))
        .unwrap_or(0);
    conn.execute(
        "DELETE FROM dir_cache WHERE path IN (
            SELECT path FROM dir_cache ORDER BY fetched_at ASC LIMIT ?1
         )",
        [(count / 4).max(1)],
    )?;
    Ok(())
}

impl Core {
    pub fn index_stats(&self) -> Result<IndexStats> {
        let (dirs, files, cache_bytes, cached_dirs, last_bootstrap, last_crawl) = {
            let conn = lock(&self.cache)?;
            let count = |sql: &str| conn.query_row(sql, [], |r| r.get::<_, i64>(0)).unwrap_or(0);
            (
                count("SELECT COUNT(*) FROM dir_index"),
                count("SELECT COUNT(*) FROM file_index"),
                count("SELECT COALESCE(SUM(bytes), 0) FROM dir_cache"),
                count("SELECT COUNT(*) FROM dir_cache"),
                db::get_meta(&conn, "last_bootstrap"),
                db::get_meta(&conn, "last_crawl"),
            )
        };
        let mode =
            db::get_setting(&*lock(&self.app)?, "index_mode").unwrap_or_else(|| "light".into());
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

    /// Carga `listado.html` en el índice (o lo deja como está si sigue fresco).
    /// Emite `IndexProgress{Start}`, `IndexProgress{Done}` e `IndexReady`.
    pub fn bootstrap(self: &Arc<Self>, force: bool) {
        let core = self.clone();
        self.rt.spawn(async move {
            match core.bootstrap_inner(force).await {
                Ok(stats) => core.emit(Event::IndexReady(stats)),
                Err(err) => core.notice(
                    Level::Error,
                    format!("No se pudo preparar el catálogo: {err}"),
                ),
            }
        });
    }

    async fn bootstrap_inner(self: &Arc<Self>, force: bool) -> Result<IndexStats> {
        let ttl = self.settings()?.bootstrap_ttl_hours as i64;
        if !force {
            let conn = lock(&self.cache)?;
            let dirs: i64 = conn
                .query_row("SELECT COUNT(*) FROM dir_index", [], |r| r.get(0))
                .unwrap_or(0);
            let fresh = db::get_meta(&conn, "last_bootstrap")
                .map(|v| is_fresh(&v, ttl))
                .unwrap_or(false);
            drop(conn);
            if dirs > 1000 && fresh {
                return self.index_stats();
            }
        }

        self.emit(Event::IndexProgress {
            state: Progress::Start,
            dirs: None,
        });
        let net = self.net();
        let paths = match net.fetch_listado().await {
            Ok(html) => catalog::parse_listado(&html),
            // Sin `listado.html` pero con FTP: rastreo superficial (spec §8).
            Err(err) => {
                let shallow = shallow_tree(&net, 3).await;
                if shallow.is_empty() {
                    return Err(err);
                }
                shallow
            }
        };
        let total = paths.len();
        {
            let mut conn = lock(&self.cache)?;
            let tx = conn.transaction()?;
            for path in &paths {
                let name = catalog::decode_name(&db::name_of(path));
                db::upsert_dir_row(
                    &tx,
                    path,
                    &name,
                    &db::parent_of(path),
                    &db::section_of(path),
                    norm::extract_year(&name),
                    "bootstrap",
                )?;
            }
            db::rebuild_dir_fts(&tx)?;
            db::set_meta(&tx, "last_bootstrap", &db::now_iso());
            tx.commit()?;
        }
        self.emit(Event::IndexProgress {
            state: Progress::Done,
            dirs: Some(total),
        });
        self.index_stats()
    }

    pub fn search(
        &self,
        query: &str,
        filter: &SearchFilter,
        limit: i64,
        offset: i64,
    ) -> Result<SearchPage> {
        let started = Instant::now();
        let empty = SearchPage {
            hits: Vec::new(),
            total: 0,
            took_ms: 0,
        };
        let Some(fts) = norm::fts_query(query) else {
            return Ok(empty);
        };

        let conn = lock(&self.cache)?;
        let fetch_limit = (offset + limit).clamp(50, 400);
        let mut hits: Vec<Hit> = Vec::new();
        let mut total = 0_i64;
        let include_files = filter.kind != Some(Kind::Dir);
        let include_dirs = filter.kind != Some(Kind::File) && filter.ext.is_none();

        if include_files {
            let mut stmt = conn.prepare(
                "SELECT i.path, i.name, i.parent, i.section, i.year, i.ext, i.size, i.mtime
                 FROM file_fts f JOIN file_index i ON i.path = f.path
                 WHERE file_fts MATCH ?1
                   AND (?2 IS NULL OR i.section = ?2)
                   AND (?3 IS NULL OR i.year = ?3)
                   AND (?4 IS NULL OR i.ext = ?4)
                 ORDER BY bm25(file_fts)
                 LIMIT ?5",
            )?;
            let rows = stmt.query_map(
                params![fts, filter.section, filter.year, filter.ext, fetch_limit],
                |row| {
                    Ok(Hit {
                        kind: Kind::File,
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
            )?;
            for (rank, row) in rows.enumerate() {
                let mut hit = row?;
                hit.score = rank as i64;
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
                    |r| r.get::<_, i64>(0),
                )
                .unwrap_or(0);
        }

        if include_dirs {
            let mut stmt = conn.prepare(
                "SELECT d.path, d.name, d.parent, d.section, d.year
                 FROM dir_fts f JOIN dir_index d ON d.path = f.path
                 WHERE dir_fts MATCH ?1
                   AND (?2 IS NULL OR d.section = ?2)
                   AND (?3 IS NULL OR d.year = ?3)
                 ORDER BY bm25(dir_fts)
                 LIMIT ?4",
            )?;
            let rows = stmt.query_map(
                params![fts, filter.section, filter.year, fetch_limit],
                |row| {
                    Ok(Hit {
                        kind: Kind::Dir,
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
            )?;
            for (rank, row) in rows.enumerate() {
                let mut hit = row?;
                hit.score = 10_000 + rank as i64;
                hits.push(hit);
            }
            total += conn
                .query_row(
                    "SELECT COUNT(*) FROM dir_fts f JOIN dir_index d ON d.path = f.path
                     WHERE dir_fts MATCH ?1
                       AND (?2 IS NULL OR d.section = ?2)
                       AND (?3 IS NULL OR d.year = ?3)",
                    params![fts, filter.section, filter.year],
                    |r| r.get::<_, i64>(0),
                )
                .unwrap_or(0);
        }
        drop(conn);

        hits.sort_by_key(|hit| hit.score);
        let start = offset.max(0) as usize;
        let end = (start + limit.max(1) as usize).min(hits.len());
        let page = if start < hits.len() {
            hits[start..end].to_vec()
        } else {
            Vec::new()
        };

        if offset == 0 {
            if let Ok(conn) = lock(&self.app) {
                let _ = conn.execute(
                    "INSERT INTO searches(query, hits, at) VALUES(?1, ?2, ?3)",
                    params![query, total, db::now_iso()],
                );
            }
        }
        Ok(SearchPage {
            hits: page,
            total,
            took_ms: started.elapsed().as_millis() as i64,
        })
    }

    pub fn recent_searches(&self) -> Result<Vec<String>> {
        let conn = lock(&self.app)?;
        let mut stmt = conn
            .prepare("SELECT query FROM searches GROUP BY query ORDER BY MAX(id) DESC LIMIT 12")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    /// Secciones raíz (`Peliculas`, `Series`…) con su número de carpetas.
    pub fn sections(&self) -> Result<Vec<SectionInfo>> {
        let conn = lock(&self.cache)?;
        let mut stmt = conn.prepare(
            "SELECT section, COUNT(*) FROM dir_index WHERE section <> ''
             GROUP BY section ORDER BY section COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([], |r| {
            let name: String = r.get(0)?;
            Ok(SectionInfo {
                path: format!("/{name}/"),
                name,
                dirs: r.get(1)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    /// Carpetas vistas más recientemente (estantería «Recientes»).
    pub fn recent_items(&self, limit: i64) -> Result<Vec<Hit>> {
        let conn = lock(&self.cache)?;
        let mut stmt = conn.prepare(
            "SELECT path, name, parent, section, year FROM dir_index
             WHERE source <> 'bootstrap' ORDER BY seen_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit], |row| {
            Ok(Hit {
                kind: Kind::Dir,
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
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn cache_clear(&self, keep_index: bool) -> Result<IndexStats> {
        {
            let conn = lock(&self.cache)?;
            conn.execute("DELETE FROM dir_cache", [])?;
            if !keep_index {
                for table in ["dir_fts", "file_fts", "dir_index", "file_index"] {
                    conn.execute(&format!("DELETE FROM {table}"), [])?;
                }
                let _ = conn.execute("DELETE FROM meta WHERE key LIKE 'last_%'", []);
            }
        }
        self.index_stats()
    }

    /// Solo-caché: devuelve el listado si está guardado y fresco.
    pub fn browse_cached(&self, path: &str) -> Option<Listing> {
        let target = canonical_dir(path);
        let ttl = self.settings().ok()?.cache_ttl_hours as i64;
        let (payload, fetched_at) = lock(&self.cache)
            .ok()?
            .query_row(
                "SELECT payload, fetched_at FROM dir_cache WHERE path = ?1",
                [&target],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .ok()?;
        if !is_fresh(&fetched_at, ttl) {
            return None;
        }
        let entries = serde_json::from_str::<Vec<Entry>>(&payload).ok()?;
        Some(Listing {
            path: target,
            entries,
            fetched_at,
            cached: true,
        })
    }

    /// Lista una carpeta (caché si es fresca y `!force`). Emite exactamente un `Event::Listing`.
    pub fn browse(self: &Arc<Self>, path: &str, force: bool) {
        let core = self.clone();
        let target = canonical_dir(path);
        self.rt.spawn(async move {
            let result = if let (false, Some(cached)) = (force, core.browse_cached(&target)) {
                Ok(cached)
            } else {
                core.browse_live(&target).await
            };
            core.emit(Event::Listing {
                path: target,
                result: result.map_err(UiError::from),
            });
        });
    }

    pub(crate) async fn browse_live(&self, target: &str) -> Result<Listing> {
        let raw = self.net().list_dir(target).await?;
        let now = db::now_iso();
        let section = db::section_of(target);
        let year = norm::extract_year(&catalog::decode_name(target));

        let entries: Vec<Entry> = raw
            .iter()
            .map(|item| Entry {
                name: item.name.clone(),
                path: format!("{}{}", target, item.href),
                kind: if item.is_dir { Kind::Dir } else { Kind::File },
                ext: if item.is_dir {
                    None
                } else {
                    extension_of(&item.name)
                },
                size: if item.is_dir { None } else { item.size },
                mtime: item.mtime.clone(),
            })
            .collect();

        let max_bytes = self.settings()?.cache_max_mb as i64 * 1024 * 1024;
        {
            let mut conn = lock(&self.cache)?;
            let tx = conn.transaction()?;
            for entry in &entries {
                match entry.kind {
                    Kind::Dir => db::upsert_dir(
                        &tx,
                        &entry.path,
                        &entry.name,
                        target,
                        &section,
                        norm::extract_year(&entry.name).or(year),
                        "browse",
                    )?,
                    Kind::File => db::upsert_file(
                        &tx,
                        &entry.path,
                        &entry.name,
                        target,
                        &section,
                        year,
                        entry.ext.as_deref(),
                        entry.size,
                        entry.mtime.as_deref(),
                    )?,
                }
            }
            let payload = serde_json::to_string(&entries)?;
            tx.execute(
                "INSERT INTO dir_cache(path, payload, bytes, fetched_at) VALUES(?1, ?2, ?3, ?4)
                 ON CONFLICT(path) DO UPDATE SET payload = excluded.payload,
                    bytes = excluded.bytes, fetched_at = excluded.fetched_at",
                params![target, payload, payload.len() as i64, now],
            )?;
            prune_cache(&tx, max_bytes)?;
            tx.commit()?;
        }
        Ok(Listing {
            path: target.to_string(),
            entries,
            fetched_at: now,
            cached: false,
        })
    }

    pub fn crawl_stop(&self) {
        self.crawl_stop.store(true, Ordering::Relaxed);
    }

    /// Rastreo profundo (BFS, 6 listados simultáneos). Emite `CrawlProgress`.
    pub fn crawl_start(self: &Arc<Self>, max_dirs: usize) {
        let core = self.clone();
        self.rt.spawn(async move {
            if let Err(err) = core.crawl_inner(max_dirs).await {
                core.notice(Level::Error, format!("El rastreo se detuvo: {err}"));
            }
            if let Ok(stats) = core.index_stats() {
                core.emit(Event::IndexReady(stats));
            }
        });
    }

    async fn crawl_inner(self: &Arc<Self>, max_dirs: usize) -> Result<()> {
        self.crawl_stop.store(false, Ordering::Relaxed);
        let mut queue: VecDeque<String> = VecDeque::from(["/".to_string()]);
        let mut visited: HashSet<String> = HashSet::new();
        let (mut processed, mut file_count, mut errors) = (0_usize, 0_usize, 0_usize);

        while !queue.is_empty() && processed < max_dirs && !self.crawl_stop.load(Ordering::Relaxed)
        {
            let mut batch: Vec<String> = Vec::new();
            while batch.len() < 6 {
                let Some(path) = queue.pop_front() else { break };
                if visited.insert(path.clone()) {
                    batch.push(path);
                }
            }
            if batch.is_empty() {
                break;
            }
            let net = self.net();
            let results = futures::future::join_all(batch.iter().map(|p| net.list_dir(p))).await;

            {
                let mut conn = lock(&self.cache)?;
                let tx = conn.transaction()?;
                for (path, result) in batch.iter().zip(results) {
                    let Ok(entries) = result else {
                        errors += 1;
                        continue;
                    };
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
                db::set_meta(&tx, "last_crawl", &db::now_iso());
                tx.commit()?;
            }
            processed += batch.len();
            self.emit(Event::CrawlProgress {
                dirs: processed,
                queued: queue.len(),
                files: file_count,
                errors,
                max: max_dirs,
            });
        }
        Ok(())
    }
}

/// Recorre las carpetas hasta `depth` niveles (alternativa a `listado.html`).
async fn shallow_tree(net: &crate::Net, depth: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut level = vec!["/".to_string()];
    for _ in 0..depth {
        let mut next = Vec::new();
        for path in &level {
            if let Ok(entries) = net.list_dir(path).await {
                for entry in entries.into_iter().filter(|e| e.is_dir) {
                    let child = format!("{}{}", path, entry.href);
                    out.push(child.clone());
                    next.push(child);
                }
            }
        }
        level = next;
    }
    out
}
