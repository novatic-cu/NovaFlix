use std::path::Path;

use rusqlite::Connection;

use crate::norm::normalize;

pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub fn open_cache(path: &Path) -> Result<Connection, String> {
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    conn.pragma_update(None, "journal_mode", "WAL").ok();
    conn.pragma_update(None, "synchronous", "NORMAL").ok();
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS meta(
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS dir_index(
            path    TEXT PRIMARY KEY,
            name    TEXT NOT NULL,
            norm    TEXT NOT NULL,
            parent  TEXT NOT NULL,
            section TEXT NOT NULL,
            year    INTEGER,
            source  TEXT NOT NULL,
            seen_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_dir_section ON dir_index(section, year);
        CREATE VIRTUAL TABLE IF NOT EXISTS dir_fts USING fts5(
            name, norm, path UNINDEXED,
            tokenize = 'unicode61 remove_diacritics 2'
        );
        CREATE TABLE IF NOT EXISTS file_index(
            path    TEXT PRIMARY KEY,
            name    TEXT NOT NULL,
            norm    TEXT NOT NULL,
            parent  TEXT NOT NULL,
            section TEXT NOT NULL,
            year    INTEGER,
            ext     TEXT,
            size    INTEGER,
            mtime   TEXT,
            seen_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_file_ext ON file_index(ext);
        CREATE VIRTUAL TABLE IF NOT EXISTS file_fts USING fts5(
            name, norm, path UNINDEXED,
            tokenize = 'unicode61 remove_diacritics 2'
        );
        CREATE TABLE IF NOT EXISTS dir_cache(
            path       TEXT PRIMARY KEY,
            payload    TEXT NOT NULL,
            bytes      INTEGER NOT NULL,
            fetched_at TEXT NOT NULL
        );
        "#,
    )
    .map_err(|e| e.to_string())?;
    Ok(conn)
}

pub fn open_app(path: &Path) -> Result<Connection, String> {
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    conn.pragma_update(None, "journal_mode", "WAL").ok();
    conn.pragma_update(None, "foreign_keys", "ON").ok();
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS items(
            path       TEXT PRIMARY KEY,
            name       TEXT NOT NULL,
            kind       TEXT NOT NULL,
            ext        TEXT,
            section    TEXT,
            year       INTEGER,
            size       INTEGER,
            first_seen TEXT NOT NULL,
            last_seen  TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS favorites(
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            target_kind TEXT NOT NULL CHECK(target_kind IN ('item','folder')),
            path        TEXT NOT NULL,
            label       TEXT,
            note        TEXT,
            created_at  TEXT NOT NULL,
            UNIQUE(target_kind, path)
        );
        CREATE TABLE IF NOT EXISTS lists(
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            name       TEXT NOT NULL,
            kind       TEXT NOT NULL CHECK(kind IN ('playlist','watchlist')),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS list_items(
            list_id  INTEGER NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
            path     TEXT NOT NULL,
            name     TEXT NOT NULL,
            position INTEGER NOT NULL,
            added_at TEXT NOT NULL,
            watched_at TEXT,
            PRIMARY KEY(list_id, path)
        );
        CREATE TABLE IF NOT EXISTS downloads(
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            path        TEXT NOT NULL,
            dest        TEXT NOT NULL,
            bytes_total INTEGER DEFAULT 0,
            bytes_done  INTEGER DEFAULT 0,
            state       TEXT NOT NULL,
            started_at  TEXT NOT NULL,
            finished_at TEXT,
            error       TEXT
        );
        CREATE TABLE IF NOT EXISTS counters(
            path      TEXT PRIMARY KEY,
            attempts  INTEGER NOT NULL DEFAULT 0,
            completed INTEGER NOT NULL DEFAULT 0,
            last_at   TEXT
        );
        CREATE TABLE IF NOT EXISTS searches(
            id    INTEGER PRIMARY KEY AUTOINCREMENT,
            query TEXT NOT NULL,
            hits  INTEGER NOT NULL,
            at    TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS settings(
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        "#,
    )
    .map_err(|e| e.to_string())?;
    Ok(conn)
}

pub fn get_meta(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
        row.get(0)
    })
    .ok()
}

pub fn set_meta(conn: &Connection, key: &str, value: &str) {
    let _ = conn.execute(
        "INSERT INTO meta(key, value) VALUES(?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![key, value],
    );
}

pub fn get_setting(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
        row.get(0)
    })
    .ok()
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO settings(key, value) VALUES(?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![key, value],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

/// Inserta/actualiza un directorio y su espejo FTS.
pub fn upsert_dir(
    tx: &rusqlite::Transaction<'_>,
    path: &str,
    name: &str,
    parent: &str,
    section: &str,
    year: Option<i64>,
    source: &str,
) -> Result<(), String> {
    upsert_dir_row(tx, path, name, parent, section, year, source)?;
    tx.execute("DELETE FROM dir_fts WHERE path = ?1", [path])
        .map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO dir_fts(rowid, name, norm, path)
         SELECT rowid, name, norm, path FROM dir_index WHERE path = ?1",
        [path],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Inserta/actualiza el directorio sin tocar FTS (para cargas masivas).
pub fn upsert_dir_row(
    tx: &rusqlite::Transaction<'_>,
    path: &str,
    name: &str,
    parent: &str,
    section: &str,
    year: Option<i64>,
    source: &str,
) -> Result<(), String> {
    let norm = normalize(name);
    tx.execute(
        "INSERT INTO dir_index(path, name, norm, parent, section, year, source, seen_at)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(path) DO UPDATE SET
             name = excluded.name,
             norm = excluded.norm,
             year = excluded.year,
             seen_at = excluded.seen_at",
        rusqlite::params![path, name, norm, parent, section, year, source, now_iso()],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

/// Reconstruye el indice FTS de directorios en una sola pasada.
pub fn rebuild_dir_fts(tx: &rusqlite::Transaction<'_>) -> Result<(), String> {
    tx.execute("DELETE FROM dir_fts", [])
        .map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO dir_fts(rowid, name, norm, path) SELECT rowid, name, norm, path FROM dir_index",
        [],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

#[allow(clippy::too_many_arguments)]
pub fn upsert_file(
    tx: &rusqlite::Transaction<'_>,
    path: &str,
    name: &str,
    parent: &str,
    section: &str,
    year: Option<i64>,
    ext: Option<&str>,
    size: Option<i64>,
    mtime: Option<&str>,
) -> Result<(), String> {
    let norm = normalize(name);
    tx.execute(
        "INSERT INTO file_index(path, name, norm, parent, section, year, ext, size, mtime, seen_at)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(path) DO UPDATE SET
             name = excluded.name,
             norm = excluded.norm,
             ext = excluded.ext,
             size = excluded.size,
             mtime = excluded.mtime,
             seen_at = excluded.seen_at",
        rusqlite::params![
            path,
            name,
            norm,
            parent,
            section,
            year,
            ext,
            size,
            mtime,
            now_iso()
        ],
    )
    .map_err(|e| e.to_string())?;

    tx.execute("DELETE FROM file_fts WHERE path = ?1", [path])
        .map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO file_fts(rowid, name, norm, path)
         SELECT rowid, name, norm, path FROM file_index WHERE path = ?1",
        [path],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn section_of(path: &str) -> String {
    path.trim_start_matches('/')
        .split('/')
        .next()
        .unwrap_or("")
        .to_string()
}

pub fn parent_of(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    match trimmed.rfind('/') {
        Some(0) => "/".to_string(),
        Some(idx) => trimmed[..idx].to_string(),
        None => "/".to_string(),
    }
}

pub fn name_of(path: &str) -> String {
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("")
        .to_string()
}
