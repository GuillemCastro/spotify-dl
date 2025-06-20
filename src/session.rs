use anyhow::Result;
use librespot::core::cache::Cache;
use librespot::core::config::SessionConfig;
use librespot::core::session::Session;
use librespot::discovery::Credentials;

pub async fn create_session(username: String, password: Option<String>) -> Result<Session> {
    let credentials_store = dirs::home_dir().map(|p| p.join(".spotify-dl"));
    let cache = Cache::new(credentials_store, None, None, None)?;

    let session_config = SessionConfig::default();
    let credentials = get_credentials(username, password, &cache);

    cache.save_credentials(&credentials);

    let session = Session::new(session_config, Some(cache));
    session.connect(credentials, false).await?;
    Ok(session)
}

fn prompt_password() -> Result<String> {
    tracing::info!("Spotify password was not provided. Please enter your Spotify password below");
    rpassword::prompt_password("Password: ").map_err(|e| e.into())
}

fn get_credentials(username: String, password: Option<String>, cache: &Cache) -> Credentials {
    match password {
        Some(password) => Credentials::with_password(username, password),
        None => cache.credentials().unwrap_or_else(|| {
            tracing::warn!("No credentials found in cache");
            Credentials::with_password(
                username,
                prompt_password().unwrap_or_else(|_| panic!("Failed to get password")),
            )
        }),
    }
}
