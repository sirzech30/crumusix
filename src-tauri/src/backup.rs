use crate::cache::SqliteDbState;
use tauri::{AppHandle, Manager};
use rusqlite::params;
use serde::{Serialize, Deserialize};
use std::fs;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BackupTrack {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: String,
    pub source: String,
    pub thumbnail: String,
    pub path: Option<String>,
    pub file_hash: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BackupData {
    pub settings: Vec<(String, String)>,
    pub tracks: Vec<BackupTrack>,
    pub playlists: Vec<(String, String)>, // (name, tracks_json)
    pub statistics: Vec<BackupStat>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BackupStat {
    pub track_id: String,
    pub play_count: u32,
    pub skip_count: u32,
    pub completion_percentage: f64,
    pub last_played: Option<u64>,
    pub first_played: Option<u64>,
    pub total_play_time: u64,
}

#[tauri::command]
pub fn backup_export_library(app_handle: AppHandle, filepath: String) -> Result<(), String> {
    let db_state = app_handle.try_state::<SqliteDbState>()
        .ok_or_else(|| "SqliteDbState not registered".to_string())?;
    let conn = db_state.conn.lock();

    // 1. Export settings (app_state key/values)
    let mut stmt = conn.prepare("SELECT key, value FROM app_state;").map_err(|e| e.to_string())?;
    let settings_rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }).map_err(|e| e.to_string())?;
    let mut settings = Vec::new();
    for r in settings_rows {
        if let Ok(item) = r {
            settings.push(item);
        }
    }

    // 2. Export tracks
    let mut stmt = conn.prepare("SELECT id, title, artist, album, duration, source, thumbnail, path, file_hash FROM tracks;").map_err(|e| e.to_string())?;
    let tracks_rows = stmt.query_map([], |row| {
        Ok(BackupTrack {
            id: row.get(0)?,
            title: row.get(1)?,
            artist: row.get(2)?,
            album: row.get(3)?,
            duration: row.get(4)?,
            source: row.get(5)?,
            thumbnail: row.get(6)?,
            path: row.get(7)?,
            file_hash: row.get(8)?,
        })
    }).map_err(|e| e.to_string())?;
    let mut tracks = Vec::new();
    for r in tracks_rows {
        if let Ok(item) = r {
            tracks.push(item);
        }
    }

    // 3. Export playlists
    let mut stmt = conn.prepare("SELECT name, tracks FROM playlists;").map_err(|e| e.to_string())?;
    let playlists_rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }).map_err(|e| e.to_string())?;
    let mut playlists = Vec::new();
    for r in playlists_rows {
        if let Ok(item) = r {
            playlists.push(item);
        }
    }

    // 4. Export statistics
    let mut stmt = conn.prepare("SELECT track_id, play_count, skip_count, completion_percentage, last_played, first_played, total_play_time FROM listening_statistics;").map_err(|e| e.to_string())?;
    let stats_rows = stmt.query_map([], |row| {
        Ok(BackupStat {
            track_id: row.get(0)?,
            play_count: row.get(1)?,
            skip_count: row.get(2)?,
            completion_percentage: row.get(3)?,
            last_played: row.get(4)?,
            first_played: row.get(5)?,
            total_play_time: row.get(6)?,
        })
    }).map_err(|e| e.to_string())?;
    let mut statistics = Vec::new();
    for r in stats_rows {
        if let Ok(item) = r {
            statistics.push(item);
        }
    }

    let backup = BackupData {
        settings,
        tracks,
        playlists,
        statistics,
    };

    let serialized = serde_json::to_string_pretty(&backup)
        .map_err(|e| format!("Failed to serialize backup: {}", e))?;
        
    fs::write(filepath, serialized)
        .map_err(|e| format!("Failed to write backup file: {}", e))?;
        
    Ok(())
}

#[tauri::command]
pub fn backup_import_library(app_handle: AppHandle, filepath: String, import_type: String) -> Result<(), String> {
    let db_state = app_handle.try_state::<SqliteDbState>()
        .ok_or_else(|| "SqliteDbState not registered".to_string())?;
    let mut conn = db_state.conn.lock();

    let content = fs::read_to_string(filepath)
        .map_err(|e| format!("Failed to read backup file: {}", e))?;
        
    let backup: BackupData = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse backup JSON: {}", e))?;

    let tx = conn.transaction().map_err(|e| e.to_string())?;

    if import_type == "Full Backup" || import_type == "Settings Only" {
        // Restore settings
        for (key, val) in &backup.settings {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            tx.execute(
                "INSERT OR REPLACE INTO app_state (key, value, updated_at) VALUES (?, ?, ?);",
                params![key, val, now],
            ).map_err(|e| e.to_string())?;
        }
    }

    if import_type == "Full Backup" || import_type == "Library" {
        // Restore tracks
        for track in &backup.tracks {
            let path_val = track.path.as_ref().unwrap_or(&track.id);
            tx.execute(
                "INSERT OR REPLACE INTO tracks (id, title, artist, album, duration, source, thumbnail, path, file_hash) 
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?);",
                params![
                    track.id,
                    track.title,
                    track.artist,
                    track.album,
                    track.duration,
                    track.source,
                    track.thumbnail,
                    path_val,
                    track.file_hash
                ],
            ).map_err(|e| e.to_string())?;
        }

        // Restore playlists
        for (name, tracks_json) in &backup.playlists {
            if let Ok(mut playlist_tracks) = serde_json::from_str::<Vec<crate::Track>>(tracks_json) {
                playlist_tracks.retain(|pt| {
                    backup.tracks.iter().any(|bt| bt.id == pt.id)
                });
                
                if let Ok(sanitized_json) = serde_json::to_string(&playlist_tracks) {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    tx.execute(
                        "INSERT OR REPLACE INTO playlists (name, tracks, updated_at) VALUES (?, ?, ?);",
                        params![name, sanitized_json, now],
                    ).map_err(|e| e.to_string())?;
                }
            } else {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                tx.execute(
                    "INSERT OR REPLACE INTO playlists (name, tracks, updated_at) VALUES (?, ?, ?);",
                    params![name, tracks_json, now],
                ).map_err(|e| e.to_string())?;
            }
        }

        // Restore statistics
        for stat in &backup.statistics {
            tx.execute(
                "INSERT OR REPLACE INTO listening_statistics (track_id, play_count, skip_count, completion_percentage, last_played, first_played, total_play_time) 
                 VALUES (?, ?, ?, ?, ?, ?, ?);",
                params![stat.track_id, stat.play_count, stat.skip_count, stat.completion_percentage, stat.last_played, stat.first_played, stat.total_play_time],
            ).map_err(|e| e.to_string())?;
        }
    }

    tx.commit().map_err(|e| format!("Failed to commit database import: {}", e))?;
    Ok(())
}
