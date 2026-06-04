use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use tauri::{State, AppHandle, Emitter, Manager};
use std::path::PathBuf;
use rand::seq::SliceRandom;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QueueItem {
    pub track_id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u32,
    pub artwork_url: String,
    pub source: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlaybackQueue {
    pub current_index: i32,
    pub items: Vec<QueueItem>,
    pub repeat_mode: String,
    pub is_shuffle: bool,
}

impl Default for PlaybackQueue {
    fn default() -> Self {
        Self {
            current_index: -1,
            items: Vec::new(),
            repeat_mode: "off".to_string(),
            is_shuffle: false,
        }
    }
}

#[derive(Clone)]
pub struct SharedQueueState {
    pub inner: Arc<RwLock<PlaybackQueue>>,
}

impl SharedQueueState {
    pub fn new(app_handle: &AppHandle) -> Self {
        let queue = load_queue(app_handle);
        Self {
            inner: Arc::new(RwLock::new(queue)),
        }
    }
}

pub fn get_queue_path(app_handle: &AppHandle) -> PathBuf {
    let mut path = app_handle
        .path()
        .app_config_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    std::fs::create_dir_all(&path).ok();
    path.push("queue.json");
    path
}

pub fn save_queue(app_handle: &AppHandle, queue: &PlaybackQueue) {
    let path = get_queue_path(app_handle);
    let tmp_path = path.with_extension("json.tmp");
    if let Ok(serialized) = serde_json::to_string_pretty(queue) {
        if std::fs::write(&tmp_path, serialized).is_ok() {
            let _ = std::fs::rename(tmp_path, path);
        }
    }
}

pub fn load_queue(app_handle: &AppHandle) -> PlaybackQueue {
    let path = get_queue_path(app_handle);
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(mut queue) = serde_json::from_str::<PlaybackQueue>(&content) {
                // Purge legacy massive base64 strings to free memory immediately
                for item in &mut queue.items {
                    if item.artwork_url.starts_with("data:image") {
                        item.artwork_url = "".to_string();
                    }
                }
                
                // Sanitize queue items: filter out local tracks whose referenced files no longer exist
                let original_len = queue.items.len();
                queue.items.retain(|item| {
                    if item.source == "local" {
                        let p = std::path::Path::new(&item.track_id);
                        p.exists()
                    } else {
                        true
                    }
                });
                
                // Adjust current index if items were filtered
                if queue.items.is_empty() {
                    queue.current_index = -1;
                } else if queue.current_index >= queue.items.len() as i32 {
                    queue.current_index = (queue.items.len() as i32) - 1;
                }
                
                if queue.items.len() != original_len {
                    save_queue(app_handle, &queue);
                }
                
                return queue;
            }
        }
    }
    PlaybackQueue::default()
}

fn emit_queue_updated(app_handle: &AppHandle, queue: &PlaybackQueue) {
    save_queue(app_handle, queue);
    app_handle.emit("queue-updated", queue.clone()).ok();
}

#[tauri::command]
pub fn queue_get_state(state: State<'_, SharedQueueState>) -> PlaybackQueue {
    state.inner.read().clone()
}

#[tauri::command]
pub fn queue_add_track(
    item: QueueItem,
    state: State<'_, SharedQueueState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let mut queue = state.inner.write();
    queue.items.push(item);
    if queue.current_index == -1 {
        queue.current_index = 0;
    }
    emit_queue_updated(&app_handle, &queue);
    Ok(())
}

#[tauri::command]
pub fn queue_play_next(
    item: QueueItem,
    state: State<'_, SharedQueueState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let mut queue = state.inner.write();
    let insert_pos = if queue.current_index == -1 {
        0
    } else {
        (queue.current_index + 1) as usize
    };
    
    if insert_pos >= queue.items.len() {
        queue.items.push(item);
    } else {
        queue.items.insert(insert_pos, item);
    }
    
    if queue.current_index == -1 {
        queue.current_index = 0;
    }
    emit_queue_updated(&app_handle, &queue);
    Ok(())
}

#[tauri::command]
pub fn queue_remove_track(
    index: usize,
    state: State<'_, SharedQueueState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let mut queue = state.inner.write();
    if index < queue.items.len() {
        queue.items.remove(index);
        
        let idx = queue.current_index;
        if queue.items.is_empty() {
            queue.current_index = -1;
        } else if idx >= index as i32 {
            queue.current_index = (idx - 1).max(0);
        }
        emit_queue_updated(&app_handle, &queue);
    }
    Ok(())
}

#[tauri::command]
pub fn queue_reorder(
    from_index: usize,
    to_index: usize,
    state: State<'_, SharedQueueState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let mut queue = state.inner.write();
    if from_index < queue.items.len() && to_index < queue.items.len() {
        let item = queue.items.remove(from_index);
        queue.items.insert(to_index, item);
        
        // Adjust current pointer if it was affected
        let idx = queue.current_index as usize;
        if idx == from_index {
            queue.current_index = to_index as i32;
        } else if from_index < idx && to_index >= idx {
            queue.current_index -= 1;
        } else if from_index > idx && to_index <= idx {
            queue.current_index += 1;
        }
        emit_queue_updated(&app_handle, &queue);
    }
    Ok(())
}

#[tauri::command]
pub fn queue_shuffle(
    state: State<'_, SharedQueueState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let mut queue = state.inner.write();
    if queue.items.len() > 1 {
        let current_idx = queue.current_index as usize;
        let mut rng = rand::thread_rng();
        
        if current_idx < queue.items.len() {
            // Keep currently playing track in place, shuffle the rest
            let current_item = queue.items.remove(current_idx);
            queue.items.shuffle(&mut rng);
            queue.items.insert(0, current_item);
            queue.current_index = 0;
        } else {
            queue.items.shuffle(&mut rng);
        }
        emit_queue_updated(&app_handle, &queue);
    }
    Ok(())
}

#[tauri::command]
pub fn queue_clear(
    state: State<'_, SharedQueueState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let mut queue = state.inner.write();
    queue.items.clear();
    queue.current_index = -1;
    emit_queue_updated(&app_handle, &queue);
    Ok(())
}

#[tauri::command]
pub fn queue_set_repeat_mode(
    mode: String,
    state: State<'_, SharedQueueState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let mut queue = state.inner.write();
    queue.repeat_mode = mode;
    emit_queue_updated(&app_handle, &queue);
    Ok(())
}

#[tauri::command]
pub fn queue_set_shuffle(
    shuffle: bool,
    state: State<'_, SharedQueueState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let mut queue = state.inner.write();
    queue.is_shuffle = shuffle;
    emit_queue_updated(&app_handle, &queue);
    Ok(())
}

#[tauri::command]
pub fn queue_set_current_index(
    index: i32,
    state: State<'_, SharedQueueState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let mut queue = state.inner.write();
    if index >= -1 && index < queue.items.len() as i32 {
        queue.current_index = index;
        emit_queue_updated(&app_handle, &queue);
    }
    Ok(())
}
