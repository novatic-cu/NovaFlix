use std::{path::PathBuf, process::Command};

use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::{db, state::AppState, visuales};

#[derive(Debug, Serialize)]
pub struct Favorite {
    pub id: i64,
    pub target_kind: String,
    pub path: String,
    pub label: String,
    pub note: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ListSummary {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct ListItem {
    pub path: String,
    pub name: String,
    pub position: i64,
    pub added_at: String,
    pub watched_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Counter {
    pub path: String,
    pub attempts: i64,
    pub completed: i64,
    pub last_at: Option<String>,
}

pub fn remember_item(
    state: &AppState,
    path: &str,
    name: &str,
    kind: &str,
    ext: Option<&str>,
    size: Option<i64>,
) -> Result<(), String> {
    let conn = state.app.lock().map_err(|_| "app bloqueada".to_string())?;
    let section = db::section_of(path);
    let year = crate::norm::extract_year(name);
    let now = db::now_iso();
    conn.execute(
        "INSERT INTO items(path, name, kind, ext, section, year, size, first_seen, last_seen)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
         ON CONFLICT(path) DO UPDATE SET
            name = excluded.name,
            last_seen = excluded.last_seen,
            size = COALESCE(excluded.size, items.size)",
        params![path, name, kind, ext, section, year, size, now],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub fn favorites(state: &AppState) -> Result<Vec<Favorite>, String> {
    let conn = state.app.lock().map_err(|_| "app bloqueada".to_string())?;
    let mut statement = conn
        .prepare(
            "SELECT id, target_kind, path, COALESCE(label, ''), note, created_at
             FROM favorites ORDER BY created_at DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(Favorite {
                id: row.get(0)?,
                target_kind: row.get(1)?,
                path: row.get(2)?,
                label: row.get(3)?,
                note: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn favorite_add(
    state: &AppState,
    target_kind: &str,
    path: &str,
    label: &str,
    note: Option<String>,
    size: Option<i64>,
) -> Result<(), String> {
    let decoded = visuales::decode_name(&db::name_of(path));
    let name = if label.trim().is_empty() { decoded } else { label.to_string() };
    remember_item(state, path, &name, target_kind, None, size)?;

    let conn = state.app.lock().map_err(|_| "app bloqueada".to_string())?;
    conn.execute(
        "INSERT INTO favorites(target_kind, path, label, note, created_at)
         VALUES(?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(target_kind, path) DO UPDATE SET
            label = excluded.label,
            note = COALESCE(excluded.note, favorites.note)",
        params![target_kind, path, name, note, db::now_iso()],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub fn favorite_remove(state: &AppState, target_kind: &str, path: &str) -> Result<(), String> {
    let conn = state.app.lock().map_err(|_| "app bloqueada".to_string())?;
    conn.execute(
        "DELETE FROM favorites WHERE target_kind = ?1 AND path = ?2",
        params![target_kind, path],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub fn lists(state: &AppState) -> Result<Vec<ListSummary>, String> {
    let conn = state.app.lock().map_err(|_| "app bloqueada".to_string())?;
    let mut statement = conn
        .prepare(
            "SELECT l.id, l.name, l.kind,
                    (SELECT COUNT(*) FROM list_items WHERE list_id = l.id),
                    l.created_at, l.updated_at
             FROM lists l ORDER BY l.updated_at DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(ListSummary {
                id: row.get(0)?,
                name: row.get(1)?,
                kind: row.get(2)?,
                count: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn list_create(state: &AppState, name: &str, kind: &str) -> Result<i64, String> {
    let conn = state.app.lock().map_err(|_| "app bloqueada".to_string())?;
    let now = db::now_iso();
    conn.execute(
        "INSERT INTO lists(name, kind, created_at, updated_at) VALUES(?1, ?2, ?3, ?3)",
        params![name, kind, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

pub fn list_delete(state: &AppState, list_id: i64) -> Result<(), String> {
    let conn = state.app.lock().map_err(|_| "app bloqueada".to_string())?;
    conn.execute("DELETE FROM lists WHERE id = ?1", [list_id])
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn list_rename(state: &AppState, list_id: i64, name: &str) -> Result<(), String> {
    let conn = state.app.lock().map_err(|_| "app bloqueada".to_string())?;
    conn.execute(
        "UPDATE lists SET name = ?2, updated_at = ?3 WHERE id = ?1",
        params![list_id, name, db::now_iso()],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub fn list_items(state: &AppState, list_id: i64) -> Result<Vec<ListItem>, String> {
    let conn = state.app.lock().map_err(|_| "app bloqueada".to_string())?;
    let mut statement = conn
        .prepare(
            "SELECT path, name, position, added_at, watched_at
             FROM list_items WHERE list_id = ?1 ORDER BY position ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([list_id], |row| {
            Ok(ListItem {
                path: row.get(0)?,
                name: row.get(1)?,
                position: row.get(2)?,
                added_at: row.get(3)?,
                watched_at: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn list_add(state: &AppState, list_id: i64, path: &str, name: Option<String>) -> Result<(), String> {
    let decoded = visuales::decode_name(&db::name_of(path));
    let label = name.unwrap_or(decoded);
    let conn = state.app.lock().map_err(|_| "app bloqueada".to_string())?;
    let next: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM list_items WHERE list_id = ?1",
            [list_id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    conn.execute(
        "INSERT INTO list_items(list_id, path, name, position, added_at)
         VALUES(?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(list_id, path) DO NOTHING",
        params![list_id, path, label, next, db::now_iso()],
    )
    .map_err(|e| e.to_string())?;
    conn.execute("UPDATE lists SET updated_at = ?2 WHERE id = ?1", params![list_id, db::now_iso()])
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn list_remove(state: &AppState, list_id: i64, path: &str) -> Result<(), String> {
    let conn = state.app.lock().map_err(|_| "app bloqueada".to_string())?;
    conn.execute(
        "DELETE FROM list_items WHERE list_id = ?1 AND path = ?2",
        params![list_id, path],
    )
    .map_err(|e| e.to_string())?;
    conn.execute("UPDATE lists SET updated_at = ?2 WHERE id = ?1", params![list_id, db::now_iso()])
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn list_reorder(state: &AppState, list_id: i64, paths: Vec<String>) -> Result<(), String> {
    let mut conn = state.app.lock().map_err(|_| "app bloqueada".to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for (index, path) in paths.iter().enumerate() {
        tx.execute(
            "UPDATE list_items SET position = ?3 WHERE list_id = ?1 AND path = ?2",
            params![list_id, path, index as i64],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.execute("UPDATE lists SET updated_at = ?2 WHERE id = ?1", params![list_id, db::now_iso()])
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}

pub fn list_mark_watched(state: &AppState, list_id: i64, path: &str, watched: bool) -> Result<(), String> {
    let conn = state.app.lock().map_err(|_| "app bloqueada".to_string())?;
    let value: Option<String> = if watched { Some(db::now_iso()) } else { None };
    conn.execute(
        "UPDATE list_items SET watched_at = ?3 WHERE list_id = ?1 AND path = ?2",
        params![list_id, path, value],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub fn counters(state: &AppState) -> Result<Vec<Counter>, String> {
    let conn = state.app.lock().map_err(|_| "app bloqueada".to_string())?;
    let mut statement = conn
        .prepare(
            "SELECT path, attempts, completed, last_at FROM counters
             ORDER BY attempts DESC LIMIT 500",
        )
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(Counter {
                path: row.get(0)?,
                attempts: row.get(1)?,
                completed: row.get(2)?,
                last_at: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn counter_for(state: &AppState, path: &str) -> Result<Counter, String> {
    let conn = state.app.lock().map_err(|_| "app bloqueada".to_string())?;
    conn.query_row(
        "SELECT path, attempts, completed, last_at FROM counters WHERE path = ?1",
        [path],
        |row| {
            Ok(Counter {
                path: row.get(0)?,
                attempts: row.get(1)?,
                completed: row.get(2)?,
                last_at: row.get(3)?,
            })
        },
    )
    .optional()
    .map_err(|e| e.to_string())
    .map(|value| {
        value.unwrap_or(Counter {
            path: path.to_string(),
            attempts: 0,
            completed: 0,
            last_at: None,
        })
    })
}

pub fn bump_attempt(state: &AppState, path: &str) {
    if let Ok(conn) = state.app.lock() {
        let _ = conn.execute(
            "INSERT INTO counters(path, attempts, completed, last_at) VALUES(?1, 1, 0, ?2)
             ON CONFLICT(path) DO UPDATE SET attempts = attempts + 1, last_at = excluded.last_at",
            params![path, db::now_iso()],
        );
    }
}

pub fn bump_completed(state: &AppState, path: &str) {
    if let Ok(conn) = state.app.lock() {
        let _ = conn.execute(
            "INSERT INTO counters(path, attempts, completed, last_at) VALUES(?1, 0, 1, ?2)
             ON CONFLICT(path) DO UPDATE SET completed = completed + 1, last_at = excluded.last_at",
            params![path, db::now_iso()],
        );
    }
}

pub fn settings(state: &AppState) -> Result<serde_json::Value, String> {
    let conn = state.app.lock().map_err(|_| "app bloqueada".to_string())?;
    let mut statement = conn
        .prepare("SELECT key, value FROM settings")
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?;

    let mut map = serde_json::Map::new();
    for row in rows {
        let (key, value) = row.map_err(|e| e.to_string())?;
        map.insert(key, serde_json::Value::String(value));
    }
    Ok(serde_json::Value::Object(map))
}

pub fn setting_set(state: &AppState, key: &str, value: &str) -> Result<(), String> {
    let conn = state.app.lock().map_err(|_| "app bloqueada".to_string())?;
    db::set_setting(&conn, key, value)
}

pub fn recent_searches(state: &AppState) -> Result<Vec<String>, String> {
    let conn = state.app.lock().map_err(|_| "app bloqueada".to_string())?;
    let mut statement = conn
        .prepare("SELECT query FROM searches ORDER BY id DESC LIMIT 12")
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Genera un archivo M3U con URLs absolutas para abrir en VLC/mpv.
pub fn export_m3u(app: &AppHandle, state: &AppState, list_id: i64) -> Result<PathBuf, String> {
    let (name, items) = {
        let conn = state.app.lock().map_err(|_| "app bloqueada".to_string())?;
        let name: String = conn
            .query_row("SELECT name FROM lists WHERE id = ?1", [list_id], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        let mut statement = conn
            .prepare("SELECT name, path FROM list_items WHERE list_id = ?1 ORDER BY position")
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([list_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        (name, rows)
    };

    let mut body = String::from("#EXTM3U\n");
    for (item_name, path) in items {
        body.push_str(&format!(
            "#EXTINF:-1,{}\n{}\n",
            item_name,
            visuales::absolute_url(&path)
        ));
    }

    let exports = state.data_dir.join("exports");
    std::fs::create_dir_all(&exports).map_err(|e| e.to_string())?;
    let slug: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let file = exports.join(format!("{}-{}.m3u8", slug.trim_matches('-'), stamp));
    std::fs::write(&file, body).map_err(|e| e.to_string())?;
    let _ = app.emit("export://done", file.to_string_lossy().to_string());
    Ok(file)
}

/// Lanza el reproductor externo configurado (VLC/mpv/celluloid) con la URL remota.
pub fn play(state: &AppState, path: &str) -> Result<(), String> {
    let url = visuales::absolute_url(path);
    let configured = state
        .app
        .lock()
        .ok()
        .and_then(|conn| db::get_setting(&conn, "player"));

    let mut candidates: Vec<String> = Vec::new();
    if let Some(player) = configured.filter(|value| !value.trim().is_empty() && value != "auto") {
        candidates.push(player);
    }
    for fallback in ["vlc", "mpv", "celluloid", "totem"] {
        candidates.push(fallback.to_string());
    }
    #[cfg(windows)]
    {
        candidates.push(r"C:\Program Files\VideoLAN\VLC\vlc.exe".to_string());
        candidates.push(r"C:\Program Files (x86)\VideoLAN\VLC\vlc.exe".to_string());
    }

    for candidate in candidates {
        if Command::new(&candidate).arg(&url).spawn().is_ok() {
            return Ok(());
        }
    }
    Err("no-player".to_string())
}
