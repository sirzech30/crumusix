pub mod storage;
pub mod metadata;
pub mod artwork;
pub mod db;
pub mod migrations;

pub use db::SqliteDbState;

use tauri::{State, AppHandle};
use metadata::{CachedTrack, CachedPlaylist, get_cached_track, cache_track, get_cached_playlist, cache_playlist};
use artwork::get_cached_artwork;
use std::sync::Arc;
use parking_lot::Mutex;

pub struct CacheDbState {
    pub db: Arc<Mutex<rusqlite::Connection>>,
}

#[tauri::command]
pub async fn cache_get_artwork(
    url: String,
    identifier: String,
    app_handle: AppHandle,
) -> Result<String, String> {
    get_cached_artwork(&app_handle, &url, &identifier).await
}

#[tauri::command]
pub fn cache_set_track(
    track: CachedTrack,
    state: State<'_, CacheDbState>,
) -> Result<(), String> {
    let db = state.db.lock();
    cache_track(&db, track)
}

#[tauri::command]
pub fn cache_get_track(
    id: String,
    state: State<'_, CacheDbState>,
) -> Result<Option<CachedTrack>, String> {
    let db = state.db.lock();
    Ok(get_cached_track(&db, &id))
}

#[tauri::command]
pub fn cache_set_playlist(
    playlist: CachedPlaylist,
    state: State<'_, CacheDbState>,
) -> Result<(), String> {
    let db = state.db.lock();
    cache_playlist(&db, playlist)
}

#[tauri::command]
pub fn cache_get_playlist(
    id: String,
    state: State<'_, CacheDbState>,
) -> Result<Option<CachedPlaylist>, String> {
    let db = state.db.lock();
    Ok(get_cached_playlist(&db, &id))
}

#[tauri::command]
pub fn library_search(
    query: String,
    state: State<'_, SqliteDbState>,
) -> Result<Vec<crate::Track>, String> {
    let conn = state.conn.lock();
    db::search_tracks(&conn, &query)
}

#[tauri::command]
pub fn library_get_all(
    state: State<'_, SqliteDbState>,
) -> Result<Vec<crate::Track>, String> {
    let conn = state.conn.lock();
    let mut stmt = conn.prepare(
        "SELECT id, title, artist, album, duration, source, thumbnail FROM tracks ORDER BY artist, album, title;"
    ).map_err(|e| e.to_string())?;
    
    let rows = stmt.query_map([], |row| {
        Ok(crate::Track {
            id: row.get(0)?,
            title: row.get(1)?,
            artist: row.get(2)?,
            album: row.get(3)?,
            duration: row.get(4)?,
            source: row.get(5)?,
            thumbnail: row.get(6)?,
        })
    }).map_err(|e| e.to_string())?;
    
    let mut results = Vec::new();
    for r in rows {
        if let Ok(track) = r {
            results.push(track);
        }
    }
    Ok(results)
}
