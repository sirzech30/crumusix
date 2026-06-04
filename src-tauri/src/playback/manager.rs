use crate::playback::provider::{PlaybackProvider, ProviderCapabilities};
use crate::playback::state::{update_global_playback_state, get_global_playback_state, PlaybackState};
use std::sync::Arc;
use std::sync::OnceLock;
use parking_lot::Mutex;

pub struct PlaybackManager {
    // Dynamic provider references registered on startup
    spotify_provider: OnceLock<Arc<dyn PlaybackProvider>>,
    youtube_provider: OnceLock<Arc<dyn PlaybackProvider>>,
    local_provider: OnceLock<Arc<dyn PlaybackProvider>>,
    active_provider_name: Mutex<Option<String>>,
}

impl PlaybackManager {
    pub fn global() -> &'static Self {
        static INSTANCE: OnceLock<PlaybackManager> = OnceLock::new();
        INSTANCE.get_or_init(|| Self {
            spotify_provider: OnceLock::new(),
            youtube_provider: OnceLock::new(),
            local_provider: OnceLock::new(),
            active_provider_name: Mutex::new(None),
        })
    }

    pub fn register_spotify_provider(&self, provider: Arc<dyn PlaybackProvider>) {
        let _ = self.spotify_provider.set(provider);
    }

    pub fn register_youtube_provider(&self, provider: Arc<dyn PlaybackProvider>) {
        let _ = self.youtube_provider.set(provider);
    }

    pub fn register_local_provider(&self, provider: Arc<dyn PlaybackProvider>) {
        let _ = self.local_provider.set(provider);
    }

    fn get_active_provider(&self) -> Option<Arc<dyn PlaybackProvider>> {
        let name = self.active_provider_name.lock().clone()?;
        match name.as_str() {
            "spotify" => self.spotify_provider.get().cloned(),
            "youtube" => self.youtube_provider.get().cloned(),
            "local" => self.local_provider.get().cloned(),
            _ => None,
        }
    }

    pub fn set_active_provider(&self, name: &str) -> Result<ProviderCapabilities, String> {
        let provider = match name {
            "spotify" => self.spotify_provider.get().cloned(),
            "youtube" => self.youtube_provider.get().cloned(),
            "local" => self.local_provider.get().cloned(),
            _ => return Err(format!("Unknown playback provider: {}", name)),
        };

        if let Some(p) = provider {
            let caps = p.get_capabilities();
            *self.active_provider_name.lock() = Some(name.to_string());
            update_global_playback_state(|state| {
                state.active_provider = Some(name.to_string());
            });
            Ok(caps)
        } else {
            Err(format!("Provider {} has not been registered", name))
        }
    }

    pub async fn play(&self, track_id: &str) -> Result<(), String> {
        if let Some(provider) = self.get_active_provider() {
            provider.play(track_id).await?;
            update_global_playback_state(|state| {
                state.is_playing = true;
                state.current_track_id = Some(track_id.to_string());
            });
            Ok(())
        } else {
            Err("No active playback provider selected".to_string())
        }
    }

    pub async fn pause(&self) -> Result<(), String> {
        if let Some(provider) = self.get_active_provider() {
            provider.pause().await?;
            update_global_playback_state(|state| {
                state.is_playing = false;
            });
            Ok(())
        } else {
            Err("No active playback provider selected".to_string())
        }
    }

    pub async fn stop(&self) -> Result<(), String> {
        if let Some(provider) = self.get_active_provider() {
            provider.stop().await?;
            update_global_playback_state(|state| {
                state.is_playing = false;
                state.current_track_id = None;
            });
            Ok(())
        } else {
            Err("No active playback provider selected".to_string())
        }
    }

    pub async fn seek(&self, position_ms: u64) -> Result<(), String> {
        if let Some(provider) = self.get_active_provider() {
            provider.seek(position_ms).await?;
            update_global_playback_state(|state| {
                state.position_ms = position_ms;
            });
            Ok(())
        } else {
            Err("No active playback provider selected".to_string())
        }
    }

    pub async fn set_volume(&self, volume: f32) -> Result<(), String> {
        if let Some(provider) = self.get_active_provider() {
            provider.set_volume(volume).await?;
            update_global_playback_state(|state| {
                state.volume = volume;
            });
            Ok(())
        } else {
            Err("No active playback provider selected".to_string())
        }
    }

    pub fn get_state(&self) -> PlaybackState {
        get_global_playback_state()
    }
}
