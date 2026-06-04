use tauri::{AppHandle, Emitter};
use serde::Serialize;
use crate::spotify::state::PlaybackState;

#[derive(Serialize, Clone, Debug)]
#[serde(tag = "type", content = "data")]
pub enum SpotifyEvent {
    StateChanged(PlaybackState),
    Timeline(u32), // position_ms
    NextTrack,
    PrevTrack,
    EndOfTrack,
    TimeToPreloadNextTrack,
}

pub fn emit_spotify_event(app_handle: &AppHandle, event: SpotifyEvent) {
    app_handle.emit("spotify-playback-event", event).ok();
}
