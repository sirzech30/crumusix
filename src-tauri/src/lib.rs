use tauri::Manager;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
pub mod spotify;
pub mod cache;
pub mod queue;
pub mod lyrics;
pub mod playback;
pub mod events;
pub mod session;
pub mod recovery;
pub mod local;
pub mod diagnostics;
pub mod stats;
pub mod backup;
pub mod config;


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct YouTubeSearchResult {
    pub id: String,
    pub title: String,
    pub thumbnail: String,
    pub duration: String,
    pub channel: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: String,
    pub source: String, // "spotify" or "youtube"
    pub thumbnail: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Playlist {
    pub name: String,
    pub tracks: Vec<Track>,
}

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

// Bypasses CORS and searches YouTube for audio streams by scraping ytInitialData from the search page
#[tauri::command]
async fn search_youtube(query: String) -> Result<Vec<YouTubeSearchResult>, String> {
    use crate::playback::video_provider::{VideoProvider, YouTubeProvider};
    let provider = YouTubeProvider;
    let results = provider.search(&query).await?;
    
    Ok(results.into_iter().map(|r| YouTubeSearchResult {
        id: r.id,
        title: r.title,
        thumbnail: r.thumbnail,
        duration: r.duration,
        channel: r.channel,
    }).collect())
}

// Runs a temporary loopback HTTP server on port 8888 to capture the Spotify OAuth authorization callback code
use std::sync::atomic::{AtomicBool, Ordering};
static OAUTH_SERVER_RUNNING: AtomicBool = AtomicBool::new(false);

#[tauri::command]
async fn start_oauth_server(app_handle: tauri::AppHandle) -> Result<String, String> {
    if OAUTH_SERVER_RUNNING.swap(true, Ordering::SeqCst) {
        return Err("OAuth authentication is already in progress. Please complete the login in your open browser window.".to_string());
    }

    let result = start_oauth_server_internal().await;
    
    OAUTH_SERVER_RUNNING.store(false, Ordering::SeqCst);
    result
}

async fn start_oauth_server_internal() -> Result<String, String> {
    let listener = TcpListener::bind("127.0.0.1:8888")
        .await
        .map_err(|e| format!("Failed to bind TCP listener to port 8888: {}. Make sure no other service is using port 8888.", e))?;
        
    let timeout_duration = std::time::Duration::from_secs(300);
    let start_time = std::time::Instant::now();
    
    loop {
        let elapsed = start_time.elapsed();
        if elapsed >= timeout_duration {
            return Err("OAuth connection timed out after 5 minutes".to_string());
        }
        let remaining = timeout_duration - elapsed;
        
        // Wait for incoming connections with timeout
        let accept_result = tokio::time::timeout(remaining, listener.accept()).await;
        
        match accept_result {
            Ok(Ok((mut stream, _))) => {
                let mut buffer = [0; 2048];
                // Read the request headers/body with a short timeout to prevent slow-loris lockups
                let read_timeout = std::time::Duration::from_secs(5);
                match tokio::time::timeout(read_timeout, stream.read(&mut buffer)).await {
                    Ok(Ok(size)) if size > 0 => {
                        let request = String::from_utf8_lossy(&buffer[..size]);
                        
                        let mut is_valid_auth = false;
                        let mut captured_code = None;
                        
                        if let Some(first_line) = request.lines().next() {
                            if first_line.starts_with("GET ") && first_line.contains("code=") {
                                if let Some(code_idx) = first_line.find("code=") {
                                    let code_start = code_idx + 5;
                                    let remaining = &first_line[code_start..];
                                    let code_end = remaining.find(' ').unwrap_or(remaining.len());
                                    let raw_code = &remaining[..code_end];
                                    let code = raw_code.split('&').next().unwrap_or(raw_code).to_string();
                                    captured_code = Some(code);
                                    is_valid_auth = true;
                                }
                            }
                        }

                        // Check if this is the actual Spotify redirect query containing "code="
                        if is_valid_auth {
                            let code = captured_code.unwrap();
                            
                            // Return a beautiful visual confirmation page in the user's browser
                            let response_body = r#"
                            <!DOCTYPE html>
                            <html>
                            <head>
                                <title>CrumusiX Authentication Success</title>
                                <style>
                                    body {
                                        font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
                                        background-color: #0b0c16;
                                        color: #f3f4f6;
                                        text-align: center;
                                        padding-top: 100px;
                                        margin: 0;
                                    }
                                    .container {
                                        max-width: 500px;
                                        margin: 0 auto;
                                        padding: 40px;
                                        background: rgba(18, 20, 38, 0.6);
                                        border: 1px solid rgba(255, 255, 255, 0.08);
                                        border-radius: 16px;
                                        box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
                                    }
                                    h1 { color: #1DB954; font-size: 2.2rem; margin-bottom: 16px; }
                                    p { color: #9ca3af; font-size: 1.1rem; line-height: 1.6; }
                                    .badge {
                                        display: inline-block;
                                        padding: 6px 12px;
                                        background: rgba(29, 185, 84, 0.15);
                                        color: #1DB954;
                                        border: 1px solid #1DB954;
                                        border-radius: 20px;
                                        font-weight: bold;
                                        text-transform: uppercase;
                                        font-size: 0.85rem;
                                        margin-bottom: 24px;
                                    }
                                </style>
                            </head>
                            <body>
                                <div class="container">
                                    <div class="badge">Connection Successful</div>
                                    <h1>CrumusiX Desktop Linked</h1>
                                    <p>Your Spotify Premium account has been successfully linked to the player. You may safely close this tab and return to the application!</p>
                                </div>
                                <script>
                                    setTimeout(() => {
                                        window.close();
                                    }, 1000);
                                </script>
                            </body>
                            </html>
                            "#;
                            
                            let response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n{}",
                                response_body.len(),
                                response_body
                            );
                            
                            stream.write_all(response.as_bytes()).await.ok();
                            stream.flush().await.ok();
                            
                            return Ok(code);
                        } else {
                            // Probe or favicon check: respond with 400 Bad Request, close socket, and loop to wait for the real auth request
                            let err_response = "HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nNot a Spotify auth callback redirect.";
                            stream.write_all(err_response.as_bytes()).await.ok();
                            stream.flush().await.ok();
                        }
                    }
                    _ => {}
                }
            }
            Ok(Err(_)) => {
                // Short sleep before next accept attempt to prevent tight-loop CPU pegging
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
            Err(_) => {
                return Err("OAuth connection timed out after 5 minutes".to_string());
            }
        }
    }
}

// Retrieves the local AppData directory path for storing playlists
fn get_playlists_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let mut path = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to resolve app config directory: {}", e))?;
    
    // Ensure the folder exists
    if !path.exists() {
        fs::create_dir_all(&path).map_err(|e| format!("Failed to create AppConfig dir: {}", e))?;
    }
    
    path.push("playlists.json");
    Ok(path)
}

// Reads all user playlists from the local JSON database file
#[tauri::command]
fn get_playlists(app_handle: tauri::AppHandle) -> Result<Vec<Playlist>, String> {
    let path = get_playlists_path(&app_handle)?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file_content = fs::read_to_string(path).map_err(|e| format!("Failed to read playlists file: {}", e))?;
    let playlists: Vec<Playlist> = serde_json::from_str(&file_content)
        .map_err(|e| format!("Failed to parse playlists JSON: {}", e))?;
    Ok(playlists)
}

// Saves/Updates a user playlist locally in the JSON file database
#[tauri::command]
fn save_playlist(app_handle: tauri::AppHandle, playlist: Playlist) -> Result<(), String> {
    let path = get_playlists_path(&app_handle)?;
    let mut playlists = get_playlists(app_handle.clone()).unwrap_or_else(|_| Vec::new());

    // Update existing playlist or create a new one
    if let Some(pos) = playlists.iter().position(|p| p.name == playlist.name) {
        playlists[pos] = playlist;
    } else {
        playlists.push(playlist);
    }

    let serialized = serde_json::to_string_pretty(&playlists)
        .map_err(|e| format!("Failed to serialize playlists: {}", e))?;
    fs::write(path, serialized).map_err(|e| format!("Failed to write playlists file: {}", e))?;
    Ok(())
}

// Deletes a user playlist by name from the local database
#[tauri::command]
fn delete_playlist(app_handle: tauri::AppHandle, name: String) -> Result<(), String> {
    let path = get_playlists_path(&app_handle)?;
    let mut playlists = get_playlists(app_handle.clone()).unwrap_or_else(|_| Vec::new());

    if let Some(pos) = playlists.iter().position(|p| p.name == name) {
        playlists.remove(pos);
        let serialized = serde_json::to_string_pretty(&playlists)
            .map_err(|e| format!("Failed to serialize playlists: {}", e))?;
        fs::write(path, serialized).map_err(|e| format!("Failed to write playlists file: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
async fn open_auth_window(url: String) -> Result<(), String> {
    // Open the Spotify auth URL in the system browser directly.
    // WebView popup windows are unreliable on Windows (flash and close immediately due to
    // WebView2 lifecycle issues when the builder handle falls out of scope).
    // The system browser is always available and works perfectly with the PKCE + local
    // loopback redirect server flow.
    tauri_plugin_opener::open_url(&url, None::<&str>)
        .map_err(|e| format!("Failed to open URL in system browser: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn toggle_mini_player(window: tauri::Window, is_mini: bool) -> Result<(), String> {
    if is_mini {
        window.set_size(tauri::Size::Logical(tauri::LogicalSize { width: 340.0, height: 380.0 })).ok();
        window.set_always_on_top(true).ok();
        window.set_resizable(false).ok();
    } else {
        window.set_size(tauri::Size::Logical(tauri::LogicalSize { width: 1100.0, height: 720.0 })).ok();
        window.set_always_on_top(false).ok();
        window.set_resizable(true).ok();
    }
    Ok(())
}

#[tauri::command]
fn get_audio_output_device() -> Result<String, String> {
    use cpal::traits::{HostTrait, DeviceTrait};
    let host = cpal::default_host();
    if let Some(device) = host.default_output_device() {
        if let Ok(name) = device.name() {
            return Ok(name);
        }
    }
    Ok("Default Audio Output".to_string())
}

#[cfg(target_os = "linux")]
fn silence_alsa_logs() {
    use std::os::raw::{c_char, c_int};
    
    unsafe extern "C" fn null_error_handler(
        _file: *const c_char,
        _line: c_int,
        _function: *const c_char,
        _err: c_int,
        _fmt: *const c_char,
    ) -> c_int {
        0
    }

    unsafe extern "C" {
        fn snd_lib_error_set_handler(
            handler: Option<unsafe extern "C" fn(*const c_char, c_int, *const c_char, c_int, *const c_char) -> c_int>
        ) -> c_int;
    }

    unsafe {
        let _ = snd_lib_error_set_handler(Some(null_error_handler));
    }
}

#[cfg(target_os = "linux")]
fn silence_jack_logs() {
    use std::os::raw::{c_char, c_int, c_void};
    
    unsafe extern "C" fn null_handler(_msg: *const c_char) {}

    unsafe extern "C" {
        fn dlopen(filename: *const c_char, flag: c_int) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    }

    const RTLD_NOW: c_int = 2;

    unsafe {
        // Probe common JACK library names dynamically
        for libname in &[b"libjack.so.0\0".as_ptr(), b"libjack.so\0".as_ptr()] {
            let lib = dlopen(*libname as *const c_char, RTLD_NOW);
            if !lib.is_null() {
                let set_error = dlsym(lib, b"jack_set_error_function\0".as_ptr() as *const c_char);
                if !set_error.is_null() {
                    let func: unsafe extern "C" fn(Option<unsafe extern "C" fn(*const c_char)>) = std::mem::transmute(set_error);
                    func(Some(null_handler));
                }
                
                let set_info = dlsym(lib, b"jack_set_info_function\0".as_ptr() as *const c_char);
                if !set_info.is_null() {
                    let func: unsafe extern "C" fn(Option<unsafe extern "C" fn(*const c_char)>) = std::mem::transmute(set_info);
                    func(Some(null_handler));
                }
                break;
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    diagnostics::StartupProfiler::global().record_milestone("Boot Started");
    
    #[cfg(target_os = "linux")]
    {
        silence_alsa_logs();
        silence_jack_logs();
    }

    let (tx, rx) = tokio::sync::mpsc::channel(32);
    let command_sender = spotify::commands::SpotifyCommandSender { tx: tx.clone() };
    let shared_state = spotify::state::SharedPlaybackState::new();
    let shared_state_clone = shared_state.clone();
    let spotify_session = spotify::session::SharedSpotifySession(std::sync::Mutex::new(spotify::session::SpotifySession::default()));
    let app_config = config::SharedAppConfig(std::sync::Mutex::new(config::AppConfig::default()));


    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            let app_handle = app.handle().clone();
            
            // Initialize File Logger
            if let Ok(config_dir) = app_handle.path().app_config_dir() {
                diagnostics::FileLogger::init(config_dir);
            }
            log_info!("CrumusiX Core booting up...");
            
            // Initialize SQLite library database
            let cache_dir = cache::storage::get_cache_dir(&app_handle);
            let sqlite_path = cache_dir.join("library_sqlite.db");
            let sqlite_conn = cache::db::init_sqlite(&sqlite_path).expect("Failed to initialize SQLite library database");
            
            // Auto-evict expired metadata cache rows on startup
            if let Ok(deleted_rows) = cache::metadata::cache_purge_expired(&sqlite_conn) {
                if deleted_rows > 0 {
                    log_info!("Cleaned up {} expired metadata cache rows on startup", deleted_rows);
                }
            }
            
            let shared_conn = std::sync::Arc::new(parking_lot::Mutex::new(sqlite_conn));

            app.manage(cache::SqliteDbState {
                conn: shared_conn.clone(),
            });

            app.manage(cache::CacheDbState {
                db: shared_conn,
            });
            diagnostics::StartupProfiler::global().record_milestone("SQLite Db Initialized");

            // Initialize Local Playback Provider
            let local_provider = std::sync::Arc::new(local::LocalProvider::new());
            app.manage(local_provider.clone());
            playback::PlaybackManager::global().register_local_provider(local_provider);
            diagnostics::StartupProfiler::global().record_milestone("Local Provider Registered");

            // Initialize queue state and manage
            let queue_state = queue::SharedQueueState::new(&app_handle);
            app.manage(queue_state);

            // Spawn hardware audio device hot-swap monitor
            playback::device::AudioDeviceManager::global().start_hotplug_monitor(app_handle.clone());

            let window = app.get_webview_window("main").unwrap();
            let window_clone = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window_clone.hide();
                }
            });

            // 1. Build System Tray Menu
            let quit_i = tauri::menu::MenuItem::with_id(app, "quit", "Quit", true, None::<&str>).unwrap();
            let show_i = tauri::menu::MenuItem::with_id(app, "show", "Show Player", true, None::<&str>).unwrap();
            
            let menu = tauri::menu::Menu::with_items(app, &[&show_i, &quit_i]).unwrap();
            
            // 2. Build Tray Icon
            let icon = tauri::include_image!("icons/32x32.png");
            
            let _tray = tauri::tray::TrayIconBuilder::new()
                .icon(icon)
                .menu(&menu)
                .on_menu_event(move |app_handle, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app_handle.exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)
                .expect("Failed to build tray icon");
            diagnostics::StartupProfiler::global().record_milestone("System Tray Configured");

            spotify::worker::PlaybackWorker::spawn(app_handle, shared_state_clone, rx, tx);
            diagnostics::StartupProfiler::global().record_milestone("App Setup Complete");
            Ok(())
        })
        .manage(command_sender)
        .manage(shared_state)
        .manage(spotify_session)
        .manage(app_config)
        .invoke_handler(tauri::generate_handler![
            search_youtube,
            start_oauth_server,
            open_auth_window,
            spotify::session::spotify_generate_auth_url,
            spotify::session::spotify_exchange_code,
            spotify::session::spotify_get_session,
            spotify::session::spotify_inject_session,
            spotify::session::spotify_logout_session,
            spotify::api::spotify_fetch_liked_songs,
            spotify::api::spotify_fetch_playlists,
            spotify::api::spotify_fetch_playlist_tracks,
            config::get_app_config,
            config::save_app_config,
            get_playlists,
            save_playlist,
            delete_playlist,
            toggle_mini_player,
            get_audio_output_device,
            // Native Spotify commands
            spotify::commands::spotify_init_native,
            spotify::commands::spotify_play,
            spotify::commands::spotify_pause,
            spotify::commands::spotify_resume,
            spotify::commands::spotify_seek,
            spotify::commands::spotify_stop,
            spotify::commands::spotify_volume,
            spotify::commands::spotify_get_state,
            spotify::commands::spotify_update_settings,
            spotify::commands::spotify_preload,
            // Cache Commands
            cache::cache_get_artwork,
            cache::cache_set_track,
            cache::cache_get_track,
            cache::cache_set_playlist,
            cache::cache_get_playlist,
            // Queue Commands
            queue::queue_get_state,
            queue::queue_add_track,
            queue::queue_play_next,
            queue::queue_remove_track,
            queue::queue_reorder,
            queue::queue_shuffle,
            queue::queue_clear,
            queue::queue_set_repeat_mode,
            queue::queue_set_shuffle,
            queue::queue_set_current_index,
            lyrics::lyrics_get_for_track,
            lyrics::lyrics_get_cache_stats,
            lyrics::lyrics_purge_cache,
            // SQLite FTS5 and Local library commands
            cache::library_search,
            cache::library_get_all,
            local::library_scan_dir_async,
            local::local_play,
            local::local_pause,
            local::local_stop,
            local::local_seek,
            local::local_volume,
            diagnostics::get_diagnostics_snapshot,
            diagnostics::get_startup_profile,
            diagnostics::export_diagnostics_report,
            stats::stats_record_transition,
            stats::stats_get_smart_collection,
            stats::stats_get_dashboard,
            backup::backup_export_library,
            backup::backup_import_library
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search_youtube() {
        println!("Running YouTube scraper test...");
        let client = reqwest::Client::new();
        let query = "linkin park".to_string();
        let encoded_query = url::form_urlencoded::byte_serialize(query.as_bytes()).collect::<String>();
        let search_url = format!("https://www.youtube.com/results?search_query={}", encoded_query);

        let response = client
            .get(&search_url)
            .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .header("Accept-Language", "en-US,en;q=0.9")
            .send()
            .await
            .unwrap();

        println!("Response Status: {}", response.status());
        let html = response.text().await.unwrap();
        println!("HTML Length: {}", html.len());
        
        let has_data = html.contains("ytInitialData");
        println!("Contains 'ytInitialData': {}", has_data);
        
        if has_data {
            let idx = html.find("ytInitialData").unwrap();
            let start = if idx > 200 { idx - 200 } else { 0 };
            let end = if idx + 500 < html.len() { idx + 500 } else { html.len() };
            println!("Context around ytInitialData:\n{}", &html[start..end]);
        } else {
            println!("First 1000 chars of HTML:\n{}", &html[..1000.min(html.len())]);
        }
        
        let results = search_youtube(query).await.unwrap();
        println!("YouTube search results count: {}", results.len());
        assert!(!results.is_empty(), "YouTube search returned zero results!");
    }

    #[tokio::test]
    async fn test_oauth_server() {
        println!("Running OAuth server loopback test...");
        
        // Spawn the start_oauth_server_internal in a background task
        let server_handle = tokio::spawn(async {
            start_oauth_server_internal().await
        });

        // Yield execution to let the server bind and start listening
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        // Try binding to 8888 concurrently. Since start_oauth_server_internal is actively bound to port 8888,
        // a second bind will fail with "Address already in use".
        let concurrent_result = tokio::net::TcpListener::bind("127.0.0.1:8888").await;
        assert!(concurrent_result.is_err(), "Concurrent port bind should fail!");

        // Make an HTTP request to the local loopback server to send the mock code
        let client = reqwest::Client::new();
        let redirect_url = "http://127.0.0.1:8888/?code=mock_auth_code_9999";
        
        let response = client.get(redirect_url).send().await;
        assert!(response.is_ok(), "Failed to send HTTP redirect callback to loopback server");
        let resp = response.unwrap();
        assert_eq!(resp.status(), 200, "OAuth response page should return 200 OK");
        let html_body = resp.text().await.unwrap();
        assert!(html_body.contains("CrumusiX Desktop Linked"), "Response HTML should be the success page");

        // Wait for the spawned server task to finish and return the code
        let server_result = server_handle.await.unwrap();
        assert!(server_result.is_ok(), "OAuth server returned an error: {:?}", server_result);
        assert_eq!(server_result.unwrap(), "mock_auth_code_9999");
    }
}
