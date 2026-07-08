use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;
use std::time::Instant;
use parking_lot::Mutex;
use serde::Serialize;
use tauri::{AppHandle, Manager};
use crate::cache::SqliteDbState;

#[derive(Serialize, Clone, Debug)]
pub struct DiagnosticsSnapshot {
    pub ram_usage_mb: f64,
    pub provider_latency_ms: u32,
    pub buffer_health_pct: u32,
    pub cache_hits: u32,
    pub cache_misses: u32,
}

pub struct DiagnosticsManager {
    provider_latency_ms: AtomicU32,
    buffer_health_pct: AtomicU32,
    cache_hits: AtomicU32,
    cache_misses: AtomicU32,
}

impl DiagnosticsManager {
    pub fn global() -> &'static Self {
        static INSTANCE: OnceLock<DiagnosticsManager> = OnceLock::new();
        INSTANCE.get_or_init(|| Self {
            provider_latency_ms: AtomicU32::new(0),
            buffer_health_pct: AtomicU32::new(100),
            cache_hits: AtomicU32::new(0),
            cache_misses: AtomicU32::new(0),
        })
    }

    pub fn record_latency(&self, ms: u32) {
        self.provider_latency_ms.store(ms, Ordering::Relaxed);
    }

    pub fn record_buffer_health(&self, pct: u32) {
        self.buffer_health_pct.store(pct, Ordering::Relaxed);
    }

    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_snapshot(&self) -> DiagnosticsSnapshot {
        let ram_bytes = get_linux_ram_usage();
        let ram_mb = (ram_bytes as f64) / (1024.0 * 1024.0);
        
        DiagnosticsSnapshot {
            ram_usage_mb: ram_mb,
            provider_latency_ms: self.provider_latency_ms.load(Ordering::Relaxed),
            buffer_health_pct: self.buffer_health_pct.load(Ordering::Relaxed),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
        }
    }
}

fn get_linux_ram_usage() -> u64 {
    if let Ok(content) = std::fs::read_to_string("/proc/self/statm") {
        let mut parts = content.split_whitespace();
        if let Some(rss_pages_str) = parts.nth(1) {
            if let Ok(rss_pages) = rss_pages_str.parse::<u64>() {
                return rss_pages * 4096;
            }
        }
    }
    0
}

#[derive(Serialize, Clone, Debug)]
pub struct StartupProfilerSnapshot {
    pub milestones: Vec<(String, u128)>,
    pub total_ms: u128,
}

pub struct StartupProfiler {
    start_time: Instant,
    milestones: Mutex<Vec<(String, u128)>>,
}

impl StartupProfiler {
    pub fn global() -> &'static Self {
        static INSTANCE: OnceLock<StartupProfiler> = OnceLock::new();
        INSTANCE.get_or_init(|| Self {
            start_time: Instant::now(),
            milestones: Mutex::new(Vec::new()),
        })
    }

    pub fn record_milestone(&self, name: &str) {
        let duration = self.start_time.elapsed().as_millis();
        self.milestones.lock().push((name.to_string(), duration));
    }

    pub fn get_snapshot(&self) -> StartupProfilerSnapshot {
        let milestones = self.milestones.lock().clone();
        let total_ms = self.start_time.elapsed().as_millis();
        StartupProfilerSnapshot {
            milestones,
            total_ms,
        }
    }
}

#[tauri::command]
pub fn get_diagnostics_snapshot() -> DiagnosticsSnapshot {
    DiagnosticsManager::global().get_snapshot()
}

#[tauri::command]
pub fn get_startup_profile() -> StartupProfilerSnapshot {
    StartupProfiler::global().get_snapshot()
}

#[tauri::command]
pub fn export_diagnostics_report(
    app_handle: AppHandle,
    filepath: String,
) -> Result<(), String> {
    let db_state = app_handle.try_state::<SqliteDbState>()
        .ok_or_else(|| "SqliteDbState not registered".to_string())?;
    let conn = db_state.conn.lock();

    // Query schema version
    let database_version: i32 = conn.query_row(
        "SELECT version FROM schema_version LIMIT 1;",
        [],
        |row| row.get(0),
    ).unwrap_or(0);

    let startup_profile = StartupProfiler::global().get_snapshot();
    let memory_metrics = DiagnosticsManager::global().get_snapshot();

    // Censored diagnostic data structures
    let report = serde_json::json!({
        "app_version": env!("CARGO_PKG_VERSION"),
        "database_version": database_version,
        "startup_profile": startup_profile,
        "memory_metrics": memory_metrics,
        "system_info": {
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
        }
    });

    let serialized = serde_json::to_string_pretty(&report)
        .map_err(|e| format!("Failed to serialize diagnostic report: {}", e))?;
        
    std::fs::write(filepath, serialized)
        .map_err(|e| format!("Failed to write diagnostic report file: {}", e))?;

    Ok(())
}

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

pub struct FileLogger {
    log_path: PathBuf,
    mutex: Mutex<()>,
}

impl FileLogger {
    pub fn global() -> Option<&'static Self> {
        static INSTANCE: OnceLock<FileLogger> = OnceLock::new();
        INSTANCE.get()
    }

    pub fn init(config_dir: PathBuf) -> &'static Self {
        static INSTANCE: OnceLock<FileLogger> = OnceLock::new();
        INSTANCE.get_or_init(|| {
            let mut log_path = config_dir;
            let _ = std::fs::create_dir_all(&log_path);
            log_path.push("app.log");
            
            // If the log is too big (> 5MB), rotate it
            if log_path.exists() {
                if let Ok(metadata) = std::fs::metadata(&log_path) {
                    if metadata.len() > 5 * 1024 * 1024 {
                        let rotated_path = log_path.with_extension("log.old");
                        let _ = std::fs::rename(&log_path, &rotated_path);
                    }
                }
            }

            FileLogger {
                log_path,
                mutex: Mutex::new(()),
            }
        })
    }

    pub fn log(&self, level: &str, message: &str) {
        let _lock = self.mutex.lock();
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
        {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let _ = writeln!(file, "[{}] [{}] {}", timestamp, level, message);
        }
    }
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        eprintln!("[INFO] {}", msg);
        if let Some(logger) = $crate::diagnostics::FileLogger::global() {
            logger.log("INFO", &msg);
        }
    }};
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        eprintln!("[WARN] {}", msg);
        if let Some(logger) = $crate::diagnostics::FileLogger::global() {
            logger.log("WARN", &msg);
        }
    }};
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        eprintln!("[ERROR] {}", msg);
        if let Some(logger) = $crate::diagnostics::FileLogger::global() {
            logger.log("ERROR", &msg);
        }
    }};
}
