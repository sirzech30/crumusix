use serde::{Serialize, Deserialize};
use parking_lot::Mutex;
use std::sync::OnceLock;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlaybackState {
    pub is_playing: bool,
    pub current_track_id: Option<String>,
    pub active_provider: Option<String>,
    pub volume: f32,
    pub position_ms: u64,
}

fn global_state() -> &'static Mutex<PlaybackState> {
    static STATE: OnceLock<Mutex<PlaybackState>> = OnceLock::new();
    STATE.get_or_init(|| {
        Mutex::new(PlaybackState {
            is_playing: false,
            current_track_id: None,
            active_provider: None,
            volume: 0.8,
            position_ms: 0,
        })
    })
}

pub fn update_global_playback_state<F>(f: F)
where
    F: FnOnce(&mut PlaybackState),
{
    let mut state = global_state().lock();
    f(&mut state);
}

pub fn get_global_playback_state() -> PlaybackState {
    global_state().lock().clone()
}
