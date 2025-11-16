use anyhow::Result;
use librespot::core::cache::Cache;
use librespot::core::config::SessionConfig;
use librespot::core::session::Session;
use librespot::discovery::Credentials;
use librespot::oauth::OAuthClientBuilder;

const SPOTIFY_CLIENT_ID: &str = "65b708073fc0480ea92a077233ca87bd";
const SPOTIFY_REDIRECT_URI: &str = "http://127.0.0.1:8898/login";

pub async fn create_session() -> Result<Session> {
    let credentials_store = dirs::home_dir().map(|p| p.join(".spotify-dl"));
    let cache = Cache::new(credentials_store, None, None, None)?;

    let session_config = SessionConfig::default();

    let credentials = match cache.credentials() {
        Some(creds) => creds,
        None => load_credentials()?,
    };

    cache.save_credentials(&credentials);

    let session = Session::new(session_config, Some(cache));
    session.connect(credentials, true).await?;
    Ok(session)
}

fn load_credentials() -> Result<Credentials> {
    OAuthClientBuilder::new(SPOTIFY_CLIENT_ID, SPOTIFY_REDIRECT_URI, vec!["streaming"])
        .build()
        .and_then(|client| client.get_access_token())
        .map(|token| token.access_token)
        .map(Credentials::with_access_token)
        .map_err(Into::into)
}
