use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::OnceLock;
use parking_lot::Mutex;
use tauri::{AppHandle, State};
use rodio::{Decoder, Sink};
use async_trait::async_trait;
use lofty::probe::Probe;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::tag::Accessor;

use crate::playback::provider::{PlaybackProvider, ProviderCapabilities};
use crate::cache::SqliteDbState;
use crate::Track;

// Global lazy output stream receiver. Leaks the stream to keep it running
// globally while exposing only the thread-safe handle.
static OUTPUT_STREAM_HANDLE: OnceLock<rodio::OutputStreamHandle> = OnceLock::new();

pub fn get_output_stream_handle() -> Result<&'static rodio::OutputStreamHandle, String> {
    if let Some(handle) = OUTPUT_STREAM_HANDLE.get() {
        return Ok(handle);
    }
    let (stream, handle) = rodio::OutputStream::try_default()
        .map_err(|e| format!("Failed to open default audio output stream: {}", e))?;
    std::mem::forget(stream); // Leak the stream to keep the decoders active forever
    match OUTPUT_STREAM_HANDLE.set(handle) {
        Ok(()) => Ok(OUTPUT_STREAM_HANDLE.get().unwrap()),
        Err(_) => Ok(OUTPUT_STREAM_HANDLE.get().unwrap()), // race: another thread already set it
    }
}

pub struct LocalProviderInner {
    sink: Option<Sink>,
    current_track_path: Option<PathBuf>,
}

pub struct LocalProvider {
    inner: Arc<Mutex<LocalProviderInner>>,
}

impl LocalProvider {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(LocalProviderInner {
                sink: None,
                current_track_path: None,
            })),
        }
    }
}

#[async_trait]
impl PlaybackProvider for LocalProvider {
    async fn play(&self, track_id: &str) -> Result<(), String> {
        let mut inner = self.inner.lock();
        
        let path = PathBuf::from(track_id);
        if !path.exists() {
            return Err(format!("Local file does not exist: {}", track_id));
        }

        if let Some(ref s) = inner.sink {
            s.stop();
        }
        
        let stream_handle = get_output_stream_handle()?;
        let sink = Sink::try_new(stream_handle)
            .map_err(|e| format!("Failed to create sink: {}", e))?;
            
        let file = File::open(&path)
            .map_err(|e| format!("Failed to open file: {}", e))?;
        let reader = BufReader::new(file);
        let source = Decoder::new(reader)
            .map_err(|e| format!("Failed to decode file: {}", e))?;
            
        sink.append(source);
        sink.play();
        
        inner.sink = Some(sink);
        inner.current_track_path = Some(path);
        
        Ok(())
    }

    async fn pause(&self) -> Result<(), String> {
        let inner = self.inner.lock();
        if let Some(ref s) = inner.sink {
            s.pause();
        }
        Ok(())
    }

    async fn stop(&self) -> Result<(), String> {
        let mut inner = self.inner.lock();
        if let Some(ref s) = inner.sink {
            s.stop();
        }
        inner.sink = None;
        inner.current_track_path = None;
        Ok(())
    }

    async fn seek(&self, position_ms: u64) -> Result<(), String> {
        let inner = self.inner.lock();
        if let Some(ref s) = inner.sink {
            let duration = std::time::Duration::from_millis(position_ms);
            let _ = s.try_seek(duration);
        }
        Ok(())
    }

    async fn set_volume(&self, volume: f32) -> Result<(), String> {
        let inner = self.inner.lock();
        if let Some(ref s) = inner.sink {
            s.set_volume(volume);
        }
        Ok(())
    }

    fn get_capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            can_seek: true,
            supports_video: false,
            supports_lyrics: false,
            supports_queue: true,
            supports_metadata: true,
        }
    }
}

fn calculate_file_hash(file_path: &Path) -> Result<String, String> {
    use std::io::Read;
    let file = File::open(file_path).map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(file);
    let mut buffer = [0u8; 8192];
    let mut total_read = 0;
    let mut hash: u64 = 0xcbf29ce484222325;
    
    while let Ok(n) = reader.read(&mut buffer) {
        if n == 0 || total_read >= 4 * 1024 * 1024 {
            break;
        }
        for byte in &buffer[..n] {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        total_read += n;
    }
    Ok(format!("{:016x}", hash))
}

use tauri::Emitter;

pub fn scan_directory(
    dir_path: &str,
    conn: Arc<Mutex<rusqlite::Connection>>,
    cache_dir: &Path,
    app_handle: AppHandle,
) -> Result<usize, String> {
    let path = Path::new(dir_path);
    if !path.exists() || !path.is_dir() {
        return Err(format!("Directory does not exist: {}", dir_path));
    }
    
    let artwork_dir = cache_dir.join("artwork_cache");
    std::fs::create_dir_all(&artwork_dir).ok();
    
    let mut files_to_scan = Vec::new();
    find_audio_files(path, &mut files_to_scan);
    
    let total_files = files_to_scan.len();
    if total_files == 0 {
        return Ok(0);
    }

    let (tx, rx) = std::sync::mpsc::channel();
    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    
    let chunk_size = (total_files + num_threads - 1) / num_threads;
    let files_to_scan = Arc::new(files_to_scan);
    let artwork_dir = Arc::new(artwork_dir);
    
    for i in 0..num_threads {
        let tx = tx.clone();
        let files = Arc::clone(&files_to_scan);
        let art_dir = Arc::clone(&artwork_dir);
        let start_idx = i * chunk_size;
        let end_idx = std::cmp::min(start_idx + chunk_size, total_files);
        
        if start_idx >= total_files {
            break;
        }
        
        std::thread::spawn(move || {
            for idx in start_idx..end_idx {
                let file_path = &files[idx];
                if let Ok(track) = parse_audio_file(file_path, &art_dir) {
                    let hash = calculate_file_hash(file_path).unwrap_or_default();
                    let _ = tx.send(Ok((track, hash, file_path.clone())));
                } else {
                    let _ = tx.send(Err(()));
                }
            }
        });
    }
    drop(tx);

    let mut conn_lock = conn.lock();
    let tx_db = conn_lock.transaction().map_err(|e| format!("Failed to begin transaction: {}", e))?;
    
    let mut added_count = 0;
    let mut processed_count = 0;
    
    while let Ok(msg) = rx.recv() {
        processed_count += 1;
        
        if let Ok((track, hash, file_path)) = msg {
            let mut is_reconciled = false;
            {
                let mut stmt = tx_db.prepare("SELECT path FROM tracks WHERE file_hash = ?;").map_err(|e| e.to_string())?;
                let mut rows = stmt.query(rusqlite::params![hash]).map_err(|e| e.to_string())?;
                if let Some(row) = rows.next().map_err(|e| e.to_string())? {
                    let existing_path: String = row.get(0).map_err(|e| e.to_string())?;
                    let current_path_str = file_path.to_string_lossy().to_string();
                    if existing_path != current_path_str {
                        tx_db.execute(
                            "UPDATE tracks SET id = ?, path = ? WHERE file_hash = ?;",
                            rusqlite::params![current_path_str, current_path_str, hash],
                        ).map_err(|e| e.to_string())?;
                        is_reconciled = true;
                    }
                }
            }
            
            if !is_reconciled {
                let res = tx_db.execute(
                    "INSERT OR REPLACE INTO tracks (id, title, artist, album, duration, source, thumbnail, path, file_hash)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9);",
                    rusqlite::params![
                        track.id,
                        track.title,
                        track.artist,
                        track.album,
                        track.duration,
                        track.source,
                        track.thumbnail,
                        file_path.to_string_lossy().to_string(),
                        hash,
                    ],
                );
                if res.is_ok() {
                    added_count += 1;
                }
            } else {
                added_count += 1;
            }
        }
        
        if processed_count % 10 == 0 || processed_count == total_files {
            let percentage = (processed_count as f64 / total_files as f64) * 100.0;
            let _ = app_handle.emit("scan-progress", serde_json::json!({
                "current": processed_count,
                "total": total_files,
                "percentage": percentage
            }));
        }
    }
    
    tx_db.commit().map_err(|e| format!("Failed to commit scan transaction: {}", e))?;
    
    Ok(added_count)
}

fn find_audio_files(path: &Path, files: &mut Vec<PathBuf>) {
    if path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    find_audio_files(&entry_path, files);
                } else {
                    if let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) {
                        let ext_lower = ext.to_lowercase();
                        if ext_lower == "mp3" || ext_lower == "flac" || ext_lower == "ogg" || 
                           ext_lower == "wav" || ext_lower == "aac" || ext_lower == "m4a" {
                            files.push(entry_path);
                        }
                    }
                }
            }
        }
    }
}

fn parse_audio_file(file_path: &Path, artwork_dir: &Path) -> Result<Track, String> {
    let tagged_file = Probe::open(file_path)
        .map_err(|e| e.to_string())?
        .read()
        .map_err(|e| e.to_string())?;
        
    let properties = tagged_file.properties();
    let duration_secs = properties.duration().as_secs();
    let duration_formatted = format_seconds(duration_secs);
    
    let mut title = file_path.file_stem().and_then(|s| s.to_str()).unwrap_or("Unknown").to_string();
    let mut artist = "Unknown Artist".to_string();
    let mut album = "Unknown Album".to_string();
    let mut artwork_path = "".to_string();
    
    if let Some(tag) = tagged_file.primary_tag() {
        if let Some(t) = tag.title() {
            if !t.trim().is_empty() { title = t.to_string(); }
        }
        if let Some(a) = tag.artist() {
            if !a.trim().is_empty() { artist = a.to_string(); }
        }
        if let Some(al) = tag.album() {
            if !al.trim().is_empty() { album = al.to_string(); }
        }
        
        let pictures = tag.pictures();
        if let Some(pic) = pictures.first() {
            let pic_data = pic.data();
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            pic_data.hash(&mut hasher);
            let pic_hash = hasher.finish();
            
            let ext = match pic.mime_type() {
                Some(lofty::picture::MimeType::Jpeg) => "jpg",
                Some(lofty::picture::MimeType::Png) => "png",
                _ => "webp",
            };
            
            let out_file_name = format!("{}.{}", pic_hash, ext);
            let out_path = artwork_dir.join(&out_file_name);
            if !out_path.exists() {
                let _ = std::fs::write(&out_path, pic_data);
            }
            
            artwork_path = out_path.to_string_lossy().to_string();
        }
    }
    
    let id = file_path.to_string_lossy().to_string();
    
    Ok(Track {
        id,
        title,
        artist,
        album,
        duration: duration_formatted,
        source: "local".to_string(),
        thumbnail: artwork_path,
    })
}

fn format_seconds(seconds: u64) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    format!("{}:{:02}", mins, secs)
}

#[tauri::command]
pub async fn library_scan_dir_async(
    dir_path: String,
    state: State<'_, SqliteDbState>,
    app_handle: AppHandle,
) -> Result<usize, String> {
    let conn = state.conn.clone();
    let cache_dir = crate::cache::storage::get_cache_dir(&app_handle);
    
    tokio::task::spawn_blocking(move || {
        scan_directory(&dir_path, conn, &cache_dir, app_handle)
    })
    .await
    .map_err(|e| format!("Task join failed: {}", e))?
}

// Local playback command delegation
#[tauri::command]
pub async fn local_play(
    track_id: String,
    local_provider: State<'_, Arc<LocalProvider>>,
) -> Result<(), String> {
    local_provider.play(&track_id).await
}

#[tauri::command]
pub async fn local_pause(
    local_provider: State<'_, Arc<LocalProvider>>,
) -> Result<(), String> {
    local_provider.pause().await
}

#[tauri::command]
pub async fn local_stop(
    local_provider: State<'_, Arc<LocalProvider>>,
) -> Result<(), String> {
    local_provider.stop().await
}

#[tauri::command]
pub async fn local_seek(
    position_ms: u64,
    local_provider: State<'_, Arc<LocalProvider>>,
) -> Result<(), String> {
    local_provider.seek(position_ms).await
}

#[tauri::command]
pub async fn local_volume(
    volume: f32,
    local_provider: State<'_, Arc<LocalProvider>>,
) -> Result<(), String> {
    local_provider.set_volume(volume).await
}
