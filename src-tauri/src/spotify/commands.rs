use tauri::State;
use crate::spotify::worker::{PlaybackCommand, BackendSettings};
use crate::spotify::state::PlaybackState;
use crate::spotify::state::SharedPlaybackState;

pub struct SpotifyCommandSender {
    pub tx: tokio::sync::mpsc::Sender<PlaybackCommand>,
}

#[tauri::command]
pub async fn spotify_init_native(
    token: String,
    settings: BackendSettings,
    sender: State<'_, SpotifyCommandSender>,
) -> Result<(), String> {
    sender.tx.send(PlaybackCommand::Init(token, settings))
        .await
        .map_err(|e| format!("Failed to send Init command: {}", e))
}

#[tauri::command]
pub async fn spotify_play(
    track_id: String,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    duration_ms: Option<u32>,
    thumbnail: Option<String>,
    sender: State<'_, SpotifyCommandSender>,
) -> Result<(), String> {
    sender.tx.send(PlaybackCommand::Play {
        track_id,
        title: title.unwrap_or_else(|| "Spotify Track".to_string()),
        artist: artist.unwrap_or_else(|| "Artist".to_string()),
        album: album.unwrap_or_else(|| "".to_string()),
        duration_ms: duration_ms.unwrap_or(0),
        thumbnail: thumbnail.unwrap_or_else(|| "".to_string()),
    })
    .await
    .map_err(|e| format!("Failed to send Play command: {}", e))
}

#[tauri::command]
pub async fn spotify_pause(
    sender: State<'_, SpotifyCommandSender>,
) -> Result<(), String> {
    sender.tx.send(PlaybackCommand::Pause)
        .await
        .map_err(|e| format!("Failed to send Pause command: {}", e))
}

#[tauri::command]
pub async fn spotify_resume(
    sender: State<'_, SpotifyCommandSender>,
) -> Result<(), String> {
    sender.tx.send(PlaybackCommand::Resume)
        .await
        .map_err(|e| format!("Failed to send Resume command: {}", e))
}

#[tauri::command]
pub async fn spotify_seek(
    position_ms: u32,
    sender: State<'_, SpotifyCommandSender>,
) -> Result<(), String> {
    sender.tx.send(PlaybackCommand::Seek(position_ms))
        .await
        .map_err(|e| format!("Failed to send Seek command: {}", e))
}

#[tauri::command]
pub async fn spotify_stop(
    sender: State<'_, SpotifyCommandSender>,
) -> Result<(), String> {
    sender.tx.send(PlaybackCommand::Stop)
        .await
        .map_err(|e| format!("Failed to send Stop command: {}", e))
}

#[tauri::command]
pub async fn spotify_volume(
    volume: f32,
    sender: State<'_, SpotifyCommandSender>,
) -> Result<(), String> {
    sender.tx.send(PlaybackCommand::Volume(volume))
        .await
        .map_err(|e| format!("Failed to send Volume command: {}", e))
}

#[tauri::command]
pub async fn spotify_update_settings(
    settings: BackendSettings,
    sender: State<'_, SpotifyCommandSender>,
) -> Result<(), String> {
    sender.tx.send(PlaybackCommand::UpdateSettings(settings))
        .await
        .map_err(|e| format!("Failed to send UpdateSettings command: {}", e))
}

#[tauri::command]
pub async fn spotify_preload(
    track_id: String,
    sender: State<'_, SpotifyCommandSender>,
) -> Result<(), String> {
    sender.tx.send(PlaybackCommand::Preload(track_id))
        .await
        .map_err(|e| format!("Failed to send Preload command: {}", e))
}

#[tauri::command]
pub fn spotify_get_state(
    state: State<'_, SharedPlaybackState>,
) -> PlaybackState {
    state.read()
}
