//! Biblioteca personal: favoritos, listas, contadores, M3U y reproductor externo.

use std::{path::PathBuf, process::Command};

use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use crate::{
    catalog, db,
    error::{lock, Error, Result},
    Core,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TargetKind {
    Item,
    Folder,
}

impl TargetKind {
    fn as_str(self) -> &'static str {
        match self {
            TargetKind::Item => "item",
            TargetKind::Folder => "folder",
        }
    }
    fn parse(value: &str) -> Self {
        if value == "folder" {
            TargetKind::Folder
        } else {
            TargetKind::Item
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ListKind {
    Playlist,
    Watchlist,
}

impl ListKind {
    fn as_str(self) -> &'static str {
        match self {
            ListKind::Playlist => "playlist",
            ListKind::Watchlist => "watchlist",
        }
    }
    fn parse(value: &str) -> Self {
        if value == "watchlist" {
            ListKind::Watchlist
        } else {
            ListKind::Playlist
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Favorite {
    pub id: i64,
    pub kind: TargetKind,
    pub path: String,
    pub label: String,
    pub note: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListSummary {
    pub id: i64,
    pub name: String,
    pub kind: ListKind,
    pub count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListItem {
    pub path: String,
    pub name: String,
    pub position: i64,
    pub added_at: String,
    pub watched_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Counter {
    pub path: String,
    pub attempts: i64,
    pub completed: i64,
    pub last_at: Option<String>,
}

impl Core {
    pub(crate) fn remember_item(
        &self,
        path: &str,
        name: &str,
        kind: TargetKind,
        size: Option<i64>,
    ) -> Result<()> {
        let conn = lock(&self.app)?;
        let now = db::now_iso();
        conn.execute(
            "INSERT INTO items(path, name, kind, ext, section, year, size, first_seen, last_seen)
             VALUES(?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7, ?7)
             ON CONFLICT(path) DO UPDATE SET
                name = excluded.name, last_seen = excluded.last_seen,
                size = COALESCE(excluded.size, items.size)",
            params![
                path,
                name,
                kind.as_str(),
                db::section_of(path),
                crate::norm::extract_year(name),
                size,
                now
            ],
        )?;
        Ok(())
    }

    // ---- Favoritos -----------------------------------------------------------------

    pub fn favorites(&self) -> Result<Vec<Favorite>> {
        let conn = lock(&self.app)?;
        let mut stmt = conn.prepare(
            "SELECT id, target_kind, path, COALESCE(label, ''), note, created_at
             FROM favorites ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Favorite {
                id: row.get(0)?,
                kind: TargetKind::parse(&row.get::<_, String>(1)?),
                path: row.get(2)?,
                label: row.get(3)?,
                note: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn favorite_add(
        &self,
        kind: TargetKind,
        path: &str,
        label: &str,
        note: Option<String>,
        size: Option<i64>,
    ) -> Result<()> {
        let name = if label.trim().is_empty() {
            catalog::decode_name(&db::name_of(path))
        } else {
            label.to_string()
        };
        self.remember_item(path, &name, kind, size)?;
        lock(&self.app)?.execute(
            "INSERT INTO favorites(target_kind, path, label, note, created_at)
             VALUES(?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(target_kind, path) DO UPDATE SET
                label = excluded.label, note = COALESCE(excluded.note, favorites.note)",
            params![kind.as_str(), path, name, note, db::now_iso()],
        )?;
        Ok(())
    }

    pub fn favorite_remove(&self, kind: TargetKind, path: &str) -> Result<()> {
        lock(&self.app)?.execute(
            "DELETE FROM favorites WHERE target_kind = ?1 AND path = ?2",
            params![kind.as_str(), path],
        )?;
        Ok(())
    }

    pub fn is_favorite(&self, kind: TargetKind, path: &str) -> bool {
        lock(&self.app)
            .ok()
            .and_then(|c| {
                c.query_row(
                    "SELECT 1 FROM favorites WHERE target_kind = ?1 AND path = ?2",
                    params![kind.as_str(), path],
                    |_| Ok(()),
                )
                .ok()
            })
            .is_some()
    }

    // ---- Listas ----------------------------------------------------------------------

    pub fn lists(&self) -> Result<Vec<ListSummary>> {
        let conn = lock(&self.app)?;
        let mut stmt = conn.prepare(
            "SELECT l.id, l.name, l.kind, (SELECT COUNT(*) FROM list_items WHERE list_id = l.id),
                    l.created_at, l.updated_at
             FROM lists l ORDER BY l.updated_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ListSummary {
                id: row.get(0)?,
                name: row.get(1)?,
                kind: ListKind::parse(&row.get::<_, String>(2)?),
                count: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn list_create(&self, name: &str, kind: ListKind) -> Result<i64> {
        if name.trim().is_empty() {
            return Err(Error::msg("La lista necesita un nombre."));
        }
        let conn = lock(&self.app)?;
        conn.execute(
            "INSERT INTO lists(name, kind, created_at, updated_at) VALUES(?1, ?2, ?3, ?3)",
            params![name.trim(), kind.as_str(), db::now_iso()],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_rename(&self, id: i64, name: &str) -> Result<()> {
        lock(&self.app)?.execute(
            "UPDATE lists SET name = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, name.trim(), db::now_iso()],
        )?;
        Ok(())
    }

    pub fn list_delete(&self, id: i64) -> Result<()> {
        lock(&self.app)?.execute("DELETE FROM lists WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn list_items(&self, id: i64) -> Result<Vec<ListItem>> {
        let conn = lock(&self.app)?;
        let mut stmt = conn.prepare(
            "SELECT path, name, position, added_at, watched_at
             FROM list_items WHERE list_id = ?1 ORDER BY position ASC",
        )?;
        let rows = stmt.query_map([id], |row| {
            Ok(ListItem {
                path: row.get(0)?,
                name: row.get(1)?,
                position: row.get(2)?,
                added_at: row.get(3)?,
                watched_at: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn list_add(&self, id: i64, path: &str, name: Option<String>) -> Result<()> {
        let label = name.unwrap_or_else(|| catalog::decode_name(&db::name_of(path)));
        let conn = lock(&self.app)?;
        let next: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(position), -1) + 1 FROM list_items WHERE list_id = ?1",
                [id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        conn.execute(
            "INSERT INTO list_items(list_id, path, name, position, added_at)
             VALUES(?1, ?2, ?3, ?4, ?5) ON CONFLICT(list_id, path) DO NOTHING",
            params![id, path, label, next, db::now_iso()],
        )?;
        conn.execute(
            "UPDATE lists SET updated_at = ?2 WHERE id = ?1",
            params![id, db::now_iso()],
        )?;
        Ok(())
    }

    pub fn list_remove(&self, id: i64, path: &str) -> Result<()> {
        let conn = lock(&self.app)?;
        conn.execute(
            "DELETE FROM list_items WHERE list_id = ?1 AND path = ?2",
            params![id, path],
        )?;
        conn.execute(
            "UPDATE lists SET updated_at = ?2 WHERE id = ?1",
            params![id, db::now_iso()],
        )?;
        Ok(())
    }

    pub fn list_reorder(&self, id: i64, paths: Vec<String>) -> Result<()> {
        let mut conn = lock(&self.app)?;
        let tx = conn.transaction()?;
        for (index, path) in paths.iter().enumerate() {
            tx.execute(
                "UPDATE list_items SET position = ?3 WHERE list_id = ?1 AND path = ?2",
                params![id, path, index as i64],
            )?;
        }
        tx.execute(
            "UPDATE lists SET updated_at = ?2 WHERE id = ?1",
            params![id, db::now_iso()],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn list_mark_watched(&self, id: i64, path: &str, watched: bool) -> Result<()> {
        let value: Option<String> = watched.then(db::now_iso);
        lock(&self.app)?.execute(
            "UPDATE list_items SET watched_at = ?3 WHERE list_id = ?1 AND path = ?2",
            params![id, path, value],
        )?;
        Ok(())
    }

    // ---- Contadores ------------------------------------------------------------------

    pub fn counters(&self) -> Result<Vec<Counter>> {
        let conn = lock(&self.app)?;
        let mut stmt = conn.prepare(
            "SELECT path, attempts, completed, last_at FROM counters ORDER BY attempts DESC LIMIT 500",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Counter {
                path: row.get(0)?,
                attempts: row.get(1)?,
                completed: row.get(2)?,
                last_at: row.get(3)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn counter_for(&self, path: &str) -> Result<Counter> {
        let conn = lock(&self.app)?;
        let found = conn
            .query_row(
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
            .optional()?;
        Ok(found.unwrap_or(Counter {
            path: path.to_string(),
            attempts: 0,
            completed: 0,
            last_at: None,
        }))
    }

    pub(crate) fn bump_attempt(&self, path: &str) {
        if let Ok(conn) = lock(&self.app) {
            let _ = conn.execute(
                "INSERT INTO counters(path, attempts, completed, last_at) VALUES(?1, 1, 0, ?2)
                 ON CONFLICT(path) DO UPDATE SET attempts = attempts + 1, last_at = excluded.last_at",
                params![path, db::now_iso()],
            );
        }
    }

    pub(crate) fn bump_completed(&self, path: &str) {
        if let Ok(conn) = lock(&self.app) {
            let _ = conn.execute(
                "INSERT INTO counters(path, attempts, completed, last_at) VALUES(?1, 0, 1, ?2)
                 ON CONFLICT(path) DO UPDATE SET completed = completed + 1, last_at = excluded.last_at",
                params![path, db::now_iso()],
            );
        }
    }

    // ---- M3U y reproducción ------------------------------------------------------------

    /// URL HTTP(S) absoluta (los reproductores externos siempre reciben HTTP, no FTP).
    pub fn stream_url(&self, path: &str) -> String {
        catalog::absolute_url_with(&self.base_url, path)
    }

    pub fn export_m3u(&self, list_id: i64) -> Result<PathBuf> {
        let (name, items) = {
            let conn = lock(&self.app)?;
            let name: String =
                conn.query_row("SELECT name FROM lists WHERE id = ?1", [list_id], |r| {
                    r.get(0)
                })?;
            let mut stmt = conn.prepare(
                "SELECT name, path FROM list_items WHERE list_id = ?1 ORDER BY position",
            )?;
            let rows = stmt
                .query_map([list_id], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            (name, rows)
        };
        let mut body = String::from("#EXTM3U\n");
        for (item_name, path) in items {
            body.push_str(&format!(
                "#EXTINF:-1,{}\n{}\n",
                item_name,
                catalog::absolute_url_with(&self.base_url, &path)
            ));
        }
        let exports = self.data_dir.join("exports");
        std::fs::create_dir_all(&exports)?;
        let slug: String = name
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect();
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let file = exports.join(format!("{}-{}.m3u8", slug.trim_matches('-'), stamp));
        std::fs::write(&file, body)?;
        Ok(file)
    }

    /// Lanza el reproductor externo con la URL remota. `Err("no-player")` si no hay ninguno.
    pub fn play(&self, path: &str) -> Result<()> {
        let url = self.stream_url(path);
        let configured = self.settings().ok().map(|s| s.player);
        let mut candidates: Vec<String> = Vec::new();
        if let Some(player) = configured.filter(|v| !v.trim().is_empty() && v != "auto") {
            candidates.push(player);
        }
        candidates.extend(["vlc", "mpv", "celluloid", "totem"].map(String::from));
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
        Err(Error::msg("no-player"))
    }
}
