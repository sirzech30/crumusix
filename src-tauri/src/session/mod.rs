use serde::{Serialize, Deserialize};
use tauri::{AppHandle, Manager};
use crate::cache::SqliteDbState;
use crate::cache::db::{get_app_state, set_app_state};
use crate::playback::state::get_global_playback_state;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SessionState {
    pub current_track_id: Option<String>,
    pub position_ms: u64,
    pub volume: f32,
    pub active_provider: Option<String>,
    pub is_playing: bool,
}

pub fn save_session(app_handle: &AppHandle) -> Result<(), String> {
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
        .map_err(|e| format!("Failed to serialize session state: {}", e))?;
        
    set_app_state(&conn, "session", &serialized)?;
    Ok(())
}

pub fn load_session(app_handle: &AppHandle) -> Option<SessionState> {
    let db_state = app_handle.try_state::<SqliteDbState>()?;
    let conn = db_state.conn.lock();
    if let Ok(Some(content)) = get_app_state(&conn, "session") {
        return serde_json::from_str(&content).ok();
    }
    None
}
