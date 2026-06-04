use rusqlite::Connection;

pub fn run_migrations(conn: &mut Connection) -> Result<(), String> {
    // Create the schema_version table if it doesn't exist
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY
        );",
        [],
    ).map_err(|e| format!("Failed to create schema_version table: {}", e))?;

    // Get current version
    let current_version: i32 = conn.query_row(
        "SELECT version FROM schema_version LIMIT 1;",
        [],
        |row| row.get(0),
    ).unwrap_or(0);

    let migrations: Vec<(i32, &str)> = vec![
        (1, "
            CREATE TABLE IF NOT EXISTS tracks (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                artist TEXT NOT NULL,
                album TEXT NOT NULL,
                duration TEXT NOT NULL,
                source TEXT NOT NULL,
                thumbnail TEXT NOT NULL,
                path TEXT
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS tracks_fts USING fts5(
                title,
                artist,
                album,
                track_id UNINDEXED,
                tokenize='unicode61'
            );
            CREATE TRIGGER IF NOT EXISTS tracks_ai AFTER INSERT ON tracks BEGIN
                INSERT INTO tracks_fts(title, artist, album, track_id)
                VALUES(new.title, new.artist, new.album, new.id);
            END;
            CREATE TRIGGER IF NOT EXISTS tracks_ad AFTER DELETE ON tracks BEGIN
                DELETE FROM tracks_fts WHERE track_id = old.id;
            END;
            CREATE TRIGGER IF NOT EXISTS tracks_au AFTER UPDATE ON tracks BEGIN
                DELETE FROM tracks_fts WHERE track_id = old.id;
                INSERT INTO tracks_fts(title, artist, album, track_id)
                VALUES(new.title, new.artist, new.album, new.id);
            END;
        "),
        (2, "
            CREATE TABLE IF NOT EXISTS lyrics_cache (
                track_id TEXT PRIMARY KEY,
                lyrics_text TEXT,
                lyrics_type TEXT,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY(track_id) REFERENCES tracks(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS app_state (
                key TEXT PRIMARY KEY,
                value TEXT,
                updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS playlists (
                name TEXT PRIMARY KEY,
                tracks TEXT,
                updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS metadata_cache (
                cache_key TEXT PRIMARY KEY,
                cache_type TEXT,
                value TEXT,
                expires_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS listening_statistics (
                track_id TEXT PRIMARY KEY,
                play_count INTEGER NOT NULL DEFAULT 0,
                skip_count INTEGER NOT NULL DEFAULT 0,
                completion_percentage REAL NOT NULL DEFAULT 0.0,
                last_played INTEGER,
                first_played INTEGER,
                total_play_time INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY(track_id) REFERENCES tracks(id) ON DELETE CASCADE
            );
        "),
        (3, "
            ALTER TABLE tracks ADD COLUMN file_hash TEXT;
        "),
        (4, "
            CREATE TABLE IF NOT EXISTS lyrics_cache_new (
                track_id TEXT PRIMARY KEY,
                lyrics_text TEXT,
                lyrics_type TEXT,
                updated_at INTEGER NOT NULL
            );
            INSERT OR IGNORE INTO lyrics_cache_new (track_id, lyrics_text, lyrics_type, updated_at)
            SELECT track_id, lyrics_text, lyrics_type, updated_at FROM lyrics_cache;
            DROP TABLE lyrics_cache;
            ALTER TABLE lyrics_cache_new RENAME TO lyrics_cache;
        "),
    ];

    for (version, sql) in migrations {
        if version > current_version {
            let tx = conn.transaction().map_err(|e| format!("Failed to start transaction for migration {}: {}", version, e))?;
            
            tx.execute_batch(sql).map_err(|e| format!("Migration {} failed: {}", version, e))?;

            // Update schema version
            if current_version == 0 {
                tx.execute("INSERT INTO schema_version (version) VALUES (?);", [version]).map_err(|e| e.to_string())?;
            } else {
                tx.execute("UPDATE schema_version SET version = ?;", [version]).map_err(|e| e.to_string())?;
            }

            tx.commit().map_err(|e| format!("Failed to commit migration {}: {}", version, e))?;
        }
    }

    Ok(())
}
