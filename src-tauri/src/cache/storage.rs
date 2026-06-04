use std::path::PathBuf;
use tauri::AppHandle;
use tauri::Manager;

pub fn get_cache_dir(app_handle: &AppHandle) -> PathBuf {
    let dir = app_handle
        .path()
        .app_cache_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    dir.join("crumusix_cache")
}


