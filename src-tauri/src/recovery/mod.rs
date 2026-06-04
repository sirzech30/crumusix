use tauri::{AppHandle, Manager};
use crate::cache::SqliteDbState;
use crate::cache::db::{get_app_state, set_app_state, delete_app_state};
use crate::playback::state::get_global_playback_state;
use crate::session::SessionState;

pub fn save_recovery_checkpoint(app_handle: &AppHandle) -> Result<(), String> {
    let current_state = get_global_playback_state();
    let session = SessionState {
        current_track_id: current_state.current_track_id,
        position_ms: current_state.position_ms,
        volume: current_state.volume,
        active_provider: current_state.active_provider,
        is_playing: current_state.is_playing,
    };
    
    let db_state = app_handle.try_state::<SqliteDbState>()
        .ok_or_else(|| "SqliteDbState not registered".to_string())?;
    let conn = db_state.conn.lock();
    
    let serialized = serde_json::to_string(&session)
        .map_err(|e| format!("Failed to serialize recovery state: {}", e))?;
        
    set_app_state(&conn, "recovery", &serialized)?;
    Ok(())
}

pub fn clear_recovery_checkpoint(app_handle: &AppHandle) {
    if let Some(db_state) = app_handle.try_state::<SqliteDbState>() {
        let conn = db_state.conn.lock();
        let _ = delete_app_state(&conn, "recovery");
    }
}

pub fn detect_unexpected_shutdown(app_handle: &AppHandle) -> Option<SessionState> {
    let db_state = app_handle.try_state::<SqliteDbState>()?;
    let conn = db_state.conn.lock();
    if let Ok(Some(content)) = get_app_state(&conn, "recovery") {
        if let Ok(session) = serde_json::from_str::<SessionState>(&content) {
            return Some(session);
        }
    }
    None
}
