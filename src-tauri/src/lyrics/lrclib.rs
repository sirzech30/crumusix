use crate::lyrics::models::{Lyrics, LyricLine};
use crate::lyrics::provider::LyricsProvider;
use crate::lyrics::parser::parse_lrc;
use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct LRCLibGetResponse {
    plain_lyrics: Option<String>,
    synced_lyrics: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct LRCLibSearchResponse {
    plain_lyrics: Option<String>,
    synced_lyrics: Option<String>,
}

pub struct LRCLibProvider {
    client: Client,
}

impl LRCLibProvider {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }
}

impl LyricsProvider for LRCLibProvider {
    async fn get_lyrics(
        &self,
        track_id: &str,
        title: &str,
        artist: &str,
        album: &str,
        duration_ms: u32,
    ) -> Result<Lyrics, String> {
        let duration_secs = duration_ms / 1000;
        
        let enc_artist = url::form_urlencoded::byte_serialize(artist.as_bytes()).collect::<String>();
        let enc_title = url::form_urlencoded::byte_serialize(title.as_bytes()).collect::<String>();
        let enc_album = url::form_urlencoded::byte_serialize(album.as_bytes()).collect::<String>();

        let url = format!(
            "https://lrclib.net/api/get?artist_name={}&track_name={}&album_name={}&duration={}",
            enc_artist,
            enc_title,
            enc_album,
            duration_secs
        );

        crate::log_info!("Fetching lyrics from LRCLIB: {}", url);

        let response = self.client.get(&url)
            .header("User-Agent", "CrumusiX Desktop Player (v2.0)")
            .send()
            .await
            .map_err(|e| format!("LRCLIB request failed: {}", e))?;

        if response.status().is_success() {
            let data: LRCLibGetResponse = response.json()
                .await
                .map_err(|e| format!("Failed to parse LRCLIB response: {}", e))?;

            if let Some(synced) = data.synced_lyrics {
                return Ok(Lyrics {
                    track_id: track_id.to_string(),
                    title: title.to_string(),
                    artist: artist.to_string(),
                    source: "lrclib".to_string(),
                    synced: true,
                    lines: parse_lrc(&synced),
                });
            } else if let Some(plain) = data.plain_lyrics {
                return Ok(Lyrics {
                    track_id: track_id.to_string(),
                    title: title.to_string(),
                    artist: artist.to_string(),
                    source: "lrclib".to_string(),
                    synced: false,
                    lines: plain.lines().map(|line| LyricLine {
                        timestamp_ms: None,
                        text: line.to_string(),
                    }).collect(),
                });
            }
        }

        // Fallback: search query
        let search_query = format!("{} {}", artist, title);
        let enc_query = url::form_urlencoded::byte_serialize(search_query.as_bytes()).collect::<String>();
        let search_url = format!(
            "https://lrclib.net/api/search?q={}",
            enc_query
        );
        
        crate::log_info!("LRCLIB direct fetch missed. Searching database via: {}", search_url);
        
        let search_response = self.client.get(&search_url)
            .header("User-Agent", "CrumusiX Desktop Player (v2.0)")
            .send()
            .await
            .map_err(|e| format!("LRCLIB search request failed: {}", e))?;

        if search_response.status().is_success() {
            let results: Vec<LRCLibSearchResponse> = search_response.json()
                .await
                .unwrap_or_else(|_| Vec::new());

            if let Some(best_match) = results.first() {
                if let Some(ref synced) = best_match.synced_lyrics {
                    return Ok(Lyrics {
                        track_id: track_id.to_string(),
                        title: title.to_string(),
                        artist: artist.to_string(),
                        source: "lrclib_search".to_string(),
                        synced: true,
                        lines: parse_lrc(synced),
                    });
                } else if let Some(ref plain) = best_match.plain_lyrics {
                    return Ok(Lyrics {
                        track_id: track_id.to_string(),
                        title: title.to_string(),
                        artist: artist.to_string(),
                        source: "lrclib_search".to_string(),
                        synced: false,
                        lines: plain.lines().map(|line| LyricLine {
                            timestamp_ms: None,
                            text: line.to_string(),
                        }).collect(),
                    });
                }
            }
        }

        Err("No lyrics found in LRCLIB".to_string())
    }
}
