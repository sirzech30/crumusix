use crate::lyrics::models::Lyrics;
use crate::cache::SqliteDbState;
use tauri::{AppHandle, Manager};
use rusqlite::params;

pub fn read_lyrics_cache(app_handle: &AppHandle, track_id: &str) -> Option<Lyrics> {
    let db_state = app_handle.try_state::<SqliteDbState>()?;
    let conn = db_state.conn.lock();
    let mut stmt = conn.prepare("SELECT lyrics_text FROM lyrics_cache WHERE track_id = ?;").ok()?;
    let mut rows = stmt.query(params![track_id]).ok()?;
    let row = rows.next().ok()??;
    let json_str: String = row.get(0).ok()?;
    serde_json::from_str(&json_str).ok()
}

pub fn write_lyrics_cache(app_handle: &AppHandle, lyrics: &Lyrics) -> Result<(), String> {
    let db_state = app_handle.try_state::<SqliteDbState>()
        .ok_or_else(|| "SqliteDbState not registered".to_string())?;
    let conn = db_state.conn.lock();
    let serialized = serde_json::to_string(lyrics)
        .map_err(|e| format!("Failed to serialize lyrics: {}", e))?;
    let ltype = if lyrics.synced { "synced" } else { "plain" };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    conn.execute(
        "INSERT OR REPLACE INTO lyrics_cache (track_id, lyrics_text, lyrics_type, updated_at) VALUES (?, ?, ?, ?);",
        params![lyrics.track_id, serialized, ltype, now],
    ).map_err(|e| format!("Failed to insert lyrics in DB: {}", e))?;
    Ok(())
}

pub fn get_lyrics_cache_count(app_handle: &AppHandle) -> usize {
    let db_state = match app_handle.try_state::<SqliteDbState>() {
        Some(s) => s,
        None => return 0,
    };
    let conn = db_state.conn.lock();
    conn.query_row("SELECT COUNT(*) FROM lyrics_cache;", [], |row| row.get(0)).unwrap_or(0)
}

pub fn purge_lyrics_cache(app_handle: &AppHandle) -> Result<(), String> {
    let db_state = app_handle.try_state::<SqliteDbState>()
        .ok_or_else(|| "SqliteDbState not registered".to_string())?;
    let conn = db_state.conn.lock();
    conn.execute("DELETE FROM lyrics_cache;", [])
        .map_err(|e| format!("Failed to clear lyrics database: {}", e))?;
    Ok(())
}
