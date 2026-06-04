use crate::cache::SqliteDbState;
use crate::Track;
use tauri::{AppHandle, Manager};
use rusqlite::params;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ListeningStat {
    pub track_id: String,
    pub play_count: u32,
    pub skip_count: u32,
    pub completion_percentage: f64,
    pub last_played: Option<u64>,
    pub first_played: Option<u64>,
    pub total_play_time: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StatsDashboard {
    pub top_tracks: Vec<(Track, u32)>,
    pub top_artists: Vec<(String, u32)>,
    pub top_albums: Vec<(String, u32)>,
    pub total_listening_time_secs: u64,
}

fn parse_duration_to_secs(dur_str: &str) -> Option<u64> {
    if let Ok(ms) = dur_str.parse::<u64>() {
        if ms > 5000 {
            return Some(ms / 1000);
        }
    }
    
    let parts: Vec<&str> = dur_str.split(':').collect();
    if parts.len() == 2 {
        let mins: u64 = parts[0].parse().ok()?;
        let secs: u64 = parts[1].parse().ok()?;
        return Some(mins * 60 + secs);
    } else if parts.len() == 3 {
        let hrs: u64 = parts[0].parse().ok()?;
        let mins: u64 = parts[1].parse().ok()?;
        let secs: u64 = parts[2].parse().ok()?;
        return Some(hrs * 3600 + mins * 60 + secs);
    }
    None
}

#[tauri::command]
pub fn stats_record_transition(
    app_handle: AppHandle,
    track_id: String,
    completed: bool,
    play_time_secs: u64,
) -> Result<(), String> {
    let db_state = app_handle.try_state::<SqliteDbState>()
        .ok_or_else(|| "SqliteDbState not registered".to_string())?;
    let conn = db_state.conn.lock();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let duration_str: Option<String> = conn.query_row(
        "SELECT duration FROM tracks WHERE id = ?;",
        params![track_id],
        |row| row.get(0),
    ).ok();

    let play_inc = if completed { 1 } else { 0 };
    let skip_inc = if !completed && play_time_secs < 30 { 1 } else { 0 };
    
    let completion_pct = if completed {
        100.0
    } else if let Some(ref dur_str) = duration_str {
        if let Some(total_secs) = parse_duration_to_secs(dur_str) {
            if total_secs > 0 {
                let pct = (play_time_secs as f64 / total_secs as f64) * 100.0;
                pct.min(100.0)
            } else {
                50.0
            }
        } else {
            50.0
        }
    } else {
        50.0
    };

    // Check if statistic exists
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM listening_statistics WHERE track_id = ?);",
        params![track_id],
        |row| row.get(0),
    ).unwrap_or(false);

    if exists {
        conn.execute(
            "UPDATE listening_statistics SET 
                play_count = play_count + ?, 
                skip_count = skip_count + ?, 
                completion_percentage = (completion_percentage + ?) / 2.0,
                last_played = ?,
                total_play_time = total_play_time + ?
             WHERE track_id = ?;",
            params![play_inc, skip_inc, completion_pct, now, play_time_secs, track_id],
        ).map_err(|e| e.to_string())?;
    } else {
        conn.execute(
            "INSERT INTO listening_statistics (track_id, play_count, skip_count, completion_percentage, last_played, first_played, total_play_time) 
             VALUES (?, ?, ?, ?, ?, ?, ?);",
            params![track_id, play_inc, skip_inc, completion_pct, now, now, play_time_secs],
        ).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub fn stats_get_smart_collection(
    app_handle: AppHandle,
    collection_type: String,
) -> Result<Vec<Track>, String> {
    let db_state = app_handle.try_state::<SqliteDbState>()
        .ok_or_else(|| "SqliteDbState not registered".to_string())?;
    let conn = db_state.conn.lock();

    let sql = match collection_type.as_str() {
        "Most Played" => {
            "SELECT t.id, t.title, t.artist, t.album, t.duration, t.source, t.thumbnail 
             FROM listening_statistics s
             JOIN tracks t ON t.id = s.track_id
             WHERE s.play_count > 0
             ORDER BY s.play_count DESC LIMIT 25;"
        }
        "Recently Played" => {
            "SELECT t.id, t.title, t.artist, t.album, t.duration, t.source, t.thumbnail 
             FROM listening_statistics s
             JOIN tracks t ON t.id = s.track_id
             ORDER BY s.last_played DESC LIMIT 25;"
        }
        "Forgotten Tracks" => {
            "SELECT t.id, t.title, t.artist, t.album, t.duration, t.source, t.thumbnail 
             FROM listening_statistics s
             JOIN tracks t ON t.id = s.track_id
             WHERE s.play_count > 1
             ORDER BY s.last_played ASC LIMIT 25;"
        }
        _ => return Err("Invalid collection type requested".to_string()),
    };

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok(Track {
            id: row.get(0)?,
            title: row.get(1)?,
            artist: row.get(2)?,
            album: row.get(3)?,
            duration: row.get(4)?,
            source: row.get(5)?,
            thumbnail: row.get(6)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for r in rows {
        if let Ok(track) = r {
            results.push(track);
        }
    }
    Ok(results)
}

#[tauri::command]
pub fn stats_get_dashboard(app_handle: AppHandle) -> Result<StatsDashboard, String> {
    let db_state = app_handle.try_state::<SqliteDbState>()
        .ok_or_else(|| "SqliteDbState not registered".to_string())?;
    let conn = db_state.conn.lock();

    // 1. Get Top Tracks
    let mut stmt = conn.prepare(
        "SELECT t.id, t.title, t.artist, t.album, t.duration, t.source, t.thumbnail, s.play_count
         FROM listening_statistics s
         JOIN tracks t ON t.id = s.track_id
         ORDER BY s.play_count DESC LIMIT 5;"
    ).map_err(|e| e.to_string())?;
    
    let top_tracks_rows = stmt.query_map([], |row| {
        let track = Track {
            id: row.get(0)?,
            title: row.get(1)?,
            artist: row.get(2)?,
            album: row.get(3)?,
            duration: row.get(4)?,
            source: row.get(5)?,
            thumbnail: row.get(6)?,
        };
        let count: u32 = row.get(7)?;
        Ok((track, count))
    }).map_err(|e| e.to_string())?;
    
    let mut top_tracks = Vec::new();
    for r in top_tracks_rows {
        if let Ok(item) = r {
            top_tracks.push(item);
        }
    }

    // 2. Get Top Artists
    let mut stmt = conn.prepare(
        "SELECT t.artist, SUM(s.play_count) as total_plays
         FROM listening_statistics s
         JOIN tracks t ON t.id = s.track_id
         GROUP BY t.artist
         ORDER BY total_plays DESC LIMIT 5;"
    ).map_err(|e| e.to_string())?;
    
    let top_artists_rows = stmt.query_map([], |row| {
        let artist: String = row.get(0)?;
        let count: u32 = row.get(1)?;
        Ok((artist, count))
    }).map_err(|e| e.to_string())?;
    
    let mut top_artists = Vec::new();
    for r in top_artists_rows {
        if let Ok(item) = r {
            top_artists.push(item);
        }
    }

    // 3. Get Top Albums
    let mut stmt = conn.prepare(
        "SELECT t.album, SUM(s.play_count) as total_plays
         FROM listening_statistics s
         JOIN tracks t ON t.id = s.track_id
         GROUP BY t.album
         ORDER BY total_plays DESC LIMIT 5;"
    ).map_err(|e| e.to_string())?;
    
    let top_albums_rows = stmt.query_map([], |row| {
        let album: String = row.get(0)?;
        let count: u32 = row.get(1)?;
        Ok((album, count))
    }).map_err(|e| e.to_string())?;
    
    let mut top_albums = Vec::new();
    for r in top_albums_rows {
        if let Ok(item) = r {
            top_albums.push(item);
        }
    }

    // 4. Total listening time
    let total_listening_time_secs: u64 = conn.query_row(
        "SELECT COALESCE(SUM(total_play_time), 0) FROM listening_statistics;",
        [],
        |row| row.get(0),
    ).unwrap_or(0);

    Ok(StatsDashboard {
        top_tracks,
        top_artists,
        top_albums,
        total_listening_time_secs,
    })
}
