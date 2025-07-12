use spotify_dl::download::{DownloadOptions, Downloader};
use spotify_dl::encoder::Format;
use spotify_dl::log;
use spotify_dl::session::create_session;
use spotify_dl::track::get_tracks;
use structopt::StructOpt;

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
    #[structopt(
        short = "d",
        long = "destination",
        help = "The directory where the songs will be downloaded"
    )]
    destination: Option<String>,
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
        help = "The format to download the tracks in. Default is flac.",
        default_value = "flac"
    )]
    format: Format,
    #[structopt(
        short = "F",
        long = "force",
        help = "Force download even if the file already exists"
    )]
    force: bool,
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
    log::configure_logger()?;

    let opt = Opt::from_args();
    create_destination_if_required(opt.destination.clone())?;

    if opt.tracks.is_empty() {
        eprintln!("No tracks provided");
        std::process::exit(1);
    }

    let session = create_session().await?;

    let track = get_tracks(opt.tracks, &session).await?;

    let downloader = Downloader::new(session);
    downloader
        .download_tracks(
            track,
            &DownloadOptions::new(opt.destination, opt.parallel, opt.format, opt.force),
        )
        .await
}
