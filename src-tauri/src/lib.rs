mod catalog;
mod commands;
mod db;
mod downloads;
mod index;
mod library;
mod norm;
mod state;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let state = state::init(&handle)?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::index_stats,
            commands::index_bootstrap,
            commands::search,
            commands::browse,
            commands::cache_clear,
            commands::crawl_start,
            commands::crawl_stop,
            commands::favorites_list,
            commands::favorite_add,
            commands::favorite_remove,
            commands::lists_all,
            commands::list_create,
            commands::list_delete,
            commands::list_rename,
            commands::list_items,
            commands::list_add,
            commands::list_remove,
            commands::list_reorder,
            commands::list_mark_watched,
            commands::counters_all,
            commands::counter_get,
            commands::settings_get,
            commands::settings_set,
            commands::recent_searches,
            commands::download_enqueue,
            commands::downloads_list,
            commands::download_pause,
            commands::download_cancel,
            commands::download_resume,
            commands::downloads_clear,
            commands::export_m3u,
            commands::play,
        ])
        .run(tauri::generate_context!())
        .expect("error al ejecutar NovaFlix");
}
