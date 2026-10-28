use std::sync::Arc;

use serde_json::Value;
use tauri::{AppHandle, State};

use crate::{
    downloads, index, library, state::AppState, visuales,
};

#[tauri::command]
pub async fn app_info(state: State<'_, Arc<AppState>>) -> Result<Value, String> {
    Ok(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "data_dir": state.data_dir.to_string_lossy(),
        "downloads_dir": state.downloads_dir.to_string_lossy(),
        "base_url": visuales::BASE,
    }))
}

#[tauri::command]
pub async fn index_stats(state: State<'_, Arc<AppState>>) -> Result<index::IndexStats, String> {
    index::stats(state.inner())
}

#[tauri::command]
pub async fn index_bootstrap(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    force: bool,
) -> Result<index::IndexStats, String> {
    index::bootstrap(&app, state.inner(), force).await
}

#[tauri::command]
pub async fn search(
    state: State<'_, Arc<AppState>>,
    query: String,
    filter: Option<index::SearchFilter>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<index::SearchPage, String> {
    index::search(
        state.inner(),
        &query,
        &filter.unwrap_or_default(),
        limit.unwrap_or(60),
        offset.unwrap_or(0),
    )
}

#[tauri::command]
pub async fn browse(
    state: State<'_, Arc<AppState>>,
    path: String,
    force: Option<bool>,
) -> Result<index::Listing, String> {
    index::browse(state.inner(), &path, force.unwrap_or(false)).await
}

#[tauri::command]
pub async fn cache_clear(
    state: State<'_, Arc<AppState>>,
    keep_index: Option<bool>,
) -> Result<index::IndexStats, String> {
    index::cache_clear(state.inner(), keep_index.unwrap_or(true))
}

#[tauri::command]
pub async fn crawl_start(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    max_dirs: Option<usize>,
) -> Result<(), String> {
    let state = state.inner().clone();
    let limit = max_dirs.unwrap_or(250_000);
    tauri::async_runtime::spawn(async move {
        let _ = index::crawl(app, state, limit).await;
    });
    Ok(())
}

#[tauri::command]
pub async fn crawl_stop(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state
        .crawl_stop
        .store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub async fn favorites_list(state: State<'_, Arc<AppState>>) -> Result<Vec<library::Favorite>, String> {
    library::favorites(state.inner())
}

#[tauri::command]
pub async fn favorite_add(
    state: State<'_, Arc<AppState>>,
    target_kind: String,
    path: String,
    label: Option<String>,
    note: Option<String>,
    size: Option<i64>,
) -> Result<(), String> {
    library::favorite_add(
        state.inner(),
        &target_kind,
        &path,
        label.as_deref().unwrap_or(""),
        note,
        size,
    )
}

#[tauri::command]
pub async fn favorite_remove(
    state: State<'_, Arc<AppState>>,
    target_kind: String,
    path: String,
) -> Result<(), String> {
    library::favorite_remove(state.inner(), &target_kind, &path)
}

#[tauri::command]
pub async fn lists_all(state: State<'_, Arc<AppState>>) -> Result<Vec<library::ListSummary>, String> {
    library::lists(state.inner())
}

#[tauri::command]
pub async fn list_create(
    state: State<'_, Arc<AppState>>,
    name: String,
    kind: String,
) -> Result<i64, String> {
    library::list_create(state.inner(), &name, &kind)
}

#[tauri::command]
pub async fn list_delete(state: State<'_, Arc<AppState>>, list_id: i64) -> Result<(), String> {
    library::list_delete(state.inner(), list_id)
}

#[tauri::command]
pub async fn list_rename(
    state: State<'_, Arc<AppState>>,
    list_id: i64,
    name: String,
) -> Result<(), String> {
    library::list_rename(state.inner(), list_id, &name)
}

#[tauri::command]
pub async fn list_items(
    state: State<'_, Arc<AppState>>,
    list_id: i64,
) -> Result<Vec<library::ListItem>, String> {
    library::list_items(state.inner(), list_id)
}

#[tauri::command]
pub async fn list_add(
    state: State<'_, Arc<AppState>>,
    list_id: i64,
    path: String,
    name: Option<String>,
) -> Result<(), String> {
    library::list_add(state.inner(), list_id, &path, name)
}

#[tauri::command]
pub async fn list_remove(
    state: State<'_, Arc<AppState>>,
    list_id: i64,
    path: String,
) -> Result<(), String> {
    library::list_remove(state.inner(), list_id, &path)
}

#[tauri::command]
pub async fn list_reorder(
    state: State<'_, Arc<AppState>>,
    list_id: i64,
    paths: Vec<String>,
) -> Result<(), String> {
    library::list_reorder(state.inner(), list_id, paths)
}

#[tauri::command]
pub async fn list_mark_watched(
    state: State<'_, Arc<AppState>>,
    list_id: i64,
    path: String,
    watched: bool,
) -> Result<(), String> {
    library::list_mark_watched(state.inner(), list_id, &path, watched)
}

#[tauri::command]
pub async fn counters_all(state: State<'_, Arc<AppState>>) -> Result<Vec<library::Counter>, String> {
    library::counters(state.inner())
}

#[tauri::command]
pub async fn counter_get(
    state: State<'_, Arc<AppState>>,
    path: String,
) -> Result<library::Counter, String> {
    library::counter_for(state.inner(), &path)
}

#[tauri::command]
pub async fn settings_get(state: State<'_, Arc<AppState>>) -> Result<Value, String> {
    library::settings(state.inner())
}

#[tauri::command]
pub async fn settings_set(
    state: State<'_, Arc<AppState>>,
    key: String,
    value: String,
) -> Result<(), String> {
    library::setting_set(state.inner(), &key, &value)
}

#[tauri::command]
pub async fn recent_searches(state: State<'_, Arc<AppState>>) -> Result<Vec<String>, String> {
    library::recent_searches(state.inner())
}

#[tauri::command]
pub async fn download_enqueue(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    paths: Vec<String>,
    dest: Option<String>,
) -> Result<Vec<i64>, String> {
    downloads::enqueue(app, state.inner().clone(), paths, dest)
}

#[tauri::command]
pub async fn downloads_list(state: State<'_, Arc<AppState>>) -> Result<Vec<downloads::DownloadRow>, String> {
    downloads::list(state.inner())
}

#[tauri::command]
pub async fn download_pause(state: State<'_, Arc<AppState>>, id: i64) -> Result<(), String> {
    downloads::pause(state.inner(), id)
}

#[tauri::command]
pub async fn download_cancel(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    id: i64,
) -> Result<(), String> {
    downloads::cancel(&app, state.inner(), id)
}

#[tauri::command]
pub async fn download_resume(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    id: i64,
) -> Result<(), String> {
    downloads::resume(app, state.inner().clone(), id)
}

#[tauri::command]
pub async fn downloads_clear(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    downloads::clear_finished(state.inner())
}

#[tauri::command]
pub async fn export_m3u(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    list_id: i64,
) -> Result<String, String> {
    library::export_m3u(&app, state.inner(), list_id)
        .map(|path| path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn play(state: State<'_, Arc<AppState>>, path: String) -> Result<(), String> {
    library::play(state.inner(), &path)
}
