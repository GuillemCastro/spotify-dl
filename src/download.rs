use std::fmt::Write;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use audiotags::Picture;
use audiotags::Tag;
use audiotags::TagType;
use bytes::Bytes;
use futures::StreamExt;
use futures::TryStreamExt;
use indicatif::MultiProgress;
use indicatif::ProgressBar;
use indicatif::ProgressState;
use indicatif::ProgressStyle;
use librespot::core::session::Session;
use librespot::playback::config::Bitrate;
use librespot::playback::config::PlayerConfig;
use librespot::playback::mixer::NoOpVolume;
use librespot::playback::mixer::VolumeGetter;
use librespot::playback::player::Player;

use crate::channel_sink::ChannelSink;
use crate::channel_sink::SinkEvent;
use crate::encoder::Format;
use crate::encoder::Samples;
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
    pub format: Format,
}

impl DownloadOptions {
    pub fn new(
        destination: Option<String>,
        compression: Option<u32>,
        parallel: usize,
        format: Format,
    ) -> Self {
        let destination =
            destination.map_or_else(|| std::env::current_dir().unwrap(), PathBuf::from);
        DownloadOptions {
            destination,
            compression,
            parallel,
            format,
        }
    }
}

impl Downloader {
    pub fn new(session: Session) -> Self {
        let mut config = PlayerConfig::default();
        config.bitrate = Bitrate::Bitrate320;
        Downloader {
            player_config: config,
            session,
            progress_bar: MultiProgress::new(),
        }
    }

    pub async fn download_tracks(
        self,
        tracks: Vec<Track>,
        options: &DownloadOptions,
    ) -> Result<()> {
        futures::stream::iter(tracks)
            .map(|track| self.download_track(track, options))
            .buffer_unordered(options.parallel)
            .try_collect::<Vec<_>>()
            .await?;

        Ok(())
    }

    #[tracing::instrument(name = "download_track", skip(self))]
    async fn download_track(&self, track: Track, options: &DownloadOptions) -> Result<()> {
        let metadata = track.metadata(&self.session).await?;
        tracing::info!("Downloading track: {:?}", metadata.track_name);

        let file_name = self.get_file_name(&metadata);
        let path = options
            .destination
            .join(file_name.clone())
            .with_extension(options.format.extension())
            .to_str()
            .ok_or(anyhow::anyhow!("Could not set the output path"))?
            .to_string();

        let (sink, mut sink_channel) = ChannelSink::new(metadata.clone());

        let file_size = sink.get_approximate_size();

        let player = Player::new(
            self.player_config.clone(),
            self.session.clone(),
            self.volume_getter(),
            move || Box::new(sink),
        );

        let pb = self.progress_bar.add(ProgressBar::new(file_size as u64));
        pb.enable_steady_tick(Duration::from_millis(100));
        pb.set_style(ProgressStyle::with_template("{spinner:.green} {msg} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
            .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
            .progress_chars("#>-"));
        pb.set_message(file_name.clone());

        player.load(track.id, true, 0);

        let mut samples = Vec::<i32>::new();

        tokio::spawn(async move {
            player.await_end_of_track().await;
            player.stop();
        });

        while let Some(event) = sink_channel.recv().await {
            match event {
                SinkEvent::Write {
                    bytes,
                    total,
                    mut content,
                } => {
                    tracing::trace!("Written {} bytes out of {}", bytes, total);
                    pb.set_position(bytes as u64);
                    samples.append(&mut content);
                }
                SinkEvent::Finished => {
                    tracing::info!("Finished downloading track: {:?}", file_name);
                    break;
                }
            }
        }

        tracing::info!("Encoding track: {:?}", file_name);
        pb.set_message(format!("Encoding {}", &file_name));
        let samples = Samples::new(samples, 44100, 2, 16);
        let encoder = crate::encoder::get_encoder(options.format);
        let stream = encoder.encode(samples).await?;

        pb.set_message(format!("Writing {}", &file_name));
        tracing::info!("Writing track: {:?} to file: {}", file_name, &path);
        stream.write_to_file(&path).await?;

        self.store_tags(path, &metadata, options).await?;

        pb.finish_with_message(format!("Downloaded {}", &file_name));
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

    async fn store_tags(
        &self,
        path: String,
        metadata: &TrackMetadata,
        options: &DownloadOptions,
    ) -> Result<()> {
        let tag_type = match options.format {
            Format::Mp3 => TagType::Id3v2,
            Format::Flac => TagType::Flac,
        };

        if options.format == Format::Mp3 {
            let tag = id3::Tag::new();
            tag.write_to_path(&path, id3::Version::Id3v24)?;
        }

        let mut tag = Tag::new().with_tag_type(tag_type).read_from_path(&path)?;
        tag.set_title(&metadata.track_name);

        let artists: String = metadata.artists.first()
            .map(|artist| artist.name.clone())
            .unwrap_or_default();
        tag.set_artist(&artists);
        tag.set_album_title(&metadata.album.name);

        tag.set_album_cover(Picture::new(
            self.get_cover_image(&metadata).await?.as_ref(),
            audiotags::MimeType::Jpeg,
        ));
        tag.write_to_path(&path)?;
        Ok(())
    }

    async fn get_cover_image(&self, metadata: &TrackMetadata) -> Result<Bytes> {
        match metadata.album.cover {
            Some(ref cover) => self
                .session
                .spclient()
                .get_image(&cover.id)
                .await
                .map_err(|e| anyhow::anyhow!("{:?}", e)),
            None => Err(anyhow::anyhow!("No cover art!")),
        }
    }
}
