use crate::lyrics::models::Lyrics;
use crate::lyrics::cache::{read_lyrics_cache, write_lyrics_cache};
use crate::lyrics::provider::LyricsProvider;
use crate::lyrics::lrclib::LRCLibProvider;
use tauri::AppHandle;

fn clean_youtube_metadata(title: &str, artist: &str) -> (String, String) {
    let mut cleaned_artist = artist.trim().to_string();
    let mut cleaned_title = title.trim().to_string();

    // 1. Split "Artist - Title" if present
    if cleaned_title.contains(" - ") {
        let parts: Vec<&str> = cleaned_title.splitn(2, " - ").collect();
        if parts.len() == 2 {
            cleaned_artist = parts[0].trim().to_string();
            cleaned_title = parts[1].trim().to_string();
        }
    }

    // 2. Remove hashtag text (e.g. #COLDVISIONS)
    if let Some(pos) = cleaned_title.find('#') {
        cleaned_title = cleaned_title[..pos].trim().to_string();
    }

    // 3. Remove common parentheses/bracket noise without regex
    let noise_keywords = [
        "official", "audio", "video", "lyric", "stream", "visual", "hq", 
        "hd", "version", "remaster", "music", "out now", "live", "clip", "mv"
    ];

    let mut result_title = String::new();
    let mut chars = cleaned_title.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c == '(' || c == '[' {
            let closing = if c == '(' { ')' } else { ']' };
            let mut inside = String::new();
            while let Some(&next_c) = chars.peek() {
                if next_c == closing {
                    chars.next();
                    break;
                }
                inside.push(chars.next().unwrap());
            }
            
            let inside_lower = inside.to_lowercase();
            let is_noise = noise_keywords.iter().any(|&keyword| inside_lower.contains(keyword));
            
            if !is_noise {
                result_title.push(c);
                result_title.push_str(&inside);
                result_title.push(closing);
            }
        } else {
            result_title.push(c);
        }
    }
    
    cleaned_title = result_title;

    // Clean up double spaces
    while cleaned_title.contains("  ") {
        cleaned_title = cleaned_title.replace("  ", " ");
    }
    cleaned_title = cleaned_title.trim().to_string();

    if cleaned_artist.is_empty() {
        cleaned_artist = artist.trim().to_string();
    }
    if cleaned_title.is_empty() {
        cleaned_title = title.trim().to_string();
    }

    (cleaned_title, cleaned_artist)
}

pub async fn get_lyrics_orchestrator(
    app_handle: &AppHandle,
    track_id: &str,
    title: &str,
    artist: &str,
    album: &str,
    duration_ms: u32,
) -> Result<Lyrics, String> {
    // 1. Preprocess metadata if from YouTube
    let is_youtube = track_id.len() == 11 || album == "YouTube Stream" || album == "YouTube fallback";
    let (final_title, final_artist) = if is_youtube {
        let (t, a) = clean_youtube_metadata(title, artist);
        crate::log_info!("Cleaned YouTube Metadata -> Title: '{}', Artist: '{}'", t, a);
        (t, a)
    } else {
        (title.to_string(), artist.to_string())
    };

    // 2. Check local offline cache first
    if let Some(cached_lyrics) = read_lyrics_cache(app_handle, track_id) {
        crate::log_info!("Lyrics cache hit for track: {}", track_id);
        return Ok(cached_lyrics);
    }

    // 3. Fetch via LRCLIB provider
    crate::log_info!("Lyrics cache miss. Fetching from provider for track: {}", track_id);
    let provider = LRCLibProvider::new();
    
    match provider.get_lyrics(track_id, &final_title, &final_artist, album, duration_ms).await {
        Ok(mut lyrics) => {
            // Ensure the lyrics model reflects the cleaned title and artist
            lyrics.title = final_title;
            lyrics.artist = final_artist;
            
            // Write to cache
            let _ = write_lyrics_cache(app_handle, &lyrics);
            Ok(lyrics)
        }
        Err(e) => {
            Err(e)
        }
    }
}
