use spotify_dl::download::{DownloadOptions, Downloader};
use spotify_dl::encoder::Format;
use spotify_dl::session::create_session;
use spotify_dl::track::get_tracks;
use structopt::StructOpt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};
use std::io::{self, Write};

#[derive(Debug, StructOpt)]
#[structopt(
    name = "spotify-dl",
    about = "A commandline utility to download music directly from Spotify"
)]
struct Opt {
    #[structopt(
        help = "A list of Spotify URIs or URLs (songs, podcasts, playlists or albums)"
    )]
    tracks: Vec<String>,
    #[structopt(short = "u", long = "username", help = "Your Spotify username")]
    username: Option<String>,
    #[structopt(short = "p", long = "password", help = "Your Spotify password")]
    password: Option<String>,
    #[structopt(
        short = "d",
        long = "destination",
        help = "The directory where the songs will be downloaded"
    )]
    destination: Option<String>,
    #[structopt(
        short = "c",
        long = "compression",
        help = "Setting the flac compression level from 0 (fastest, least compression) to
8 (slowest, most compression). A value larger than 8 will be Treated as 8. Default is 4. NOT USED."
    )]
    compression: Option<u32>,
    #[structopt(
        short = "t",
        long = "parallel",
        help = "Number of parallel downloads. Default is 5.",
        default_value = "5"
    )]
    parallel: usize,
    #[structopt(
        short = "f",
        long = "format",
        help = "The format to download the tracks in. Default is mp3.",
        default_value = "mp3"
    )]
    format: Format
}

pub fn configure_logger() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();
}

pub fn create_destination_if_required(destination: Option<String>) -> anyhow::Result<()> {
    if let Some(destination) = destination {
        if !std::path::Path::new(&destination).exists() {
            tracing::info!("Creating destination directory: {}", destination);
            std::fs::create_dir_all(destination)?;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    configure_logger();

    let mut opt = Opt::from_args();
    create_destination_if_required(opt.destination.clone())?;

    if opt.tracks.is_empty() {
        print!("Enter a Spotify URL or URI: ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();
        if input.is_empty() {
            eprintln!("No tracks provided");
            std::process::exit(1);
        }
        opt.tracks.push(input.to_string());
    }

    if opt.compression.is_some() {
        eprintln!("Compression level is not supported yet. It will be ignored.");
    }

    let user_name = opt.username.or_else(|| {
        println!("No username provided via arguments. Attempting to fetch from latest credentials cache.");
        std::fs::read_to_string("credentials.json").ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| v.get("username")?.as_str().map(|s| s.to_string()))
    });

    let session = create_session(user_name.unwrap(), opt.password).await?;
    let track = get_tracks(opt.tracks, &session).await?;

    let downloader = Downloader::new(session);
    downloader
        .download_tracks(
            track,
            &DownloadOptions::new(opt.destination, opt.compression, opt.parallel, opt.format),
        )
        .await
}
