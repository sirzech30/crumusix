use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Arc;
use parking_lot::Mutex;
use crate::Track;
use crate::cache::migrations::run_migrations;

pub struct SqliteDbState {
    pub conn: Arc<Mutex<Connection>>,
}

pub fn init_sqlite(db_path: &Path) -> Result<Connection, String> {
    match init_sqlite_internal(db_path) {
        Ok(conn) => Ok(conn),
        Err(err) => {
            crate::log_error!("Database initialization failed: {}. Attempting self-healing recovery...", err);
            
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            if db_path.exists() {
                let corrupted_path = db_path.with_extension(format!("db.corrupted.{}", now));
                let _ = std::fs::rename(db_path, &corrupted_path);
                
                let shm_path = db_path.with_extension("db-shm");
                if shm_path.exists() {
                    let _ = std::fs::rename(&shm_path, shm_path.with_extension(format!("db-shm.corrupted.{}", now)));
                }
                
                let wal_path = db_path.with_extension("db-wal");
                if wal_path.exists() {
                    let _ = std::fs::rename(&wal_path, wal_path.with_extension(format!("db-wal.corrupted.{}", now)));
                }
            }
            
            init_sqlite_internal(db_path).map_err(|e| {
                format!("Failed to initialize clean database after recovery attempt: {}", e)
            })
        }
    }
}

fn init_sqlite_internal(db_path: &Path) -> Result<Connection, String> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create database directory: {}", e))?;
    }
    let mut conn = Connection::open(db_path).map_err(|e| format!("Failed to open SQLite: {}", e))?;
    
    // Enable WAL mode for high concurrency & speed
    let _ = conn.execute("PRAGMA journal_mode=WAL;", []);
    
    // Enable SQLite foreign keys for cascade deletes relational integrity
    let _ = conn.execute("PRAGMA foreign_keys=ON;", []);
    
    // Configure SQLite busy timeout to 5 seconds to gracefully queue concurrent write/read operations
    let _ = conn.execute("PRAGMA busy_timeout = 5000;", []);
    
    // Run transactional sequential migrations
    run_migrations(&mut conn)?;
    
    Ok(conn)
}

pub fn set_app_state(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    conn.execute(
        "INSERT OR REPLACE INTO app_state (key, value, updated_at) VALUES (?, ?, ?);",
        params![key, value, now],
    ).map_err(|e| format!("Failed to set app_state {}: {}", key, e))?;
    Ok(())
}

pub fn get_app_state(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    let mut stmt = conn.prepare("SELECT value FROM app_state WHERE key = ?;").map_err(|e| e.to_string())?;
    let mut rows = stmt.query(params![key]).map_err(|e| e.to_string())?;
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let value: String = row.get(0).map_err(|e| e.to_string())?;
        Ok(Some(value))
    } else {
        Ok(None)
    }
}

pub fn delete_app_state(conn: &Connection, key: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM app_state WHERE key = ?;",
        params![key],
    ).map_err(|e| format!("Failed to delete app_state {}: {}", key, e))?;
    Ok(())
}

pub fn search_tracks(conn: &Connection, search_query: &str) -> Result<Vec<Track>, String> {
    if search_query.trim().is_empty() {
        return Ok(Vec::new());
    }
    
    let clean_query = search_query.replace("'", "''");
    let fts_query = format!("{}*", clean_query);
    
    let mut stmt = conn.prepare(
        "SELECT t.id, t.title, t.artist, t.album, t.duration, t.source, t.thumbnail 
         FROM tracks_fts fts
         JOIN tracks t ON t.id = fts.track_id
         WHERE tracks_fts MATCH ?
         LIMIT 50;"
    ).map_err(|e| e.to_string())?;
    
    let rows = stmt.query_map(params![fts_query], |row| {
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
    
    if results.is_empty() {
        let like_query = format!("%{}%", clean_query);
        let mut stmt = conn.prepare(
            "SELECT id, title, artist, album, duration, source, thumbnail 
             FROM tracks
             WHERE title LIKE ? OR artist LIKE ? OR album LIKE ?
             LIMIT 50;"
        ).map_err(|e| e.to_string())?;
        
        let rows = stmt.query_map(params![like_query, like_query, like_query], |row| {
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
        
        for r in rows {
            if let Ok(track) = r {
                results.push(track);
            }
        }
    }
    
    Ok(results)
}
