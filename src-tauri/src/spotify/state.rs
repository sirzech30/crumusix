use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TrackInfo {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u32,
    pub thumbnail: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PlaybackState {
    pub current_track: Option<TrackInfo>,
    pub position_ms: u32,
    pub is_playing: bool,
    pub volume: f32,
    pub is_connected: bool,
    pub device_id: Option<String>,
    pub is_buffering: bool,
}

#[derive(Clone)]
pub struct SharedPlaybackState {
    inner: Arc<RwLock<PlaybackState>>,
}

impl SharedPlaybackState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(PlaybackState {
                volume: 0.8, // Default 80% volume
                ..Default::default()
            })),
        }
    }

    pub fn read(&self) -> PlaybackState {
        self.inner.read().clone()
    }

    pub fn write<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut PlaybackState) -> R,
    {
        let mut lock = self.inner.write();
        f(&mut lock)
    }
}
