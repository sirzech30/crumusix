use std::time::{Instant, Duration};
use rusqlite::Connection;

fn main() {
    println!("=========================================================");
    println!("     CRUMUSIX PERFORMANCE MICROBENCHMARK SUITE          ");
    println!("=========================================================");

    bench_fts_search();
    bench_lrc_parser();
    bench_sqlite_session();
    bench_cache_latency();
    
    println!("=========================================================");
    println!("     BENCHMARKS COMPLETED SUCCESSFULLY                   ");
    println!("=========================================================");
}

fn bench_fts_search() {
    println!("Running FTS virtual search index benchmarks...");
    let mut conn = Connection::open_in_memory().unwrap();
    
    // Initialize FTS table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tracks (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            artist TEXT NOT NULL,
            album TEXT NOT NULL,
            duration TEXT NOT NULL,
            source TEXT NOT NULL,
            thumbnail TEXT NOT NULL,
            path TEXT
        );",
        [],
    ).unwrap();
    
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS tracks_fts USING fts5(
            title,
            artist,
            album,
            track_id UNINDEXED,
            tokenize='unicode61'
        );",
        [],
    ).unwrap();
    
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS tracks_ai AFTER INSERT ON tracks BEGIN
            INSERT INTO tracks_fts(title, artist, album, track_id)
            VALUES(new.title, new.artist, new.album, new.id);
        END;",
        [],
    ).unwrap();

    // Populate mock 5,000 tracks
    let tx = conn.transaction().unwrap();
    for i in 0..5000 {
        tx.execute(
            "INSERT INTO tracks (id, title, artist, album, duration, source, thumbnail, path) 
             VALUES (?, ?, ?, ?, '3:45', 'local', 'thumb.jpg', 'path/to/song.mp3');",
            rusqlite::params![
                format!("track_{}", i),
                format!("Song Number {}", i),
                format!("Artist {}", i % 100),
                format!("Album {}", i % 200),
            ]
        ).unwrap();
    }
    tx.commit().unwrap();

    // Measure match query time
    let start = Instant::now();
    let iterations = 1000;
    for i in 0..iterations {
        let query = format!("Song Number {}*", i % 5000);
        let mut stmt = conn.prepare(
            "SELECT t.id, t.title FROM tracks_fts fts
             JOIN tracks t ON t.id = fts.track_id
             WHERE tracks_fts MATCH ? LIMIT 10;"
        ).unwrap();
        let _ = stmt.query_map([query], |row| {
            let id: String = row.get(0)?;
            let title: String = row.get(1)?;
            Ok((id, title))
        }).unwrap().count();
    }
    let duration = start.elapsed();
    println!(" FTS query speed: {:?} per query ({} iterations)", duration / iterations, iterations);
}

fn bench_lrc_parser() {
    println!("Running LRC parser timing benchmarks...");
    let lrc_data = r#"
[00:12.00] Line one of lyrics
[00:15.30] Line two of lyrics
[00:18.80] Line three of lyrics
[00:22.00] Line four of lyrics
[00:25.10] Line five of lyrics
[00:28.40] Line six of lyrics
[00:31.90] Line seven of lyrics
    "#.repeat(50); // Generates 350 lines

    let start = Instant::now();
    let iterations = 1000;
    for _ in 0..iterations {
        let mut lines = Vec::new();
        for line in lrc_data.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.contains(']') {
                let parts: Vec<&str> = trimmed.splitn(2, ']').collect();
                if parts.len() == 2 {
                    let ts_part = &parts[0][1..];
                    let text = parts[1].to_string();
                    let ms = parse_lrc_timestamp(ts_part);
                    lines.push((ms, text));
                }
            }
        }
        std::hint::black_box(lines);
    }
    let duration = start.elapsed();
    println!(" LRC parser speed: {:?} per parse ({} lines, {} iterations)", duration / iterations, 350, iterations);
}

fn parse_lrc_timestamp(ts: &str) -> Option<u64> {
    let parts: Vec<&str> = ts.split(':').collect();
    if parts.len() == 2 {
        let min: u64 = parts[0].parse().ok()?;
        let sec_parts: Vec<&str> = parts[1].split('.').collect();
        if sec_parts.len() == 2 {
            let sec: u64 = sec_parts[0].parse().ok()?;
            let ms: u64 = sec_parts[1].parse().ok()?;
            return Some((min * 60 + sec) * 1000 + ms * 10);
        }
    }
    None
}

fn bench_sqlite_session() {
    println!("Running SQLite session transaction write benchmarks...");
    let conn = Connection::open_in_memory().unwrap();
    conn.execute(
        "CREATE TABLE IF NOT EXISTS app_state (
            key TEXT PRIMARY KEY,
            value TEXT,
            updated_at INTEGER NOT NULL
        );",
        [],
    ).unwrap();

    let start = Instant::now();
    let iterations = 500;
    for i in 0..iterations {
        let session_json = format!(r#"{{"current_track_id":"track_{}","position_ms":{},"volume":0.8,"active_provider":"local","is_playing":true}}"#, i, i * 1000);
        conn.execute(
            "INSERT OR REPLACE INTO app_state (key, value, updated_at) VALUES ('session', ?, ?);",
            rusqlite::params![session_json, i],
        ).unwrap();
    }
    let duration = start.elapsed();
    println!(" Session save speed: {:?} per write ({} iterations)", duration / iterations, iterations);
}

fn bench_cache_latency() {
    println!("Running Key-Value Metadata Cache read/write latency benchmarks...");
    let conn = Connection::open_in_memory().unwrap();
    conn.execute(
        "CREATE TABLE IF NOT EXISTS metadata_cache (
            cache_key TEXT PRIMARY KEY,
            cache_type TEXT,
            value TEXT,
            expires_at INTEGER NOT NULL
        );",
        [],
    ).unwrap();

    let start = Instant::now();
    let iterations = 1000;
    for i in 0..iterations {
        let key = format!("track_cache_{}", i);
        let val = format!(r#"{{"data":{{"id":"{}","title":"Song {}","artist":"Artist","album":"Album","duration_ms":220000,"artwork_url":""}},"cached_at":1000,"expires_at":2000}}"#, key, i);
        
        // Write Latency
        conn.execute(
            "INSERT OR REPLACE INTO metadata_cache (cache_key, cache_type, value, expires_at) VALUES (?, 'track', ?, 2000);",
            rusqlite::params![key, val],
        ).unwrap();
    }
    let write_duration = start.elapsed();

    let start = Instant::now();
    for i in 0..iterations {
        let key = format!("track_cache_{}", i);
        let mut stmt = conn.prepare("SELECT value FROM metadata_cache WHERE cache_key = ?;").unwrap();
        let _val: String = stmt.query_row(rusqlite::params![key], |row| row.get(0)).unwrap();
    }
    let read_duration = start.elapsed();

    println!(" Cache write latency: {:?} per write", write_duration / iterations);
    println!(" Cache read latency:  {:?} per read", read_duration / iterations);
}
