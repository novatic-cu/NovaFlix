//! Migración de los datos de v1 a v2 (data-model.md §5).

use std::path::Path;

use rusqlite::Connection;

use crate::{
    db::{user_version, SCHEMA_VERSION},
    error::{Error, Result},
};

pub struct Outcome {
    /// Aviso para el usuario si algo salió mal (la migración no pierde datos).
    pub notice: Option<String>,
}

fn has_table(conn: &Connection, name: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
        [name],
        |_| Ok(()),
    )
    .is_ok()
}

fn has_column(conn: &Connection, table: &str, column: &str) -> bool {
    conn.prepare(&format!("PRAGMA table_info({table})"))
        .and_then(|mut stmt| {
            let names: Vec<String> = stmt
                .query_map([], |r| r.get::<_, String>(1))?
                .flatten()
                .collect();
            Ok(names.iter().any(|n| n == column))
        })
        .unwrap_or(false)
}

/// Prepara `app.db` y `cache.db` en `dir` antes de abrirlas con normalidad.
pub fn prepare(dir: &Path) -> Result<Outcome> {
    let mut notice = None;
    let app = dir.join("app.db");
    if app.exists() {
        match migrate_app(&app) {
            Ok(()) => {}
            Err(err @ Error::Msg(_)) => return Err(err), // versión futura: no se toca
            Err(err) => {
                // Conservar los datos de v1 intactos y arrancar limpio.
                let failed = dir.join("app.db.failed");
                let _ = std::fs::rename(&app, &failed);
                for suffix in ["-wal", "-shm"] {
                    let _ = std::fs::remove_file(dir.join(format!("app.db{suffix}")));
                }
                notice = Some(format!(
                    "No se pudieron migrar tus datos de la versión anterior ({err}). \
                     Se conservan en {} y en app.db.v1.bak.",
                    failed.display()
                ));
            }
        }
    }
    migrate_cache(&dir.join("cache.db"));
    Ok(Outcome { notice })
}

fn migrate_app(path: &Path) -> Result<()> {
    let conn = Connection::open(path)?;
    let version = user_version(&conn);
    if version > SCHEMA_VERSION {
        return Err(Error::msg(format!(
            "Los datos de NovaFlix son de una versión más nueva (esquema {version}). \
             Actualiza la aplicación; no se ha modificado nada."
        )));
    }
    if version == SCHEMA_VERSION || !has_table(&conn, "items") {
        return Ok(()); // ya migrada, o vacía
    }

    // Es de v1: copia de seguridad (una sola vez) y migración transaccional.
    let backup = path.with_file_name("app.db.v1.bak");
    if !backup.exists() {
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)").ok();
        std::fs::copy(path, &backup)?;
    }

    conn.execute_batch("BEGIN")?;
    let result = (|| -> Result<()> {
        if has_table(&conn, "downloads") {
            if !has_column(&conn, "downloads", "proto") {
                conn.execute("ALTER TABLE downloads ADD COLUMN proto TEXT", [])?;
            }
            if !has_column(&conn, "downloads", "group_id") {
                conn.execute("ALTER TABLE downloads ADD COLUMN group_id INTEGER", [])?;
            }
            conn.execute(
                "UPDATE downloads SET state = 'paused' WHERE state IN ('active','queued')",
                [],
            )?;
        }
        // v1 guardaba las rutas con `&amp;` literal (el servidor escapa `&` en sus enlaces).
        for (table, column) in [
            ("favorites", "path"),
            ("list_items", "path"),
            ("items", "path"),
            ("counters", "path"),
            ("downloads", "path"),
        ] {
            if has_table(&conn, table) {
                conn.execute(
                    &format!(
                        "UPDATE OR IGNORE {table} SET {column} = REPLACE({column}, '&amp;', '&') \
                         WHERE {column} LIKE '%&amp;%'"
                    ),
                    [],
                )?;
            }
        }
        conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        Ok(())
    })();
    match result {
        Ok(()) => conn.execute_batch("COMMIT")?,
        Err(err) => {
            let _ = conn.execute_batch("ROLLBACK");
            return Err(err);
        }
    }
    Ok(())
}

/// `cache.db` es desechable: se adopta si es compatible; si no, se borra.
fn migrate_cache(path: &Path) {
    if !path.exists() {
        return;
    }
    let ok = Connection::open(path)
        .map(|conn| {
            let version = user_version(&conn);
            version <= SCHEMA_VERSION
                && (version == SCHEMA_VERSION
                    || (has_table(&conn, "dir_index")
                        && has_column(&conn, "dir_index", "source")
                        && has_table(&conn, "dir_cache")))
        })
        .unwrap_or(false);
    if !ok {
        let _ = std::fs::remove_file(path);
        for suffix in ["-wal", "-shm"] {
            let _ = std::fs::remove_file(path.with_file_name(format!("cache.db{suffix}")));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    /// Esquema de `app.db` tal como lo creaba v1 (`git show v1:src-tauri/src/db.rs`).
    const V1_SCHEMA: &str = r#"
        CREATE TABLE items(path TEXT PRIMARY KEY, name TEXT NOT NULL, kind TEXT NOT NULL, ext TEXT,
            section TEXT, year INTEGER, size INTEGER, first_seen TEXT NOT NULL, last_seen TEXT NOT NULL);
        CREATE TABLE favorites(id INTEGER PRIMARY KEY AUTOINCREMENT,
            target_kind TEXT NOT NULL CHECK(target_kind IN ('item','folder')), path TEXT NOT NULL,
            label TEXT, note TEXT, created_at TEXT NOT NULL, UNIQUE(target_kind, path));
        CREATE TABLE lists(id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL,
            kind TEXT NOT NULL CHECK(kind IN ('playlist','watchlist')), created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
        CREATE TABLE list_items(list_id INTEGER NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
            path TEXT NOT NULL, name TEXT NOT NULL, position INTEGER NOT NULL, added_at TEXT NOT NULL,
            watched_at TEXT, PRIMARY KEY(list_id, path));
        CREATE TABLE downloads(id INTEGER PRIMARY KEY AUTOINCREMENT, path TEXT NOT NULL, dest TEXT NOT NULL,
            bytes_total INTEGER DEFAULT 0, bytes_done INTEGER DEFAULT 0, state TEXT NOT NULL,
            started_at TEXT NOT NULL, finished_at TEXT, error TEXT);
        CREATE TABLE counters(path TEXT PRIMARY KEY, attempts INTEGER NOT NULL DEFAULT 0,
            completed INTEGER NOT NULL DEFAULT 0, last_at TEXT);
        CREATE TABLE searches(id INTEGER PRIMARY KEY AUTOINCREMENT, query TEXT NOT NULL, hits INTEGER NOT NULL, at TEXT NOT NULL);
        CREATE TABLE settings(key TEXT PRIMARY KEY, value TEXT NOT NULL);
    "#;

    fn v1_fixture(dir: &Path) {
        let conn = Connection::open(dir.join("app.db")).unwrap();
        conn.execute_batch(V1_SCHEMA).unwrap();
        conn.execute_batch(
            r#"
            INSERT INTO favorites(target_kind,path,label,note,created_at) VALUES
              ('folder','/Peliculas/Wicked%20%282024%29/','Wicked','mola','2026-10-30T10:00:00Z'),
              ('item','/Series/A.mkv','A',NULL,'2026-10-31T10:00:00Z'),
              ('folder','/Cursos/Java,%20Python%20&amp;%20C/','Cursos','roto en v1','2026-11-01T10:00:00Z');
            INSERT INTO lists(name,kind,created_at,updated_at) VALUES('Noche de cine','playlist','2026-10-30T10:00:00Z','2026-10-30T10:00:00Z');
            INSERT INTO list_items(list_id,path,name,position,added_at) VALUES
              (1,'/Peliculas/a.mp4','a',0,'2026-10-30T10:00:00Z'),(1,'/Peliculas/b.mp4','b',1,'2026-10-30T10:00:00Z');
            INSERT INTO counters(path,attempts,completed,last_at) VALUES('/Peliculas/a.mp4',3,1,'2026-11-01T00:00:00Z');
            INSERT INTO settings(key,value) VALUES('cache_ttl_hours','6'),('player','mpv');
            INSERT INTO downloads(path,dest,bytes_total,bytes_done,state,started_at) VALUES
              ('/Peliculas/a.mp4','/tmp/a.mp4',100,40,'active','2026-11-01T00:00:00Z'),
              ('/Peliculas/b.mp4','/tmp/b.mp4',100,100,'done','2026-11-01T00:00:00Z');
            "#,
        )
        .unwrap();
    }

    fn counts(conn: &Connection) -> Vec<i64> {
        [
            "favorites",
            "lists",
            "list_items",
            "counters",
            "settings",
            "downloads",
        ]
        .iter()
        .map(|t| {
            conn.query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |r| r.get(0))
                .unwrap()
        })
        .collect()
    }

    #[test]
    fn v1_data_survives_migration() {
        let dir = tempfile::tempdir().unwrap();
        v1_fixture(dir.path());
        let before = counts(&Connection::open(dir.path().join("app.db")).unwrap());

        let outcome = prepare(dir.path()).unwrap();
        assert!(outcome.notice.is_none());
        assert!(dir.path().join("app.db.v1.bak").exists());

        let conn = db::open_app(&dir.path().join("app.db")).unwrap();
        assert_eq!(db::user_version(&conn), SCHEMA_VERSION);
        assert_eq!(counts(&conn), before, "no se pierde ninguna fila");
        assert_eq!(db::get_setting(&conn, "player").as_deref(), Some("mpv"));
        // La descarga en curso queda en pausa; la terminada no cambia.
        let states: Vec<String> = conn
            .prepare("SELECT state FROM downloads ORDER BY id")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .flatten()
            .collect();
        assert_eq!(states, ["paused", "done"]);
        // Las rutas siguen siendo canónicas (sin tocar).
        let path: String = conn
            .query_row("SELECT path FROM favorites WHERE label='Wicked'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(path, "/Peliculas/Wicked%20%282024%29/");
    }

    #[test]
    fn repairs_html_escaped_paths_from_v1() {
        let dir = tempfile::tempdir().unwrap();
        v1_fixture(dir.path());
        prepare(dir.path()).unwrap();
        let conn = Connection::open(dir.path().join("app.db")).unwrap();
        let path: String = conn
            .query_row("SELECT path FROM favorites WHERE label='Cursos'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(path, "/Cursos/Java,%20Python%20&%20C/");
    }

    #[test]
    fn migration_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        v1_fixture(dir.path());
        prepare(dir.path()).unwrap();
        let first = counts(&Connection::open(dir.path().join("app.db")).unwrap());
        prepare(dir.path()).unwrap();
        prepare(dir.path()).unwrap();
        assert_eq!(
            counts(&Connection::open(dir.path().join("app.db")).unwrap()),
            first
        );
    }

    #[test]
    fn future_versions_are_not_touched() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.db");
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(V1_SCHEMA).unwrap();
            conn.pragma_update(None, "user_version", SCHEMA_VERSION + 5)
                .unwrap();
        }
        let err = prepare(dir.path())
            .err()
            .expect("debe rechazar")
            .to_string();
        assert!(err.contains("más nueva"), "{err}");
        assert!(!dir.path().join("app.db.v1.bak").exists());
        assert_eq!(
            user_version(&Connection::open(&path).unwrap()),
            SCHEMA_VERSION + 5
        );
    }

    #[test]
    fn fresh_directory_needs_nothing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(prepare(dir.path()).unwrap().notice.is_none());
        assert!(!dir.path().join("app.db.v1.bak").exists());
    }
}
