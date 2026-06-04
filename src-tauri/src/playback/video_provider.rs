use async_trait::async_trait;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VideoSearchResult {
    pub id: String,
    pub title: String,
    pub thumbnail: String,
    pub duration: String,
    pub channel: String,
}

#[async_trait]
pub trait VideoProvider: Send + Sync {
    async fn search(&self, query: &str) -> Result<Vec<VideoSearchResult>, String>;
    async fn get_stream_url(&self, video_id: &str) -> Result<String, String>;
}

pub struct YouTubeProvider;

#[async_trait]
impl VideoProvider for YouTubeProvider {
    async fn search(&self, query: &str) -> Result<Vec<VideoSearchResult>, String> {
        let client = reqwest::Client::new();
        let encoded_query = url::form_urlencoded::byte_serialize(query.as_bytes()).collect::<String>();
        let search_url = format!("https://www.youtube.com/results?search_query={}", encoded_query);

        let response = client
            .get(&search_url)
            .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .header("Accept-Language", "en-US,en;q=0.9")
            .send()
            .await
            .map_err(|e| format!("Network request failed: {}", e))?;

        let html = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response body: {}", e))?;

        // Find ytInitialData and parse the JSON object by matching curly braces dynamically
        let search_str = "ytInitialData";
        let idx = html.find(search_str).ok_or("Could not find ytInitialData in HTML")?;
        let remaining = &html[idx..];
        let start_json = remaining.find('{').ok_or("Could not find JSON start in ytInitialData")?;
        let remaining_json = &remaining[start_json..];
        
        let mut brace_count = 0;
        let mut end_idx = 0;
        let mut in_string = false;
        let mut escaped = false;
        
        for (i, c) in remaining_json.char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            match c {
                '\\' => escaped = true,
                '"' => in_string = !in_string,
                '{' if !in_string => brace_count += 1,
                '}' if !in_string => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        end_idx = i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        
        if end_idx == 0 {
            return Err("Could not find matching end brace for ytInitialData JSON".to_string());
        }
        let json_str = &remaining_json[..end_idx];

        let v: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse ytInitialData JSON: {}", e))?;

        let mut results = Vec::new();

        // Traverse the nested JSON hierarchy to extract search results
        if let Some(contents) = v["contents"]["twoColumnSearchResultsRenderer"]["primaryContents"]["sectionListRenderer"]["contents"]
            .as_array()
        {
            for section in contents {
                if let Some(items) = section["itemSectionRenderer"]["contents"].as_array() {
                    for item in items {
                        if let Some(video) = item.get("videoRenderer") {
                            let id = video["videoId"].as_str().unwrap_or("").to_string();
                            if id.is_empty() {
                                continue;
                            }

                            let title = video["title"]["runs"][0]["text"].as_str().unwrap_or("").to_string();
                            let thumbnail = video["thumbnail"]["thumbnails"][0]["url"].as_str().unwrap_or("").to_string();
                            let duration = video["lengthText"]["simpleText"].as_str().unwrap_or("").to_string();
                            let channel = video["ownerText"]["runs"][0]["text"].as_str().unwrap_or("").to_string();

                            results.push(VideoSearchResult {
                                id,
                                title,
                                thumbnail,
                                duration,
                                channel,
                            });
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    async fn get_stream_url(&self, video_id: &str) -> Result<String, String> {
        // Safe, abstract resolution interface for getting direct audio stream URLs
        // This decouples PlaybackManager from any specific scraper / parser
        Ok(format!("https://www.youtube.com/watch?v={}", video_id))
    }
}
