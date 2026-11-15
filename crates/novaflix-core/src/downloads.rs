//! Cola de descargas: `.part` + reanudación (Range/REST), progreso, límite de velocidad,
//! verificación de tamaño y descarga recursiva de carpetas.

use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use futures::StreamExt;
use rusqlite::params;
use serde::Serialize;
use tokio::io::AsyncWriteExt;

use crate::{
    catalog, db,
    error::{lock, Error, Result, UiError},
    Core, Event, Level, Mode,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DownloadState {
    Queued,
    Active,
    Paused,
    Done,
    Error,
    Cancelled,
}

impl DownloadState {
    pub fn as_str(self) -> &'static str {
        match self {
            DownloadState::Queued => "queued",
            DownloadState::Active => "active",
            DownloadState::Paused => "paused",
            DownloadState::Done => "done",
            DownloadState::Error => "error",
            DownloadState::Cancelled => "cancelled",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "active" => DownloadState::Active,
            "paused" => DownloadState::Paused,
            "done" => DownloadState::Done,
            "error" => DownloadState::Error,
            "cancelled" => DownloadState::Cancelled,
            _ => DownloadState::Queued,
        }
    }

    pub fn is_finished(self) -> bool {
        matches!(
            self,
            DownloadState::Done | DownloadState::Error | DownloadState::Cancelled
        )
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadRow {
    pub id: i64,
    pub path: String,
    pub dest: String,
    pub bytes_total: i64,
    pub bytes_done: i64,
    pub state: DownloadState,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub error: Option<String>,
    pub proto: Option<String>,
    pub group_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct DownloadEvent {
    pub id: i64,
    pub state: DownloadState,
    pub done: u64,
    /// 0 = desconocido.
    pub total: u64,
    pub speed_bps: u64,
    pub proto: Mode,
    pub dest: Option<PathBuf>,
    pub error: Option<UiError>,
}

/// Qué hacer si el destino ya existe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OnExists {
    Replace,
    Skip,
    /// Conserva ambos: `nombre (1).ext`.
    #[default]
    Keep,
}

#[derive(Debug, Clone)]
pub struct PlannedFile {
    pub path: String,
    pub relative: String,
    pub size: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct FolderPlan {
    pub root: String,
    pub files: Vec<PlannedFile>,
    pub total_bytes: u64,
}

/// Control de una descarga en curso.
#[derive(Default)]
pub struct Control {
    stop: AtomicBool,
    /// Si es `true`, al parar se descarta el `.part` (cancelar) en lugar de conservarlo (pausar).
    discard: AtomicBool,
}

pub fn sanitize_file_name(name: &str) -> String {
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

fn unique_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    for n in 1..10_000 {
        let candidate = path.with_file_name(format!("{stem} ({n}){ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    path.to_path_buf()
}

fn part_path(dest: &Path) -> PathBuf {
    PathBuf::from(format!("{}.part", dest.to_string_lossy()))
}

impl Core {
    fn update_row(
        &self,
        id: i64,
        done: i64,
        total: i64,
        state: DownloadState,
        error: Option<&str>,
        proto: Option<Mode>,
    ) {
        if let Ok(conn) = lock(&self.app) {
            let finished = state.is_finished().then(db::now_iso);
            let _ = conn.execute(
                "UPDATE downloads SET bytes_done = ?2, bytes_total = ?3, state = ?4, error = ?5,
                    finished_at = COALESCE(?6, finished_at), proto = COALESCE(?7, proto)
                 WHERE id = ?1",
                params![
                    id,
                    done,
                    total,
                    state.as_str(),
                    error,
                    finished,
                    proto.map(|m| m.as_str())
                ],
            );
        }
    }

    fn emit_download(&self, ev: DownloadEvent) {
        self.emit(Event::Download(ev));
    }

    pub fn downloads(&self) -> Result<Vec<DownloadRow>> {
        let conn = lock(&self.app)?;
        let mut stmt = conn.prepare(
            "SELECT id, path, dest, bytes_total, bytes_done, state, started_at, finished_at, error, proto, group_id
             FROM downloads ORDER BY id DESC LIMIT 300",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(DownloadRow {
                id: row.get(0)?,
                path: row.get(1)?,
                dest: row.get(2)?,
                bytes_total: row.get(3)?,
                bytes_done: row.get(4)?,
                state: DownloadState::parse(&row.get::<_, String>(5)?),
                started_at: row.get(6)?,
                finished_at: row.get(7)?,
                error: row.get(8)?,
                proto: row.get(9)?,
                group_id: row.get(10)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn downloads_clear_finished(&self) -> Result<()> {
        lock(&self.app)?.execute(
            "DELETE FROM downloads WHERE state IN ('done', 'error', 'cancelled')",
            [],
        )?;
        Ok(())
    }

    fn target_dir(&self, dest_dir: Option<PathBuf>) -> Result<PathBuf> {
        let dir = dest_dir.unwrap_or_else(|| self.downloads_dir());
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    /// Encola archivos con la política por defecto (`Keep`). Devuelve los ids creados.
    pub fn download_enqueue(
        self: &Arc<Self>,
        paths: Vec<String>,
        dest_dir: Option<PathBuf>,
    ) -> Result<Vec<i64>> {
        self.download_enqueue_with(paths, dest_dir, OnExists::Keep)
    }

    pub fn download_enqueue_with(
        self: &Arc<Self>,
        paths: Vec<String>,
        dest_dir: Option<PathBuf>,
        on_exists: OnExists,
    ) -> Result<Vec<i64>> {
        let dir = self.target_dir(dest_dir)?;
        let mut ids = Vec::new();
        for path in paths {
            let name = sanitize_file_name(&catalog::decode_name(&db::name_of(&path)));
            if let Some(id) = self.enqueue_one(&path, dir.join(name), on_exists, None)? {
                ids.push(id);
            }
        }
        Ok(ids)
    }

    fn enqueue_one(
        self: &Arc<Self>,
        remote: &str,
        dest: PathBuf,
        on_exists: OnExists,
        group_id: Option<i64>,
    ) -> Result<Option<i64>> {
        let dest = match (dest.exists(), on_exists) {
            (true, OnExists::Skip) => return Ok(None),
            (true, OnExists::Keep) => unique_path(&dest),
            _ => dest,
        };
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let id = {
            let conn = lock(&self.app)?;
            conn.execute(
                "INSERT INTO downloads(path, dest, bytes_total, bytes_done, state, started_at, group_id)
                 VALUES(?1, ?2, 0, 0, 'queued', ?3, ?4)",
                params![remote, dest.to_string_lossy(), db::now_iso(), group_id],
            )?;
            conn.last_insert_rowid()
        };
        self.bump_attempt(remote);
        lock(&self.cancels)?.insert(id, Arc::new(Control::default()));
        self.spawn_run(id, remote.to_string(), dest);
        Ok(Some(id))
    }

    fn spawn_run(self: &Arc<Self>, id: i64, remote: String, dest: PathBuf) {
        let core = self.clone();
        self.rt
            .spawn(async move { core.run_download(id, remote, dest).await });
    }

    pub fn download_pause(&self, id: i64) {
        if let Ok(map) = lock(&self.cancels) {
            if let Some(control) = map.get(&id) {
                control.stop.store(true, Ordering::Relaxed);
            }
        }
    }

    pub fn download_cancel(&self, id: i64) {
        let dest = lock(&self.app).ok().and_then(|c| {
            c.query_row("SELECT dest FROM downloads WHERE id = ?1", [id], |r| {
                r.get::<_, String>(0)
            })
            .ok()
        });
        let running = lock(&self.cancels).ok().and_then(|m| m.get(&id).cloned());
        match running {
            Some(control) => {
                control.discard.store(true, Ordering::Relaxed);
                control.stop.store(true, Ordering::Relaxed);
            }
            None => {
                // No hay tarea viva (p. ej. tras reiniciar): se descarta aquí.
                if let Some(dest) = &dest {
                    let _ = std::fs::remove_file(part_path(Path::new(dest)));
                }
                self.update_row(id, 0, 0, DownloadState::Cancelled, None, None);
                self.emit_download(DownloadEvent {
                    id,
                    state: DownloadState::Cancelled,
                    done: 0,
                    total: 0,
                    speed_bps: 0,
                    proto: self.active_mode(),
                    dest: None,
                    error: None,
                });
            }
        }
    }

    pub fn download_resume(self: &Arc<Self>, id: i64) -> Result<()> {
        let (path, dest) = {
            let conn = lock(&self.app)?;
            conn.query_row(
                "SELECT path, dest FROM downloads WHERE id = ?1",
                [id],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )?
        };
        lock(&self.cancels)?.insert(id, Arc::new(Control::default()));
        self.update_row(id, 0, 0, DownloadState::Queued, None, None);
        self.spawn_run(id, path, PathBuf::from(dest));
        Ok(())
    }

    // ---- Carpetas --------------------------------------------------------------------

    /// Calcula el contenido recursivo de una carpeta. Emite exactamente un `Event::FolderPlan`.
    pub fn plan_folder_download(self: &Arc<Self>, path: &str) {
        let core = self.clone();
        let root = path.to_string();
        self.rt.spawn(async move {
            let result = core.plan_folder(&root).await.map_err(UiError::from);
            core.emit(Event::FolderPlan { root, result });
        });
    }

    async fn plan_folder(&self, root: &str) -> Result<FolderPlan> {
        const MAX_FILES: usize = 20_000;
        const MAX_DEPTH: usize = 24;
        let net = self.net();
        let root = if root.ends_with('/') {
            root.to_string()
        } else {
            format!("{root}/")
        };
        let mut files = Vec::new();
        let mut stack = vec![(root.clone(), String::new(), 0_usize)];
        while let Some((dir, rel, depth)) = stack.pop() {
            for entry in net.list_dir(&dir).await? {
                let child = format!("{}{}", dir, entry.href);
                let relative = format!("{}{}", rel, entry.name);
                if entry.is_dir {
                    if depth < MAX_DEPTH {
                        stack.push((child, format!("{relative}/"), depth + 1));
                    }
                } else {
                    files.push(PlannedFile {
                        path: child,
                        relative,
                        size: entry.size.map(|s| s as u64),
                    });
                    if files.len() > MAX_FILES {
                        return Err(Error::msg(
                            "La carpeta tiene demasiados archivos para descargarla de una vez.",
                        ));
                    }
                }
            }
        }
        files.sort_by(|a, b| a.relative.cmp(&b.relative));
        let total_bytes = files.iter().filter_map(|f| f.size).sum();
        Ok(FolderPlan {
            root,
            files,
            total_bytes,
        })
    }

    /// Encola un plan de carpeta conservando la estructura de subcarpetas.
    pub fn download_folder(
        self: &Arc<Self>,
        plan: &FolderPlan,
        dest_dir: Option<PathBuf>,
        on_exists: OnExists,
    ) -> Result<Vec<i64>> {
        let base = self
            .target_dir(dest_dir)?
            .join(sanitize_file_name(&catalog::decode_name(&db::name_of(
                &plan.root,
            ))));
        let mut ids: Vec<i64> = Vec::new();
        for file in &plan.files {
            let mut dest = base.clone();
            for component in file.relative.split('/').filter(|c| !c.is_empty()) {
                dest.push(sanitize_file_name(component));
            }
            let group = ids.first().copied();
            if let Some(id) = self.enqueue_one(&file.path, dest, on_exists, group)? {
                if ids.is_empty() {
                    lock(&self.app)?
                        .execute("UPDATE downloads SET group_id = ?1 WHERE id = ?1", [id])?;
                }
                ids.push(id);
            }
        }
        Ok(ids)
    }

    // ---- Tarea de descarga -----------------------------------------------------------

    async fn run_download(self: Arc<Self>, id: i64, remote: String, dest: PathBuf) {
        let Ok(_permit) = self.dl_sem.clone().acquire_owned().await else {
            return;
        };
        let control = lock(&self.cancels)
            .ok()
            .and_then(|m| m.get(&id).cloned())
            .unwrap_or_default();
        let part = part_path(&dest);
        let proto = self.active_mode();

        let finish = |state: DownloadState, done: u64, total: u64, error: Option<Error>| {
            let message = error.as_ref().map(|e| e.to_string());
            self.update_row(
                id,
                done as i64,
                total as i64,
                state,
                message.as_deref(),
                Some(proto),
            );
            self.emit_download(DownloadEvent {
                id,
                state,
                done,
                total,
                speed_bps: 0,
                proto,
                dest: (state == DownloadState::Done).then(|| dest.clone()),
                error: error.map(UiError::from),
            });
            if let Ok(mut map) = lock(&self.cancels) {
                map.remove(&id);
            }
        };

        if control.stop.load(Ordering::Relaxed) {
            return finish(DownloadState::Paused, 0, 0, None);
        }
        self.update_row(id, 0, 0, DownloadState::Active, None, Some(proto));

        let limit_bps = self
            .settings()
            .map(|s| s.speed_limit_kbps as u64 * 1024)
            .unwrap_or(0);
        let mut restarted = false;

        'attempt: loop {
            let mut start = tokio::fs::metadata(&part)
                .await
                .map(|m| m.len())
                .unwrap_or(0);
            let download = match self.net().open(&remote, start).await {
                Ok(d) => d,
                Err(err) => return finish(DownloadState::Error, start, 0, Some(err)),
            };
            if start > 0 && !download.resumed {
                self.notice(
                    Level::Warn,
                    "El servidor no admite reanudar: se empieza de nuevo.",
                );
                let _ = tokio::fs::remove_file(&part).await;
                start = 0;
            }
            let total = download.total.unwrap_or(0);

            let mut file = match tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&part)
                .await
            {
                Ok(f) => f,
                Err(err) => return finish(DownloadState::Error, start, total, Some(err.into())),
            };

            let mut stream = download.stream;
            let mut done = start;
            let began = Instant::now();
            let mut last_emit = Instant::now();
            let (mut last_done, mut speed) = (done, 0_u64);

            while let Some(chunk) = stream.next().await {
                if control.stop.load(Ordering::Relaxed) {
                    let _ = file.flush().await;
                    drop(file);
                    if control.discard.load(Ordering::Relaxed) {
                        let _ = tokio::fs::remove_file(&part).await;
                        return finish(DownloadState::Cancelled, 0, total, None);
                    }
                    return finish(DownloadState::Paused, done, total, None);
                }
                match chunk {
                    Ok(bytes) => {
                        if let Err(err) = file.write_all(&bytes).await {
                            return finish(DownloadState::Error, done, total, Some(err.into()));
                        }
                        done += bytes.len() as u64;
                    }
                    // Un servidor FTP que acepta REST pero reenvía desde el inicio: reiniciar una vez.
                    Err(Error::Ftp(msg)) if msg.contains("REST") && !restarted => {
                        drop(file);
                        let _ = tokio::fs::remove_file(&part).await;
                        restarted = true;
                        continue 'attempt;
                    }
                    Err(err) => {
                        let _ = file.flush().await;
                        return finish(DownloadState::Error, done, total, Some(err));
                    }
                }
                if limit_bps > 0 {
                    let expected =
                        Duration::from_secs_f64((done - start) as f64 / limit_bps as f64);
                    if let Some(wait) = expected.checked_sub(began.elapsed()) {
                        tokio::time::sleep(wait).await;
                    }
                }
                if last_emit.elapsed() > Duration::from_millis(180) {
                    let dt = last_emit.elapsed().as_secs_f64().max(0.001);
                    let instant = ((done - last_done) as f64 / dt) as u64;
                    speed = if speed == 0 {
                        instant
                    } else {
                        (speed + instant) / 2
                    };
                    last_emit = Instant::now();
                    last_done = done;
                    self.update_row(
                        id,
                        done as i64,
                        total as i64,
                        DownloadState::Active,
                        None,
                        Some(proto),
                    );
                    self.emit_download(DownloadEvent {
                        id,
                        state: DownloadState::Active,
                        done,
                        total,
                        speed_bps: speed,
                        proto,
                        dest: None,
                        error: None,
                    });
                }
            }

            let _ = file.flush().await;
            drop(file);

            // Verificación de tamaño (FR-025): si no coincide, el `.part` no es de fiar.
            if total > 0 && done != total {
                let _ = tokio::fs::remove_file(&part).await;
                return finish(
                    DownloadState::Error,
                    done,
                    total,
                    Some(Error::msg(format!(
                        "El tamaño no coincide ({done} de {total} bytes). Reintenta la descarga."
                    ))),
                );
            }
            if let Err(err) = tokio::fs::rename(&part, &dest).await {
                return finish(DownloadState::Error, done, total, Some(err.into()));
            }
            self.bump_completed(&remote);
            let total = if total > 0 { total } else { done };
            return finish(DownloadState::Done, done, total, None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_hostile_names() {
        assert_eq!(sanitize_file_name("../../etc/passwd"), "_.._etc_passwd");
        assert_eq!(sanitize_file_name(".."), "archivo");
        assert_eq!(sanitize_file_name("a\0b:c*d?.mp4"), "a_b_c_d_.mp4");
        assert_eq!(sanitize_file_name("   "), "archivo");
        assert!(!sanitize_file_name("C:\\Windows\\x").contains(['\\', ':']));
    }

    #[test]
    fn keeps_both_files() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("a.mp4");
        std::fs::write(&first, b"x").unwrap();
        assert_eq!(unique_path(&first), dir.path().join("a (1).mp4"));
        std::fs::write(dir.path().join("a (1).mp4"), b"x").unwrap();
        assert_eq!(unique_path(&first), dir.path().join("a (2).mp4"));
    }

    #[test]
    fn state_roundtrip() {
        for s in [
            DownloadState::Queued,
            DownloadState::Active,
            DownloadState::Paused,
            DownloadState::Done,
            DownloadState::Error,
            DownloadState::Cancelled,
        ] {
            assert_eq!(DownloadState::parse(s.as_str()), s);
        }
    }
}
