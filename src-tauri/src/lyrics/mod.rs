pub mod models;
pub mod provider;
pub mod parser;
pub mod lrclib;
pub mod cache;
pub mod manager;

use models::Lyrics;
use manager::get_lyrics_orchestrator;
use tauri::{AppHandle, command};

#[command]
pub async fn lyrics_get_for_track(
    app_handle: AppHandle,
    track_id: String,
    title: String,
    artist: String,
    album: String,
    duration_ms: u32,
) -> Result<Lyrics, String> {
    get_lyrics_orchestrator(&app_handle, &track_id, &title, &artist, &album, duration_ms).await
}

#[command]
pub fn lyrics_get_cache_stats(app_handle: AppHandle) -> Result<usize, String> {
    Ok(cache::get_lyrics_cache_count(&app_handle))
}

#[command]
pub fn lyrics_purge_cache(app_handle: AppHandle) -> Result<(), String> {
    cache::purge_lyrics_cache(&app_handle)
}
