use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use futures::StreamExt;
use rusqlite::params;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

use crate::{catalog, db, library, state::AppState};

#[derive(Debug, Serialize)]
pub struct DownloadRow {
    pub id: i64,
    pub path: String,
    pub dest: String,
    pub bytes_total: i64,
    pub bytes_done: i64,
    pub state: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub error: Option<String>,
}

fn sanitize_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let trimmed = cleaned.trim().trim_start_matches('.').to_string();
    if trimmed.is_empty() {
        "archivo".to_string()
    } else {
        trimmed
    }
}

fn update_row(state: &AppState, id: i64, done: i64, total: i64, status: &str, error: Option<&str>) {
    if let Ok(conn) = state.app.lock() {
        let finished = if matches!(status, "done" | "error" | "cancelled") {
            Some(db::now_iso())
        } else {
            None
        };
        let _ = conn.execute(
            "UPDATE downloads SET bytes_done = ?2, bytes_total = ?3, state = ?4,
                error = ?5, finished_at = COALESCE(?6, finished_at)
             WHERE id = ?1",
            params![id, done, total, status, error, finished],
        );
    }
}

pub fn list(state: &AppState) -> Result<Vec<DownloadRow>, String> {
    let conn = state.app.lock().map_err(|_| "app bloqueada".to_string())?;
    let mut statement = conn
        .prepare(
            "SELECT id, path, dest, bytes_total, bytes_done, state, started_at, finished_at, error
             FROM downloads ORDER BY id DESC LIMIT 300",
        )
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(DownloadRow {
                id: row.get(0)?,
                path: row.get(1)?,
                dest: row.get(2)?,
                bytes_total: row.get(3)?,
                bytes_done: row.get(4)?,
                state: row.get(5)?,
                started_at: row.get(6)?,
                finished_at: row.get(7)?,
                error: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn clear_finished(state: &AppState) -> Result<(), String> {
    let conn = state.app.lock().map_err(|_| "app bloqueada".to_string())?;
    conn.execute(
        "DELETE FROM downloads WHERE state IN ('done', 'error', 'cancelled')",
        [],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub fn enqueue(
    app: AppHandle,
    state: Arc<AppState>,
    paths: Vec<String>,
    dest_dir: Option<String>,
) -> Result<Vec<i64>, String> {
    let target_dir = dest_dir
        .map(PathBuf::from)
        .or_else(|| {
            state
                .app
                .lock()
                .ok()
                .and_then(|conn| db::get_setting(&conn, "download_dir"))
                .map(PathBuf::from)
        })
        .unwrap_or_else(|| state.downloads_dir.clone());

    std::fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;

    let mut ids = Vec::new();
    for path in paths {
        let decoded = catalog::decode_name(&db::name_of(&path));
        let file_name = sanitize_file_name(&decoded);
        let dest = target_dir.join(file_name);

        let id = {
            let conn = state.app.lock().map_err(|_| "app bloqueada".to_string())?;
            conn.execute(
                "INSERT INTO downloads(path, dest, bytes_total, bytes_done, state, started_at)
                 VALUES(?1, ?2, 0, 0, 'queued', ?3)",
                params![path, dest.to_string_lossy().to_string(), db::now_iso()],
            )
            .map_err(|e| e.to_string())?;
            conn.last_insert_rowid()
        };

        library::bump_attempt(&state, &path);
        state
            .cancels
            .lock()
            .map_err(|_| "descargas bloqueadas".to_string())?
            .insert(id, Arc::new(AtomicBool::new(false)));

        ids.push(id);
        tauri::async_runtime::spawn(run(app.clone(), state.clone(), id, path, dest));
    }

    Ok(ids)
}

pub fn pause(state: &AppState, id: i64) -> Result<(), String> {
    if let Ok(map) = state.cancels.lock() {
        if let Some(flag) = map.get(&id) {
            flag.store(true, Ordering::Relaxed);
        }
    }
    Ok(())
}

pub fn cancel(app: &AppHandle, state: &AppState, id: i64) -> Result<(), String> {
    pause(state, id)?;
    update_row(state, id, 0, 0, "cancelled", None);
    let _ = app.emit(
        "download://progress",
        serde_json::json!({ "id": id, "state": "cancelled", "done": 0, "total": 0 }),
    );
    Ok(())
}

pub fn resume(app: AppHandle, state: Arc<AppState>, id: i64) -> Result<(), String> {
    let (path, dest) = {
        let conn = state.app.lock().map_err(|_| "app bloqueada".to_string())?;
        conn.query_row(
            "SELECT path, dest FROM downloads WHERE id = ?1",
            [id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .map_err(|e| e.to_string())?
    };

    state
        .cancels
        .lock()
        .map_err(|_| "descargas bloqueadas".to_string())?
        .insert(id, Arc::new(AtomicBool::new(false)));
    update_row(&state, id, 0, 0, "queued", None);
    tauri::async_runtime::spawn(run(app, state, id, path, PathBuf::from(dest)));
    Ok(())
}

async fn run(app: AppHandle, state: Arc<AppState>, id: i64, remote_path: String, dest: PathBuf) {
    let _permit = match state.dl_sem.clone().acquire_owned().await {
        Ok(permit) => permit,
        Err(_) => return,
    };

    let cancel = state
        .cancels
        .lock()
        .ok()
        .and_then(|map| map.get(&id).cloned())
        .unwrap_or_else(|| Arc::new(AtomicBool::new(false)));

    if cancel.load(Ordering::Relaxed) {
        update_row(&state, id, 0, 0, "paused", None);
        return;
    }

    let url = catalog::absolute_url(&remote_path);
    let part = PathBuf::from(format!("{}.part", dest.to_string_lossy()));

    update_row(&state, id, 0, 0, "active", None);

    let mut start = tokio::fs::metadata(&part)
        .await
        .map(|meta| meta.len())
        .unwrap_or(0);
    if start > 0 {
        if let Ok(existing) = tokio::fs::metadata(&dest).await {
            if existing.len() == start {
                start = 0;
                let _ = tokio::fs::remove_file(&part).await;
            }
        }
    }

    let mut request = state.http.get(&url);
    if start > 0 {
        request = request.header("Range", format!("bytes={}-", start));
    }

    let response = match request.send().await {
        Ok(response) => response,
        Err(err) => {
            update_row(&state, id, start as i64, 0, "error", Some(&err.to_string()));
            let _ = app.emit(
                "download://progress",
                serde_json::json!({ "id": id, "state": "error", "done": start, "total": 0, "error": err.to_string() }),
            );
            return;
        }
    };

    let status = response.status();
    if !status.is_success() && status != reqwest::StatusCode::PARTIAL_CONTENT {
        let message = format!("HTTP {}", status);
        update_row(&state, id, 0, 0, "error", Some(&message));
        let _ = app.emit(
            "download://progress",
            serde_json::json!({ "id": id, "state": "error", "done": 0, "total": 0, "error": message }),
        );
        return;
    }

    if status == reqwest::StatusCode::OK && start > 0 {
        start = 0;
        let _ = tokio::fs::remove_file(&part).await;
    }

    let total = response
        .content_length()
        .map(|length| length as i64 + start as i64)
        .unwrap_or(0);

    let mut file = match tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&part)
        .await
    {
        Ok(file) => file,
        Err(err) => {
            update_row(
                &state,
                id,
                start as i64,
                total,
                "error",
                Some(&err.to_string()),
            );
            return;
        }
    };

    let mut stream = response.bytes_stream();
    let mut done = start;
    let mut last_emit = Instant::now();

    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::Relaxed) {
            let _ = file.flush().await;
            update_row(&state, id, done as i64, total, "paused", None);
            let _ = app.emit(
                "download://progress",
                serde_json::json!({ "id": id, "state": "paused", "done": done, "total": total }),
            );
            return;
        }

        match chunk {
            Ok(bytes) => {
                if let Err(err) = tokio::io::AsyncWriteExt::write_all(&mut file, &bytes).await {
                    update_row(
                        &state,
                        id,
                        done as i64,
                        total,
                        "error",
                        Some(&err.to_string()),
                    );
                    return;
                }
                done += bytes.len() as u64;
            }
            Err(err) => {
                update_row(
                    &state,
                    id,
                    done as i64,
                    total,
                    "error",
                    Some(&err.to_string()),
                );
                let _ = app.emit(
                    "download://progress",
                    serde_json::json!({ "id": id, "state": "error", "done": done, "total": total, "error": err.to_string() }),
                );
                return;
            }
        }

        if last_emit.elapsed() > Duration::from_millis(180) {
            last_emit = Instant::now();
            update_row(&state, id, done as i64, total, "active", None);
            let _ = app.emit(
                "download://progress",
                serde_json::json!({ "id": id, "state": "active", "done": done, "total": total }),
            );
        }
    }

    let _ = file.flush().await;
    drop(file);

    if let Err(err) = tokio::fs::rename(&part, &dest).await {
        update_row(
            &state,
            id,
            done as i64,
            total,
            "error",
            Some(&err.to_string()),
        );
        return;
    }

    update_row(&state, id, done as i64, total, "done", None);
    library::bump_completed(&state, &remote_path);
    let _ = app.emit(
        "download://progress",
        serde_json::json!({
            "id": id,
            "state": "done",
            "done": done,
            "total": if total > 0 { total } else { done as i64 },
            "dest": dest.to_string_lossy().to_string(),
        }),
    );
}
