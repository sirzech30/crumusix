use std::sync::Arc;
use tokio::sync::mpsc;
use tauri::{AppHandle, Manager};
use librespot::core::session::Session;
use librespot::core::config::SessionConfig;
use librespot::core::authentication::Credentials;
use librespot::core::spotify_id::SpotifyId;
use librespot::core::SpotifyUri;
use librespot::playback::config::{PlayerConfig, AudioFormat};
use librespot::playback::player::{Player, PlayerEvent};
use librespot::playback::mixer::VolumeGetter;
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, PlatformConfig};
use crate::{log_info, log_warn, log_error};

#[derive(Clone)]
pub struct SharedVolume {
    volume: Arc<std::sync::atomic::AtomicU16>,
}

impl SharedVolume {
    pub fn new(initial_volume: u16) -> Self {
        Self {
            volume: Arc::new(std::sync::atomic::AtomicU16::new(initial_volume)),
        }
    }

    pub fn set(&self, val: u16) {
        self.volume.store(val, std::sync::atomic::Ordering::Relaxed);
    }
}

impl VolumeGetter for SharedVolume {
    fn attenuation_factor(&self) -> f64 {
        let vol = self.volume.load(std::sync::atomic::Ordering::Relaxed) as f64 / 65535.0;
        // Cubic volume mapping for natural hearing curve
        vol * vol * vol
    }
}

use crate::spotify::state::{SharedPlaybackState, TrackInfo};
use crate::spotify::auth::{get_spotify_cache_path, get_cached_credentials, create_credentials_from_token};
use crate::spotify::events::{emit_spotify_event, SpotifyEvent};

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct BackendSettings {
    pub normalisation: bool,
    pub cache_enabled: bool,
    pub gapless: bool,
    pub mpris_enabled: bool,
    pub cache_size_mb: u32,
}

impl Default for BackendSettings {
    fn default() -> Self {
        Self {
            normalisation: true,
            cache_enabled: true,
            gapless: true,
            mpris_enabled: true,
            cache_size_mb: 2000, // 2GB default
        }
    }
}

#[derive(Debug)]
pub enum PlaybackCommand {
    Init(String, BackendSettings), // OAuth Access Token & Settings
    Play {
        track_id: String,
        title: String,
        artist: String,
        album: String,
        duration_ms: u32,
        thumbnail: String,
    },
    Pause,
    Resume,
    Seek(u32), // position in milliseconds
    Volume(f32), // volume from 0.0 to 1.0
    Stop,
    UpdateSettings(BackendSettings),
    Preload(String),
    TogglePlay,
    Next,
    Previous,
    SyncMpris,
}

pub struct PlaybackWorker {
    app_handle: AppHandle,
    state: SharedPlaybackState,
    receiver: mpsc::Receiver<PlaybackCommand>,
    session: Option<Session>,
    player: Option<Arc<Player>>,
    shared_volume: SharedVolume,
    settings: BackendSettings,
    mpris_controls: Option<MediaControls>,
    receiver_sender_clone: mpsc::Sender<PlaybackCommand>,
}

impl PlaybackWorker {
    pub fn spawn(
        app_handle: AppHandle,
        state: SharedPlaybackState,
        receiver: mpsc::Receiver<PlaybackCommand>,
        receiver_sender_clone: mpsc::Sender<PlaybackCommand>,
    ) {
        // Initial volume matches frontend default of 80%
        let initial_vol_u16 = (0.8 * 65535.0) as u16;
        let shared_volume = SharedVolume::new(initial_vol_u16);

        let mut worker = Self {
            app_handle,
            state,
            receiver,
            session: None,
            player: None,
            shared_volume,
            settings: BackendSettings::default(),
            mpris_controls: None,
            receiver_sender_clone,
        };
        
        tauri::async_runtime::spawn(async move {
            worker.run().await;
        });
    }

    async fn run(&mut self) {
        // Check if cached credentials exist and auto-initialize
        if let Some(credentials) = get_cached_credentials(&self.app_handle) {
            log_info!("[Spotify Worker] Found cached Spotify credentials. Auto-initializing native session...");
            self.connect_session(credentials).await;
        }

        // Main command listener loop
        while let Some(command) = self.receiver.recv().await {
            match command {
                PlaybackCommand::Init(token, settings) => {
                    log_info!("[Spotify Worker] Initializing native Spotify session from OAuth token with settings: {:?}", settings);
                    self.settings = settings;
                    let credentials = create_credentials_from_token(&token);
                    self.connect_session(credentials).await;
                }
                PlaybackCommand::Play { track_id, title, artist, album, duration_ms, thumbnail } => {
                    self.play_track(&track_id, title, artist, album, duration_ms, thumbnail).await;
                }
                PlaybackCommand::Pause => {
                    if let Some(ref player) = self.player {
                        player.pause();
                        self.state.write(|s| s.is_playing = false);
                        emit_spotify_event(&self.app_handle, SpotifyEvent::StateChanged(self.state.read()));
                        self.sync_mpris_state();
                    }
                }
                PlaybackCommand::Resume => {
                    if let Some(ref player) = self.player {
                        player.play();
                        self.state.write(|s| s.is_playing = true);
                        emit_spotify_event(&self.app_handle, SpotifyEvent::StateChanged(self.state.read()));
                        self.sync_mpris_state();
                    }
                }
                PlaybackCommand::Seek(position_ms) => {
                    if let Some(ref player) = self.player {
                        player.seek(position_ms);
                        self.state.write(|s| s.position_ms = position_ms);
                        emit_spotify_event(&self.app_handle, SpotifyEvent::StateChanged(self.state.read()));
                        self.sync_mpris_state();
                    }
                }
                PlaybackCommand::Volume(volume) => {
                    self.shared_volume.set((volume * 65535.0) as u16);
                    self.state.write(|s| s.volume = volume);
                    emit_spotify_event(&self.app_handle, SpotifyEvent::StateChanged(self.state.read()));
                }
                PlaybackCommand::Stop => {
                    if let Some(ref player) = self.player {
                        player.stop();
                        self.state.write(|s| {
                            s.is_playing = false;
                            s.current_track = None;
                            s.position_ms = 0;
                        });
                        emit_spotify_event(&self.app_handle, SpotifyEvent::StateChanged(self.state.read()));
                        self.sync_mpris_state();
                    }
                }
                PlaybackCommand::UpdateSettings(new_settings) => {
                    log_info!("[Spotify Worker] Updating backend Spotify settings dynamically: {:?}", new_settings);
                    self.settings = new_settings;
                    
                    // Update MPRIS integration immediately
                    self.update_mpris_integration();
                }
                PlaybackCommand::Preload(track_id_str) => {
                    self.preload_track(&track_id_str).await;
                }
                PlaybackCommand::TogglePlay => {
                    if let Some(ref player) = self.player {
                        let is_playing = self.state.read().is_playing;
                        if is_playing {
                            player.pause();
                            self.state.write(|s| s.is_playing = false);
                        } else {
                            player.play();
                            self.state.write(|s| s.is_playing = true);
                        }
                        emit_spotify_event(&self.app_handle, SpotifyEvent::StateChanged(self.state.read()));
                        self.sync_mpris_state();
                    }
                }
                PlaybackCommand::Next => {
                    emit_spotify_event(&self.app_handle, SpotifyEvent::NextTrack);
                }
                PlaybackCommand::Previous => {
                    emit_spotify_event(&self.app_handle, SpotifyEvent::PrevTrack);
                }
                PlaybackCommand::SyncMpris => {
                    self.sync_mpris_state();
                }
            }
        }
    }

    async fn connect_session(&mut self, credentials: Credentials) {
        // Disconnect old session/player
        if let Some(ref player) = self.player {
            player.stop();
        }
        self.player = None;
        self.session = None;

        let session_config = SessionConfig::default();
        
        // Setup cache directory based on settings
        let cache_path = get_spotify_cache_path(&self.app_handle);
        std::fs::create_dir_all(&cache_path).ok();
        
        let audio_cache_dir = if self.settings.cache_enabled {
            Some(cache_path.join("audio"))
        } else {
            None
        };
        
        let max_cache_bytes = if self.settings.cache_enabled && self.settings.cache_size_mb > 0 {
            Some((self.settings.cache_size_mb as u64) * 1024 * 1024)
        } else {
            None
        };

        let cache = librespot::core::cache::Cache::new(
            Some(cache_path),
            None,
            audio_cache_dir,
            max_cache_bytes,
        ).ok();

        let session = Session::new(session_config, cache);
        match session.connect(credentials, true).await {
            Ok(()) => {
                let dev_id = session.device_id().to_string();
                log_info!("[Spotify Worker] Successfully connected native Spotify session. Device ID: {}", dev_id);
                
                self.session = Some(session.clone());
                
                // Initialize player with settings-configured PlayerConfig and rodio backend
                let mut player_config = PlayerConfig::default();
                player_config.position_update_interval = Some(std::time::Duration::from_millis(500));
                player_config.normalisation = self.settings.normalisation;
                
                // Try to find an audio backend; if none is available, log an error and bail
                let backend_fn = match librespot::playback::audio_backend::find(None) {
                    Some(b) => b,
                    None => {
                        log_error!("[Spotify Worker] No native Spotify audio backend found on this system. Cannot initialize player.");
                        self.state.write(|s| {
                            s.is_connected = false;
                            s.device_id = None;
                        });
                        emit_spotify_event(&self.app_handle, SpotifyEvent::StateChanged(self.state.read()));
                        return;
                    }
                };

                let player = Player::new(
                    player_config,
                    session,
                    Box::new(self.shared_volume.clone()),
                    move || (backend_fn)(None, AudioFormat::default())
                );
                let mut events = player.get_player_event_channel();

                self.player = Some(player);

                // Update state
                self.state.write(|s| {
                    s.is_connected = true;
                    s.device_id = Some(dev_id.clone());
                });

                emit_spotify_event(&self.app_handle, SpotifyEvent::StateChanged(self.state.read()));

                // Setup system hardware media control keys (MPRIS)
                self.update_mpris_integration();

                // Spawn background event listener for player events!
                let app_handle_clone = self.app_handle.clone();
                let state_clone = self.state.clone();
                let tx_clone = self.receiver_sender_clone.clone();
                tauri::async_runtime::spawn(async move {
                    while let Some(event) = events.recv().await {
                        match event {
                            PlayerEvent::Playing { position_ms, .. } => {
                                state_clone.write(|s| {
                                    s.is_playing = true;
                                    s.position_ms = position_ms;
                                    s.is_buffering = false;
                                });
                                emit_spotify_event(&app_handle_clone, SpotifyEvent::StateChanged(state_clone.read()));
                                tx_clone.try_send(PlaybackCommand::SyncMpris).ok();
                            }
                            PlayerEvent::Paused { position_ms, .. } => {
                                state_clone.write(|s| {
                                    s.is_playing = false;
                                    s.position_ms = position_ms;
                                });
                                emit_spotify_event(&app_handle_clone, SpotifyEvent::StateChanged(state_clone.read()));
                                tx_clone.try_send(PlaybackCommand::SyncMpris).ok();
                            }
                            PlayerEvent::Stopped { .. } => {
                                state_clone.write(|s| {
                                    s.is_playing = false;
                                    s.position_ms = 0;
                                });
                                emit_spotify_event(&app_handle_clone, SpotifyEvent::StateChanged(state_clone.read()));
                                tx_clone.try_send(PlaybackCommand::SyncMpris).ok();
                            }
                            PlayerEvent::EndOfTrack { .. } => {
                                emit_spotify_event(&app_handle_clone, SpotifyEvent::EndOfTrack);
                                tx_clone.try_send(PlaybackCommand::SyncMpris).ok();
                            }
                            PlayerEvent::TimeToPreloadNextTrack { .. } => {
                                emit_spotify_event(&app_handle_clone, SpotifyEvent::TimeToPreloadNextTrack);
                                
                                // Native preloading optimization!
                                if let Some(queue_state) = app_handle_clone.try_state::<crate::queue::SharedQueueState>() {
                                    let queue = queue_state.inner.read();
                                    let next_idx = queue.current_index + 1;
                                    if next_idx >= 0 && next_idx < queue.items.len() as i32 {
                                        let next_item = &queue.items[next_idx as usize];
                                        if next_item.source == "spotify" {
                                            let _ = tx_clone.try_send(PlaybackCommand::Preload(next_item.track_id.clone()));
                                        }
                                    }
                                }
                            }
                            PlayerEvent::Loading { .. } => {
                                state_clone.write(|s| s.is_buffering = true);
                                emit_spotify_event(&app_handle_clone, SpotifyEvent::StateChanged(state_clone.read()));
                            }
                            PlayerEvent::PositionChanged { position_ms, .. }
                            | PlayerEvent::PositionCorrection { position_ms, .. }
                            | PlayerEvent::Seeked { position_ms, .. } => {
                                state_clone.write(|s| {
                                    s.position_ms = position_ms;
                                });
                                emit_spotify_event(&app_handle_clone, SpotifyEvent::StateChanged(state_clone.read()));
                                tx_clone.try_send(PlaybackCommand::SyncMpris).ok();
                            }
                            _ => {}
                        }
                    }
                });
            }
                    Err(err) => {
                log_error!("[Spotify Worker] Failed to connect native Spotify session: {}", err);
                self.state.write(|s| {
                    s.is_connected = false;
                    s.device_id = None;
                });
                emit_spotify_event(&self.app_handle, SpotifyEvent::StateChanged(self.state.read()));
            }
        }
    }

    async fn play_track(
        &mut self,
        track_id_str: &str,
        title: String,
        artist: String,
        album: String,
        duration_ms: u32,
        thumbnail: String,
    ) {
        if self.session.is_none() {
            // Try to auto connect first
            if let Some(credentials) = get_cached_credentials(&self.app_handle) {
                self.connect_session(credentials).await;
            }
        }

        let player = match self.player {
            Some(ref p) => p,
            None => {
                log_warn!("[Spotify Worker] Cannot play track: Spotify session is not connected!");
                return;
            }
        };

        if let Ok(track_id) = SpotifyId::from_base62(track_id_str) {
            log_info!("[Spotify Worker] Loading track base62 id natively: {}", track_id_str);
            player.load(SpotifyUri::Track { id: track_id }, true, 0);
            
            // Set current track state
            self.state.write(|s| {
                s.current_track = Some(TrackInfo {
                    id: track_id_str.to_string(),
                    title,
                    artist,
                    album,
                    duration_ms,
                    thumbnail,
                });
                s.is_playing = true;
                s.position_ms = 0;
            });

            emit_spotify_event(&self.app_handle, SpotifyEvent::StateChanged(self.state.read()));
            self.sync_mpris_state();
        } else {
            log_warn!("[Spotify Worker] Invalid Spotify base62 track ID: {}", track_id_str);
        }
    }

    async fn preload_track(&mut self, track_id_str: &str) {
        if !self.settings.gapless {
            return;
        }
        let player = match self.player {
            Some(ref p) => p,
            None => return,
        };
        if let Ok(track_id) = SpotifyId::from_base62(track_id_str) {
            log_info!("[Spotify Worker] Preloading track base62 id natively: {}", track_id_str);
            player.preload(SpotifyUri::Track { id: track_id });
        }
    }

    fn update_mpris_integration(&mut self) {
        if !self.settings.mpris_enabled {
            self.mpris_controls = None;
            return;
        }

        if self.mpris_controls.is_some() {
            self.sync_mpris_state();
            return;
        }

        #[cfg(not(target_os = "windows"))]
        let hwnd = None;
        #[cfg(target_os = "windows")]
        let hwnd = None;

        let config = PlatformConfig {
            dbus_name: "crumusix",
            display_name: "CrumusiX",
            hwnd,
        };

        if let Ok(mut controls) = MediaControls::new(config) {
            let tx_clone = self.receiver_sender_clone.clone();
            
            let attach_res = controls.attach(move |event| {
                let tx = tx_clone.clone();
                tokio::spawn(async move {
                    match event {
                        MediaControlEvent::Play => {
                            tx.send(PlaybackCommand::Resume).await.ok();
                        }
                        MediaControlEvent::Pause => {
                            tx.send(PlaybackCommand::Pause).await.ok();
                        }
                        MediaControlEvent::Toggle => {
                            tx.send(PlaybackCommand::TogglePlay).await.ok();
                        }
                        MediaControlEvent::Stop => {
                            tx.send(PlaybackCommand::Stop).await.ok();
                        }
                        MediaControlEvent::Next => {
                            tx.send(PlaybackCommand::Next).await.ok();
                        }
                        MediaControlEvent::Previous => {
                            tx.send(PlaybackCommand::Previous).await.ok();
                        }
                        _ => {}
                    }
                });
            });

            if attach_res.is_ok() {
                self.mpris_controls = Some(controls);
                self.sync_mpris_state();
                log_info!("[Spotify Worker] Successfully registered system media control keys (MPRIS)!");
            } else {
                log_warn!("[Spotify Worker] Failed to attach media controls callback");
            }
        } else {
            log_warn!("[Spotify Worker] Failed to initialize media controls");
        }
    }

    fn sync_mpris_state(&mut self) {
        let controls = match self.mpris_controls.as_mut() {
            Some(c) => c,
            None => return,
        };

        let state = self.state.read();
        
        let playback_status = if state.is_playing {
            souvlaki::MediaPlayback::Playing {
                progress: Some(souvlaki::MediaPosition(std::time::Duration::from_millis(state.position_ms as u64))),
            }
        } else if state.current_track.is_some() {
            souvlaki::MediaPlayback::Paused {
                progress: Some(souvlaki::MediaPosition(std::time::Duration::from_millis(state.position_ms as u64))),
            }
        } else {
            souvlaki::MediaPlayback::Stopped
        };
        controls.set_playback(playback_status).ok();

        if let Some(ref track) = state.current_track {
            let metadata = MediaMetadata {
                title: Some(&track.title),
                artist: Some(&track.artist),
                album: Some(&track.album),
                duration: Some(std::time::Duration::from_millis(track.duration_ms as u64)),
                cover_url: if track.thumbnail.is_empty() { None } else { Some(&track.thumbnail) },
                ..Default::default()
            };
            controls.set_metadata(metadata).ok();
        } else {
            controls.set_metadata(MediaMetadata::default()).ok();
        }
    }
}
