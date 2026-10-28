use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::AtomicBool,
        Arc, Mutex,
    },
};

use rusqlite::Connection;
use tauri::{AppHandle, Manager};
use tokio::sync::Semaphore;

pub struct AppState {
    pub cache: Mutex<Connection>,
    pub app: Mutex<Connection>,
    pub http: reqwest::Client,
    pub data_dir: PathBuf,
    pub downloads_dir: PathBuf,
    pub cancels: Mutex<HashMap<i64, Arc<AtomicBool>>>,
    pub dl_sem: Arc<Semaphore>,
    pub crawl_stop: Arc<AtomicBool>,
}

pub fn init(handle: &AppHandle) -> Result<Arc<AppState>, String> {
    let data_dir = handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("no se pudo resolver app_data_dir: {e}"))?;
    std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;

    let downloads_dir = data_dir.join("Downloads");
    std::fs::create_dir_all(&downloads_dir).map_err(|e| e.to_string())?;

    let cache = crate::db::open_cache(&data_dir.join("cache.db"))?;
    let app = crate::db::open_app(&data_dir.join("app.db"))?;

    let http = reqwest::Client::builder()
        .user_agent("NovaFlix/0.1 (+desktop search)")
        .timeout(std::time::Duration::from_secs(45))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| e.to_string())?;

    Ok(Arc::new(AppState {
        cache: Mutex::new(cache),
        app: Mutex::new(app),
        http,
        data_dir,
        downloads_dir,
        cancels: Mutex::new(HashMap::new()),
        dl_sem: Arc::new(Semaphore::new(1)),
        crawl_stop: Arc::new(AtomicBool::new(false)),
    }))
}
