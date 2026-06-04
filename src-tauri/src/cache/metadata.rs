use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use rusqlite::{params, Connection};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CachedMetadata<T> {
    pub data: T,
    pub cached_at: u64,
    pub expires_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CachedTrack {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u32,
    pub artwork_url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CachedAlbum {
    pub id: String,
    pub name: String,
    pub artist: String,
    pub release_date: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CachedArtist {
    pub id: String,
    pub name: String,
    pub genres: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CachedPlaylist {
    pub id: String,
    pub name: String,
    pub owner: String,
    pub tracks: Vec<CachedTrack>,
}

fn get_current_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

const TRACK_LIFETIME: u64 = 7 * 24 * 60 * 60; // 7 Days
const ALBUM_LIFETIME: u64 = 30 * 24 * 60 * 60; // 30 Days
const ARTIST_LIFETIME: u64 = 30 * 24 * 60 * 60; // 30 Days
const PLAYLIST_LIFETIME: u64 = 12 * 60 * 60; // 12 Hours

pub fn cache_track(db: &Connection, track: CachedTrack) -> Result<(), String> {
    let now = get_current_time();
    let entry = CachedMetadata {
        data: track.clone(),
        cached_at: now,
        expires_at: now + TRACK_LIFETIME,
    };
    let serialized = serde_json::to_string(&entry).map_err(|e| e.to_string())?;
    db.execute(
        "INSERT OR REPLACE INTO metadata_cache (cache_key, cache_type, value, expires_at) VALUES (?, 'track', ?, ?);",
        params![track.id, serialized, entry.expires_at],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_cached_track(db: &Connection, id: &str) -> Option<CachedTrack> {
    let mut stmt = db.prepare("SELECT value, expires_at FROM metadata_cache WHERE cache_key = ? AND cache_type = 'track';").ok()?;
    let mut rows = stmt.query(params![id]).ok()?;
    let row = rows.next().ok()??;
    let value_str: String = row.get(0).ok()?;
    let expires_at: u64 = row.get(1).ok()?;
    
    let now = get_current_time();
    if now > expires_at {
        let _ = db.execute("DELETE FROM metadata_cache WHERE cache_key = ? AND cache_type = 'track';", params![id]);
        None
    } else {
        let mut entry: CachedMetadata<CachedTrack> = serde_json::from_str(&value_str).ok()?;
        // Strip legacy massive base64 data URLs to instantly clear memory bloat
        if entry.data.artwork_url.starts_with("data:image") {
            entry.data.artwork_url = "".to_string();
            if let Ok(serialized) = serde_json::to_string(&entry) {
                let _ = db.execute(
                    "UPDATE metadata_cache SET value = ? WHERE cache_key = ? AND cache_type = 'track';",
                    params![serialized, id],
                );
            }
        }
        Some(entry.data)
    }
}

pub fn cache_album(db: &Connection, album: CachedAlbum) -> Result<(), String> {
    let now = get_current_time();
    let entry = CachedMetadata {
        data: album.clone(),
        cached_at: now,
        expires_at: now + ALBUM_LIFETIME,
    };
    let serialized = serde_json::to_string(&entry).map_err(|e| e.to_string())?;
    db.execute(
        "INSERT OR REPLACE INTO metadata_cache (cache_key, cache_type, value, expires_at) VALUES (?, 'album', ?, ?);",
        params![album.id, serialized, entry.expires_at],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_cached_album(db: &Connection, id: &str) -> Option<CachedAlbum> {
    let mut stmt = db.prepare("SELECT value, expires_at FROM metadata_cache WHERE cache_key = ? AND cache_type = 'album';").ok()?;
    let mut rows = stmt.query(params![id]).ok()?;
    let row = rows.next().ok()??;
    let value_str: String = row.get(0).ok()?;
    let expires_at: u64 = row.get(1).ok()?;
    
    let now = get_current_time();
    if now > expires_at {
        let _ = db.execute("DELETE FROM metadata_cache WHERE cache_key = ? AND cache_type = 'album';", params![id]);
        None
    } else {
        let entry: CachedMetadata<CachedAlbum> = serde_json::from_str(&value_str).ok()?;
        Some(entry.data)
    }
}

pub fn cache_artist(db: &Connection, artist: CachedArtist) -> Result<(), String> {
    let now = get_current_time();
    let entry = CachedMetadata {
        data: artist.clone(),
        cached_at: now,
        expires_at: now + ARTIST_LIFETIME,
    };
    let serialized = serde_json::to_string(&entry).map_err(|e| e.to_string())?;
    db.execute(
        "INSERT OR REPLACE INTO metadata_cache (cache_key, cache_type, value, expires_at) VALUES (?, 'artist', ?, ?);",
        params![artist.id, serialized, entry.expires_at],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_cached_artist(db: &Connection, id: &str) -> Option<CachedArtist> {
    let mut stmt = db.prepare("SELECT value, expires_at FROM metadata_cache WHERE cache_key = ? AND cache_type = 'artist';").ok()?;
    let mut rows = stmt.query(params![id]).ok()?;
    let row = rows.next().ok()??;
    let value_str: String = row.get(0).ok()?;
    let expires_at: u64 = row.get(1).ok()?;
    
    let now = get_current_time();
    if now > expires_at {
        let _ = db.execute("DELETE FROM metadata_cache WHERE cache_key = ? AND cache_type = 'artist';", params![id]);
        None
    } else {
        let entry: CachedMetadata<CachedArtist> = serde_json::from_str(&value_str).ok()?;
        Some(entry.data)
    }
}

pub fn cache_playlist(db: &Connection, playlist: CachedPlaylist) -> Result<(), String> {
    let now = get_current_time();
    let entry = CachedMetadata {
        data: playlist.clone(),
        cached_at: now,
        expires_at: now + PLAYLIST_LIFETIME,
    };
    let serialized = serde_json::to_string(&entry).map_err(|e| e.to_string())?;
    db.execute(
        "INSERT OR REPLACE INTO metadata_cache (cache_key, cache_type, value, expires_at) VALUES (?, 'playlist', ?, ?);",
        params![playlist.id, serialized, entry.expires_at],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_cached_playlist(db: &Connection, id: &str) -> Option<CachedPlaylist> {
    let mut stmt = db.prepare("SELECT value, expires_at FROM metadata_cache WHERE cache_key = ? AND cache_type = 'playlist';").ok()?;
    let mut rows = stmt.query(params![id]).ok()?;
    let row = rows.next().ok()??;
    let value_str: String = row.get(0).ok()?;
    let expires_at: u64 = row.get(1).ok()?;
    
    let now = get_current_time();
    if now > expires_at {
        let _ = db.execute("DELETE FROM metadata_cache WHERE cache_key = ? AND cache_type = 'playlist';", params![id]);
        None
    } else {
        let entry: CachedMetadata<CachedPlaylist> = serde_json::from_str(&value_str).ok()?;
        Some(entry.data)
    }
}

pub fn cache_purge_expired(db: &Connection) -> Result<usize, String> {
    let now = get_current_time();
    let rows_deleted = db.execute(
        "DELETE FROM metadata_cache WHERE expires_at < ?;",
        params![now],
    ).map_err(|e| e.to_string())?;
    
    // Purge cached lyrics older than 30 days
    let _ = db.execute(
        "DELETE FROM lyrics_cache WHERE updated_at < ?;",
        params![now - 30 * 24 * 60 * 60],
    );
    
    Ok(rows_deleted)
}
