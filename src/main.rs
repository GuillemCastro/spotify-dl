use spotify_dl::download::{DownloadOptions, Downloader};
use spotify_dl::session::create_session;
use spotify_dl::track::get_tracks;
use structopt::StructOpt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Debug, StructOpt)]
#[structopt(
    name = "spotify-dl",
    about = "A commandline utility to download music directly from Spotify"
)]
struct Opt {
    #[structopt(
        help = "A list of Spotify URIs or URLs (songs, podcasts, playlists or albums)",
        required = true
    )]
    tracks: Vec<String>,
    #[structopt(short = "u", long = "username", help = "Your Spotify username")]
    username: String,
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
8 (slowest, most compression). A value larger than 8 will be Treated as 8. Default is 4."
    )]
    compression: Option<u32>,
    #[structopt(
        short = "t",
        long = "parallel",
        help = "Number of parallel downloads. Default is 5.",
        default_value = "5"
    )]
    parallel: usize,
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

    let opt = Opt::from_args();
    create_destination_if_required(opt.destination.clone())?;

    if opt.tracks.is_empty() {
        eprintln!("No tracks provided");
        std::process::exit(1);
    }

    let session = create_session(opt.username, opt.password).await?;

    let track = get_tracks(opt.tracks, &session).await?;

    let downloader = Downloader::new(session);
    downloader
        .download_tracks(
            track,
            &DownloadOptions::new(opt.destination, opt.compression, opt.parallel),
        )
        .await
}
