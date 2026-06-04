use serde::{Deserialize, Serialize};
use reqwest::Client;
use reqwest::header::AUTHORIZATION;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SpotifyPlaylist {
    pub id: String,
    pub name: String,
    pub tracks_total: u32,
    pub thumbnail: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SpotifyTrackItem {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: String,
    pub source: String,
    pub thumbnail: String,
}

#[tauri::command]
pub async fn spotify_fetch_liked_songs(
    offset: u32,
    limit: u32,
    token: String,
) -> Result<Vec<SpotifyTrackItem>, String> {
    let client = Client::new();

    let url = format!("https://api.spotify.com/v1/me/tracks?limit={}&offset={}", limit, offset);
    let response = client
        .get(&url)
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Spotify API connection error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Spotify API returned status error: {}", response.status()));
    }

    #[derive(Deserialize)]
    struct TrackObject {
        id: String,
        name: String,
        artists: Vec<ArtistObject>,
        album: AlbumObject,
        duration_ms: u64,
    }

    #[derive(Deserialize)]
    struct ArtistObject {
        name: String,
    }

    #[derive(Deserialize)]
    struct AlbumObject {
        name: String,
        images: Vec<ImageObject>,
    }

    #[derive(Deserialize)]
    struct ImageObject {
        url: String,
    }

    #[derive(Deserialize)]
    struct LikedItem {
        track: Option<TrackObject>,
    }

    #[derive(Deserialize)]
    struct LikedResponse {
        items: Vec<LikedItem>,
    }

    let data: LikedResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to deserialize Spotify tracks payload: {}", e))?;

    let mut tracks = Vec::new();
    for item in data.items {
        if let Some(track) = item.track {
            let total_sec = track.duration_ms / 1000;
            let min = total_sec / 60;
            let sec = total_sec % 60;
            let dur_str = format!("{}:{:02}", min, sec);

            let artists = track.artists
                .iter()
                .map(|a| a.name.clone())
                .collect::<Vec<String>>()
                .join(", ");

            let thumbnail = track.album.images
                .first()
                .map(|i| i.url.clone())
                .unwrap_or_else(|| "".to_string());

            tracks.push(SpotifyTrackItem {
                id: track.id,
                title: track.name,
                artist: artists,
                album: track.album.name,
                duration: dur_str,
                source: "spotify".to_string(),
                thumbnail,
            });
        }
    }

    Ok(tracks)
}

#[tauri::command]
pub async fn spotify_fetch_playlists(
    token: String,
) -> Result<Vec<SpotifyPlaylist>, String> {
    let client = Client::new();

    let response = client
        .get("https://api.spotify.com/v1/me/playlists?limit=50")
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Spotify API connection error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Spotify API returned status error: {}", response.status()));
    }

    let data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Spotify playlists JSON: {}", e))?;

    let mut playlists = Vec::new();
    if let Some(items) = data["items"].as_array() {
        for item in items {
            let id = match item["id"].as_str() {
                Some(s) => s.to_string(),
                None => continue,
            };

            let name = item["name"].as_str().unwrap_or("Unnamed Playlist").to_string();

            // Support multiple formats for tracks count
            let tracks_total = if let Some(total) = item["tracks"]["total"].as_u64() {
                total as u32
            } else if let Some(total) = item["items"]["total"].as_u64() {
                total as u32
            } else if let Some(items_arr) = item["tracks"]["items"].as_array() {
                items_arr.len() as u32
            } else if let Some(items_arr) = item["items"].as_array() {
                items_arr.len() as u32
            } else {
                0
            };

            let mut thumbnail = String::new();
            if let Some(images) = item["images"].as_array() {
                if let Some(first_img) = images.first() {
                    if let Some(url) = first_img["url"].as_str() {
                        thumbnail = url.to_string();
                    }
                }
            }

            playlists.push(SpotifyPlaylist {
                id,
                name,
                tracks_total,
                thumbnail,
            });
        }
    }

    Ok(playlists)
}

#[tauri::command]
pub async fn spotify_fetch_playlist_tracks(
    playlist_id: String,
    token: String,
) -> Result<Vec<SpotifyTrackItem>, String> {
    let client = Client::new();
    let is_system = playlist_id.starts_with("37i9dQZF");
    let url = if is_system {
        format!("https://api.spotify.com/v1/playlists/{}/tracks?limit=100", playlist_id)
    } else {
        format!("https://api.spotify.com/v1/playlists/{}", playlist_id)
    };

    let response = client
        .get(&url)
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Spotify API connection error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Spotify API returned status error: {}", response.status()));
    }

    let data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Spotify playlist JSON: {}", e))?;

    let mut tracks = Vec::new();
    let items_val = if data["tracks"]["items"].is_array() {
        &data["tracks"]["items"]
    } else if data["items"]["items"].is_array() {
        &data["items"]["items"]
    } else {
        &data["items"]
    };

    if let Some(items) = items_val.as_array() {
        for item in items {
            let mut track = &item["track"];
            if track.is_null() {
                track = &item["item"];
            }
            if track.is_null() {
                continue;
            }

            let id = match track["id"].as_str() {
                Some(s) => s.to_string(),
                None => continue,
            };

            let name = match track["name"].as_str() {
                Some(s) => s.to_string(),
                None => continue,
            };

            let duration_ms = track["duration_ms"].as_u64().unwrap_or(0);
            let total_sec = duration_ms / 1000;
            let min = total_sec / 60;
            let sec = total_sec % 60;
            let dur_str = format!("{}:{:02}", min, sec);

            let mut artist_names = Vec::new();
            if let Some(artists) = track["artists"].as_array() {
                for artist in artists {
                    if let Some(art_name) = artist["name"].as_str() {
                        artist_names.push(art_name.to_string());
                    }
                }
            }
            let artist = artist_names.join(", ");

            let album_name = track["album"]["name"].as_str().unwrap_or("Unknown Album").to_string();

            let mut thumbnail = String::new();
            if let Some(images) = track["album"]["images"].as_array() {
                if let Some(first_img) = images.first() {
                    if let Some(url) = first_img["url"].as_str() {
                        thumbnail = url.to_string();
                    }
                }
            }

            tracks.push(SpotifyTrackItem {
                id,
                title: name,
                artist,
                album: album_name,
                duration: dur_str,
                source: "spotify".to_string(),
                thumbnail,
            });
        }
    }

    Ok(tracks)
}

