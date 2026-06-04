use tauri::AppHandle;
use crate::cache::storage::get_cache_dir;

pub async fn get_cached_artwork(app_handle: &AppHandle, url: &str, identifier: &str) -> Result<String, String> {
    if url.is_empty() {
        return Ok("".to_string());
    }

    let artwork_dir = get_cache_dir(app_handle).join("artwork");
    std::fs::create_dir_all(&artwork_dir).ok();

    let safe_id = identifier.replace(|c: char| !c.is_alphanumeric() && c != '_' && c != '-', "");
    let filename = format!("{}.webp", safe_id);
    let local_path = artwork_dir.join(filename);

    if local_path.exists() {
        return Ok(local_path.to_string_lossy().to_string());
    }

    match reqwest::get(url).await {
        Ok(response) => {
            if let Ok(bytes) = response.bytes().await {
                if std::fs::write(&local_path, bytes).is_ok() {
                    return Ok(local_path.to_string_lossy().to_string());
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to download artwork from {}: {}", url, e);
        }
    }

    Ok(url.to_string())
}
