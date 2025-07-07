use anyhow::Result;
use librespot::core::cache::Cache;
use librespot::core::config::SessionConfig;
use librespot::core::session::Session;
use librespot::discovery::Credentials;
use librespot_oauth::get_access_token;

const SPOTIFY_CLIENT_ID: &str = "65b708073fc0480ea92a077233ca87bd";
const SPOTIFY_REDIRECT_URI: &str = "http://127.0.0.1:8898/login";

pub async fn create_session() -> Result<Session> {
    let credentials_store = dirs::home_dir().map(|p| p.join(".spotify-dl"));
    let cache = Cache::new(credentials_store, None, None, None)?;

    let session_config = SessionConfig::default();

    let credentials = match cache.credentials() {
        Some(creds) => creds,
        None => match load_credentials() {
            Ok(creds) => creds,
            Err(e) => return Err(e),
        },
    };
   
    cache.save_credentials(&credentials);

    let session = Session::new(session_config, Some(cache));
    session.connect(credentials, true).await?;
    Ok(session)
}

pub fn load_credentials() -> Result<Credentials> {
    let token = match get_access_token(SPOTIFY_CLIENT_ID, SPOTIFY_REDIRECT_URI, vec!["streaming"]) {
        Ok(token) => token,
        Err(e) => return Err(e.into()),
    };
    Ok(Credentials::with_access_token(token.access_token))
}