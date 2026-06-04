use serde::{Serialize, Deserialize};
use async_trait::async_trait;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProviderCapabilities {
    pub can_seek: bool,
    pub supports_video: bool,
    pub supports_lyrics: bool,
    pub supports_queue: bool,
    pub supports_metadata: bool,
}

#[async_trait]
pub trait PlaybackProvider: Send + Sync {
    async fn play(&self, track_id: &str) -> Result<(), String>;
    async fn pause(&self) -> Result<(), String>;
    async fn stop(&self) -> Result<(), String>;
    async fn seek(&self, position_ms: u64) -> Result<(), String>;
    async fn set_volume(&self, volume: f32) -> Result<(), String>;
    fn get_capabilities(&self) -> ProviderCapabilities;
}
