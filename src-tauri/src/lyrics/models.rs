use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LyricLine {
    pub timestamp_ms: Option<u64>,
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Lyrics {
    pub track_id: String,
    pub title: String,
    pub artist: String,
    pub source: String,
    pub synced: bool,
    pub lines: Vec<LyricLine>,
}
