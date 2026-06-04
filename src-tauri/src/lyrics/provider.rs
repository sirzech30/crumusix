use crate::lyrics::models::Lyrics;

pub trait LyricsProvider: Send + Sync {
    fn get_lyrics(
        &self,
        track_id: &str,
        title: &str,
        artist: &str,
        album: &str,
        duration_ms: u32,
    ) -> impl std::future::Future<Output = Result<Lyrics, String>> + Send;
}
