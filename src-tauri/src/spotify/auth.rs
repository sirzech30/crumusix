use std::path::PathBuf;
use tauri::AppHandle;
use tauri::Manager;
use librespot::core::authentication::Credentials;
use librespot::core::cache::Cache;

pub fn get_spotify_cache_path(app_handle: &AppHandle) -> PathBuf {
    let cache_dir = app_handle
        .path()
        .app_cache_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    cache_dir.join("spotify")
}

pub fn get_spotify_cache(app_handle: &AppHandle) -> Option<Cache> {
    let path = get_spotify_cache_path(app_handle);
    // Secure the directory to permissions 700 (standard for librespot)
    std::fs::create_dir_all(&path).ok();
    
    // In librespot 0.8:
    // pub fn new(
    //     credentials_directory: Option<PathBuf>,
    //     volume_directory: Option<PathBuf>,
    //     audio_cache_directory: Option<PathBuf>,
    //     audio_cache_max_size: Option<usize>,
    // ) -> Result<Cache, CacheError>
    Cache::new(Some(path), None, None, None).ok()
}

pub fn get_cached_credentials(app_handle: &AppHandle) -> Option<Credentials> {
    let cache = get_spotify_cache(app_handle)?;
    cache.credentials()
}

pub fn create_credentials_from_token(token: &str) -> Credentials {
    Credentials::with_access_token(token)
}
