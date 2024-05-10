use std::fmt::Write;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use indicatif::MultiProgress;
use indicatif::ProgressBar;
use indicatif::ProgressState;
use indicatif::ProgressStyle;
use librespot::core::session::Session;
use librespot::playback::config::PlayerConfig;
use librespot::playback::mixer::NoOpVolume;
use librespot::playback::mixer::VolumeGetter;
use librespot::playback::player::Player;

use crate::file_sink::FileSink;
use crate::file_sink::SinkEvent;
use crate::track::Track;
use crate::track::TrackMetadata;

pub struct Downloader {
    player_config: PlayerConfig,
    session: Session,
    progress_bar: MultiProgress,
}

#[derive(Debug, Clone)]
pub struct DownloadOptions {
    pub destination: PathBuf,
    pub compression: Option<u32>,
    pub parallel: usize,
}

impl DownloadOptions {
    pub fn new(destination: Option<String>, compression: Option<u32>, parallel: usize) -> Self {
        let destination =
            destination.map_or_else(|| std::env::current_dir().unwrap(), PathBuf::from);
        DownloadOptions {
            destination,
            compression,
            parallel,
        }
    }
}

impl Downloader {
    pub fn new(session: Session) -> Self {
        Downloader {
            player_config: PlayerConfig::default(),
            session,
            progress_bar: MultiProgress::new(),
        }
    }

    pub async fn download_tracks(
        self,
        tracks: Vec<Track>,
        options: &DownloadOptions,
    ) -> Result<()> {
        let this = Arc::new(self);

        let chunks = tracks.chunks(options.parallel);
        for chunk in chunks {
            let mut tasks = Vec::new();
            for track in chunk {
                let t = track.clone();
                let downloader = this.clone();
                let options = options.clone();
                tasks.push(tokio::spawn(async move {
                    downloader.download_track(t, &options).await
                }));
            }
            for task in tasks {
                task.await??;
            }
        }

        Ok(())
    }

    #[tracing::instrument(name = "download_track", skip(self))]
    async fn download_track(&self, track: Track, options: &DownloadOptions) -> Result<()> {
        let metadata = track.metadata(&self.session).await?;
        tracing::info!("Downloading track: {:?}", metadata);

        let file_name = self.get_file_name(&metadata);
        let path = options
            .destination
            .join(file_name.clone())
            .with_extension("flac")
            .to_str()
            .ok_or(anyhow::anyhow!("Could not set the output path"))?
            .to_string();

        let (file_sink, mut sink_channel) = FileSink::new(path.to_string(), metadata);

        let file_size = file_sink.get_approximate_size();

        let (mut player, _) = Player::new(
            self.player_config.clone(),
            self.session.clone(),
            self.volume_getter(),
            move || Box::new(file_sink),
        );

        let pb = self.progress_bar.add(ProgressBar::new(file_size as u64));
        pb.set_style(ProgressStyle::with_template("{spinner:.green} {msg} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
            .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
            .progress_chars("#>-"));
        pb.set_message(file_name.clone());

        player.load(track.id, true, 0);

        let name = file_name.clone();
        tokio::spawn(async move {
            while let Some(event) = sink_channel.recv().await {
                match event {
                    SinkEvent::Written { bytes, total } => {
                        tracing::trace!("Written {} bytes out of {}", bytes, total);
                        pb.set_position(bytes as u64);
                    }
                    SinkEvent::Finished => {
                        pb.finish_with_message(format!("Downloaded {}", name));
                    }
                }
            }
        });

        player.await_end_of_track().await;
        player.stop();

        tracing::info!("Downloaded track: {:?}", file_name);
        Ok(())
    }

    fn volume_getter(&self) -> Box<dyn VolumeGetter + Send> {
        Box::new(NoOpVolume)
    }

    fn get_file_name(&self, metadata: &TrackMetadata) -> String {
        // If there is more than 3 artists, add the first 3 and add "and others" at the end
        if metadata.artists.len() > 3 {
            let artists_name = metadata
                .artists
                .iter()
                .take(3)
                .map(|artist| artist.name.clone())
                .collect::<Vec<String>>()
                .join(", ");
            return self.clean_file_name(format!(
                "{}, and others - {}",
                artists_name, metadata.track_name
            ));
        }

        let artists_name = metadata
            .artists
            .iter()
            .map(|artist| artist.name.clone())
            .collect::<Vec<String>>()
            .join(", ");
        self.clean_file_name(format!("{} - {}", artists_name, metadata.track_name))
    }

    fn clean_file_name(&self, file_name: String) -> String {
        let invalid_chars = ['<', '>', ':', '\'', '"', '/', '\\', '|', '?', '*'];
        let mut clean = String::new();

        // Everything but Windows should allow non-ascii characters
        let allows_non_ascii = !cfg!(windows);
        for c in file_name.chars() {
            if !invalid_chars.contains(&c) && (c.is_ascii() || allows_non_ascii) && !c.is_control()
            {
                clean.push(c);
            }
        }
        clean
    }
}
