mod file_sink;

extern crate rpassword;

use std::{path::PathBuf};
use tokio_core::reactor::Core;

use librespot::{core::authentication::Credentials, metadata::Playlist};
use librespot::core::config::SessionConfig;
use librespot::core::session::Session;
use librespot::core::spotify_id::SpotifyId;
use librespot::playback::config::PlayerConfig;

use librespot::playback::player::Player;
use librespot::playback::audio_backend::{Open};

use librespot::metadata::{Track, Artist, Metadata};

use structopt::StructOpt;

use indicatif::{ProgressBar, ProgressStyle};

#[derive(Debug, StructOpt)]
#[structopt(name = "spotify-dl", about = "A commandline utility to download music directly from Spotify")]
struct Opt {
    #[structopt(help = "A list of Spotify URIs (songs, podcasts or playlists)")]
    tracks: Vec<String>,
    #[structopt(short = "u", long = "username", help = "Your Spotify username")]
    username: String,
    #[structopt(short = "d", long = "destination", default_value = ".", help = "The directory where the songs will be downloaded")]
    destination: String
}

fn create_session(core: &mut Core, credentials: Credentials) -> Session {
    let session_config = SessionConfig::default();
    let session = core.run(Session::connect(session_config, credentials, None, core.handle()))
        .unwrap();
    session
}

fn download_tracks(core: &mut Core, session: &Session, destination: PathBuf, tracks: Vec<SpotifyId>) {
    let player_config = PlayerConfig::default();
    let bar_style = ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} (ETA: {eta}) {msg}")
        .progress_chars("##-");
    let bar = ProgressBar::new(tracks.len() as u64);
    bar.set_style(bar_style);
    bar.enable_steady_tick(500);
    for track in tracks {
        let track_item = core.run(Track::get(&session, track)).unwrap();
        let artist_name: String;
        if track_item.artists.len() > 1 {
            let mut tmp: String = String::new();
            for artist in track_item.artists {
                let artist_item = core.run(Artist::get(&session, artist)).unwrap();
                tmp.push_str(artist_item.name.as_str());
                tmp.push_str(", ");
            }
            artist_name = String::from(tmp.trim_end_matches(", "));
        }
        else {
            artist_name = core.run(Artist::get(&session, track_item.artists[0])).unwrap().name;
        }
        let full_track_name = format!("{} - {}", artist_name, track_item.name);
        let filename = format!("{}.flac", full_track_name);
        let joined_path = destination.join(&filename);
        let path = joined_path.to_str().unwrap();
        bar.set_message(full_track_name.as_str());
        let file_sink = file_sink::FileSink::open(Some(path.to_owned()));
        let (mut player, _) = Player::new(player_config.clone(), session.clone(), None, move || {
            Box::new(file_sink)
        });
        player.load(track, true, 0);
        core.run(player.get_end_of_track_future()).unwrap();
        player.stop();
        bar.inc(1);
    }
    bar.finish();
}

fn main() {

    let opt = Opt::from_args();

    let mut core = Core::new().unwrap();

    let username = opt.username;
    let password = rpassword::read_password_from_tty(Some("Password: ")).unwrap();
    let credentials = Credentials::with_password(username, password);

    let session = create_session(&mut core, credentials.clone());

    let mut tracks: Vec<SpotifyId> = Vec::new();

    for track_url in opt.tracks {
        let track = SpotifyId::from_uri(track_url.as_str()).unwrap();
        match &track.audio_type {
            librespot::core::spotify_id::SpotifyAudioType::Track => {
                tracks.push(track);
            }
            librespot::core::spotify_id::SpotifyAudioType::Podcast => {
                tracks.push(track);
            }
            librespot::core::spotify_id::SpotifyAudioType::NonPlayable => {
                match core.run(Playlist::get(&session, track)) {
                    Ok(mut playlist) => {
                        println!("Adding all songs from playlist {} (by {}) to the queue", &playlist.name, &playlist.user);
                        tracks.append(&mut playlist.tracks);
                    }
                    Err(_) => {
                        println!("Unsupported track {}", &track_url);
                    }
                }
            }
        }
    }

    download_tracks(&mut core, &session, PathBuf::from(opt.destination), tracks);

}
