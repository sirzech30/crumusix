use std::sync::Mutex;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use sha2::{Digest, Sha256};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use tauri::State;

#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct SpotifySession {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<u64>,
    pub client_id: Option<String>,
}

pub struct SharedSpotifySession(pub Mutex<SpotifySession>);

// Active PKCE memory state to secure the login handshake
struct PkceState {
    code_verifier: String,
}

static PKCE_STORE: Mutex<Option<PkceState>> = Mutex::new(None);

/// Generates the Spotify PKCE Auth URL and saves the verifier in Rust memory
#[tauri::command]
pub fn spotify_generate_auth_url(client_id: String) -> Result<String, String> {
    // 1. Generate code verifier: 43 random alphanumeric characters
    let code_verifier: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(43)
        .map(char::from)
        .collect();

    // 2. Generate SHA-256 digest
    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let digest = hasher.finalize();

    // 3. Base64URL encode without padding
    let code_challenge = URL_SAFE_NO_PAD.encode(digest);

    // Save PKCE verifier for the exchange phase
    let mut store = PKCE_STORE.lock().map_err(|e| format!("Failed to lock PKCE store: {}", e))?;
    *store = Some(PkceState {
        code_verifier: code_verifier.clone(),
    });

    // 4. Construct Spotify auth URL
    let redirect_uri = "http://127.0.0.1:8888";
    let scopes = "streaming user-read-playback-state user-modify-playback-state user-read-email user-read-private user-library-read playlist-read-private playlist-read-collaborative user-top-read";
    
    let auth_url = format!(
        "https://accounts.spotify.com/authorize?client_id={}&response_type=code&redirect_uri={}&scope={}&code_challenge_method=S256&code_challenge={}",
        client_id,
        url::form_urlencoded::byte_serialize(redirect_uri.as_bytes()).collect::<String>(),
        url::form_urlencoded::byte_serialize(scopes.as_bytes()).collect::<String>(),
        code_challenge
    );

    Ok(auth_url)
}

/// Exchanges the captured OAuth code for full access/refresh tokens
#[tauri::command]
pub async fn spotify_exchange_code(
    code: String,
    client_id: String,
    session_state: State<'_, SharedSpotifySession>,
) -> Result<SpotifySession, String> {
    let verifier = {
        let mut store = PKCE_STORE.lock().map_err(|e| format!("Failed to lock PKCE store: {}", e))?;
        store.take().ok_or_else(|| "No active PKCE flow found. Please restart authentication.".to_string())?.code_verifier
    };

    let client = Client::new();
    let redirect_uri = "http://127.0.0.1:8888";

    let params = [
        ("client_id", client_id.as_str()),
        ("grant_type", "authorization_code"),
        ("code", code.as_str()),
        ("redirect_uri", redirect_uri),
        ("code_verifier", verifier.as_str()),
    ];

    let response = client
        .post("https://accounts.spotify.com/api/token")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Network error exchanging Spotify code: {}", e))?;

    if !response.status().is_success() {
        let err_text = response.text().await.unwrap_or_default();
        return Err(format!("Spotify returned error response: {}", err_text));
    }

    #[derive(Deserialize)]
    struct TokenResponse {
        access_token: String,
        refresh_token: Option<String>,
        expires_in: u64,
    }

    let data: TokenResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse token response: {}", e))?;

    let now_sec = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("System clock error: {}", e))?
        .as_secs();

    let mut session = session_state.0.lock().map_err(|e| format!("Failed to lock session: {}", e))?;
    session.access_token = Some(data.access_token.clone());
    if let Some(ref_token) = data.refresh_token {
        session.refresh_token = Some(ref_token);
    }
    session.expires_at = Some(now_sec + data.expires_in);
    session.client_id = Some(client_id);

    Ok(session.clone())
}

/// Refreshes the Spotify access token dynamically if it is expired
pub async fn spotify_refresh_token_internal(
    session_state: &SharedSpotifySession,
) -> Result<String, String> {
    let (refresh_token, client_id) = {
        let session = session_state.0.lock().map_err(|e| format!("Failed to lock session: {}", e))?;
        let ref_token = session.refresh_token.clone().ok_or_else(|| "No refresh token available".to_string())?;
        let cid = session.client_id.clone().ok_or_else(|| "No client id available".to_string())?;
        (ref_token, cid)
    };

    let client = Client::new();
    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token.as_str()),
        ("client_id", client_id.as_str()),
    ];

    let response = client
        .post("https://accounts.spotify.com/api/token")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Failed to send refresh request: {}", e))?;

    if !response.status().is_success() {
        let err_text = response.text().await.unwrap_or_default();
        return Err(format!("Spotify refresh error response: {}", err_text));
    }

    #[derive(Deserialize)]
    struct RefreshResponse {
        access_token: String,
        refresh_token: Option<String>,
        expires_in: u64,
    }

    let data: RefreshResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse refresh payload: {}", e))?;

    let now_sec = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("System clock error: {}", e))?
        .as_secs();

    let mut session = session_state.0.lock().map_err(|e| format!("Failed to lock session: {}", e))?;
    session.access_token = Some(data.access_token.clone());
    if let Some(ref_token) = data.refresh_token {
        session.refresh_token = Some(ref_token);
    }
    session.expires_at = Some(now_sec + data.expires_in);

    Ok(data.access_token)
}

/// Retrieves the current Spotify session status or triggers a refresh if needed
#[tauri::command]
pub async fn spotify_get_session(
    session_state: State<'_, SharedSpotifySession>,
) -> Result<SpotifySession, String> {
    let now_sec = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("System clock error: {}", e))?
        .as_secs();

    let needs_refresh = {
        let session = session_state.0.lock().map_err(|e| format!("Failed to lock session: {}", e))?;
        if session.access_token.is_none() {
            false
        } else if let Some(exp) = session.expires_at {
            now_sec >= exp.saturating_sub(60) // Refresh if within 1 minute of expiration
        } else {
            true
        }
    };

    if needs_refresh {
        spotify_refresh_token_internal(&session_state).await.ok();
    }

    let session = session_state.0.lock().map_err(|e| format!("Failed to lock session: {}", e))?;
    Ok(session.clone())
}

/// Dynamic injection helper to set active credentials
#[tauri::command]
pub fn spotify_inject_session(
    session: SpotifySession,
    session_state: State<'_, SharedSpotifySession>,
) -> Result<(), String> {
    let mut store = session_state.0.lock().map_err(|e| format!("Failed to lock session: {}", e))?;
    *store = session;
    Ok(())
}

/// Clear current Spotify login session
#[tauri::command]
pub fn spotify_logout_session(
    session_state: State<'_, SharedSpotifySession>,
) -> Result<(), String> {
    let mut session = session_state.0.lock().map_err(|e| format!("Failed to lock session: {}", e))?;
    session.access_token = None;
    session.refresh_token = None;
    session.expires_at = None;
    Ok(())
}
