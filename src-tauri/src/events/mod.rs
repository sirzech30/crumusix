use tauri::{AppHandle, Emitter};
use serde::Serialize;
use crate::playback::state::{get_global_playback_state, PlaybackState};

#[derive(Serialize, Clone, Debug)]
#[serde(tag = "type", content = "payload")]
pub enum AppEvent {
    #[serde(rename = "playback-state-changed")]
    PlaybackStateChanged(PlaybackState),
    #[serde(rename = "track-changed")]
    TrackChanged {
        track_id: String,
        title: String,
        artist: String,
    },
    #[serde(rename = "queue-updated")]
    QueueUpdated {
        size: usize,
    },
    #[serde(rename = "session-restored")]
    SessionRestored,
}

pub fn emit_global_event(app_handle: &AppHandle, event: AppEvent) {
    if let Err(e) = app_handle.emit("crumusix-event", event) {
        println!("Failed to emit global Tauri IPC event: {}", e);
    }
}

pub fn emit_state_sync(app_handle: &AppHandle) {
    let state = get_global_playback_state();
    emit_global_event(app_handle, AppEvent::PlaybackStateChanged(state));
}
