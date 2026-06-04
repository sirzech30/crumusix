use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use crate::Track;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppSpotifySettings {
    pub normalisation: bool,
    pub cache_enabled: bool,
    pub gapless: bool,
    pub mpris_enabled: bool,
    pub cache_size_mb: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppPerformanceSettings {
    pub visualizer_enabled: bool,
    pub premium_graphics: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub active_screen: String,
    pub volume: u32,
    pub is_muted: bool,
    pub spotify_access_token: Option<String>,
    pub spotify_refresh_token: Option<String>,
    pub spotify_token_expiry: u64,
    pub spotify_client_id: String,
    pub spotify_settings: AppSpotifySettings,
    pub performance_settings: AppPerformanceSettings,
    pub recent_tracks: Vec<Track>,
    pub show_video: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            active_screen: "home".to_string(),
            volume: 80,
            is_muted: false,
            spotify_access_token: None,
            spotify_refresh_token: None,
            spotify_token_expiry: 0,
            spotify_client_id: "d08327e1f5a14ea59b1feda104f5c255".to_string(),
            spotify_settings: AppSpotifySettings {
                normalisation: true,
                cache_enabled: true,
                gapless: true,
                mpris_enabled: true,
                cache_size_mb: 2000,
            },
            performance_settings: AppPerformanceSettings {
                visualizer_enabled: false, // Default false for slate professional theme
                premium_graphics: true,
            },
            recent_tracks: Vec::new(),
            show_video: true,
        }
    }
}

pub struct SharedAppConfig(pub Mutex<AppConfig>);

fn get_config_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let mut path = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to resolve config directory: {}", e))?;
    
    fs::create_dir_all(&path).ok();
    path.push("config.json");
    Ok(path)
}

#[tauri::command]
pub async fn get_app_config(
    app_handle: AppHandle,
    state: State<'_, SharedAppConfig>,
) -> Result<AppConfig, String> {
    let path = get_config_path(&app_handle)?;
    if !path.exists() {
        // Save the default config initially
        let default_config = AppConfig::default();
        let serialized = serde_json::to_string_pretty(&default_config)
            .map_err(|e| format!("Failed to serialize default config: {}", e))?;
        fs::write(&path, serialized)
            .map_err(|e| format!("Failed to write config file: {}", e))?;
        
        let mut inner = state.0.lock().map_err(|e| format!("Failed to lock config state: {}", e))?;
        *inner = default_config.clone();
        return Ok(default_config);
    }

    let file_content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;
    let config: AppConfig = serde_json::from_str(&file_content)
        .unwrap_or_else(|_| {
            // Return default if parsing fails
            AppConfig::default()
        });

    let mut inner = state.0.lock().map_err(|e| format!("Failed to lock config state: {}", e))?;
    *inner = config.clone();
    
    Ok(config)
}

#[tauri::command]
pub fn save_app_config(
    app_handle: AppHandle,
    config: AppConfig,
    state: State<'_, SharedAppConfig>,
) -> Result<(), String> {
    let path = get_config_path(&app_handle)?;
    let serialized = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    
    fs::write(path, serialized)
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    let mut inner = state.0.lock().map_err(|e| format!("Failed to lock config state: {}", e))?;
    *inner = config;

    Ok(())
}
