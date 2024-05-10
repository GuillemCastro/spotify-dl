use std::path::Path;

use audiotags::Tag;
use audiotags::TagType;
use librespot::playback::audio_backend::Sink;
use librespot::playback::audio_backend::SinkError;
use librespot::playback::convert::Converter;
use librespot::playback::decoder::AudioPacket;
use flac_bound::FlacEncoder;

use crate::track::TrackMetadata;

pub enum SinkEvent {
    Written { bytes: usize, total: usize },
    Finished,
}
pub type SinkEventChannel = tokio::sync::mpsc::UnboundedReceiver<SinkEvent>;

pub struct FileSink {
    sink: String,
    content: Vec<i32>,
    metadata: TrackMetadata,
    compression: u32,
    event_sender: tokio::sync::mpsc::UnboundedSender<SinkEvent>,
}

impl FileSink {
    pub fn set_compression(&mut self, compression: u32) {
        self.compression = compression;
    }

    pub fn new(path: String, track: TrackMetadata) -> (Self, SinkEventChannel) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        (
            FileSink {
                sink: path,
                content: Vec::new(),
                metadata: track,
                compression: 4,
                event_sender: tx,
            },
            rx,
        )
    }

    pub fn get_approximate_size(&self) -> usize {
        self.convert_track_duration_to_size()
    }

    fn convert_track_duration_to_size(&self) -> usize {
        let duration = self.metadata.duration / 1000;
        let sample_rate = 44100;
        let channels = 2;
        let bits_per_sample = 16;
        let bytes_per_sample = bits_per_sample / 8;
        (duration as usize) * sample_rate * channels * bytes_per_sample * 2
    }
}

impl Sink for FileSink {
    fn start(&mut self) -> Result<(), SinkError> {
        Ok(())
    }

    fn stop(&mut self) -> Result<(), SinkError> {
        tracing::info!("Writing to file: {:?}", &self.sink);
        let mut encoder = FlacEncoder::new()
            .ok_or(SinkError::OnWrite(
                "Failed to create flac encoder".to_string(),
            ))?
            .channels(2)
            .bits_per_sample(16)
            .compression_level(self.compression)
            .init_file(&self.sink)
            .map_err(|e| {
                SinkError::OnWrite(format!("Failed to init flac encoder: {:?}", e).to_string())
            })?;
        encoder
            .process_interleaved(self.content.as_slice(), (self.content.len() / 2) as u32)
            .map_err(|_| SinkError::OnWrite("Failed to write flac".to_string()))?;
        encoder
            .finish()
            .map_err(|_| SinkError::OnWrite("Failed to finish encondig".to_string()))?;

        let mut tag = Tag::new()
            .with_tag_type(TagType::Flac)
            .read_from_path(Path::new(&self.sink))
            .map_err(|_| SinkError::OnWrite("Failed to read metadata".to_string()))?;

        tag.set_album_title(&self.metadata.album.name);
        for artist in &self.metadata.artists {
            tag.add_artist(&artist.name);
        }
        tag.set_title(&self.metadata.track_name);
        tag.write_to_path(&self.sink)
            .map_err(|_| SinkError::OnWrite("Failed to write metadata".to_string()))?;

        self.event_sender
            .send(SinkEvent::Finished)
            .map_err(|_| SinkError::OnWrite("Failed to send finished event".to_string()))?;
        Ok(())
    }

    fn write(&mut self, packet: AudioPacket, converter: &mut Converter) -> Result<(), SinkError> {
        let data = converter.f64_to_s16(
            packet
                .samples()
                .map_err(|_| SinkError::OnWrite("Failed to get samples".to_string()))?,
        );
        let mut data32: Vec<i32> = data.iter().map(|el| i32::from(*el)).collect();
        self.content.append(&mut data32);

        self.event_sender
            .send(SinkEvent::Written {
                bytes: self.content.len() * std::mem::size_of::<i32>(),
                total: self.convert_track_duration_to_size(),
            })
            .map_err(|_| SinkError::OnWrite("Failed to send event".to_string()))?;

        Ok(())
    }
}
