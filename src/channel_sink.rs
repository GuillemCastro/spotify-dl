use librespot::playback::audio_backend::Sink;
use librespot::playback::audio_backend::SinkError;
use librespot::playback::convert::Converter;
use librespot::playback::decoder::AudioPacket;

use crate::track::TrackMetadata;

pub enum SinkEvent {
    Write { bytes: usize, total: usize, content: Vec<i32> },
    Finished,
}
pub type SinkEventChannel = tokio::sync::mpsc::UnboundedReceiver<SinkEvent>;

pub struct ChannelSink {
    sender: tokio::sync::mpsc::UnboundedSender<SinkEvent>,
    bytes_total: usize,
    bytes_sent: usize,
}

impl ChannelSink {

    pub fn new(track: &TrackMetadata) -> (Self, SinkEventChannel) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        (
            ChannelSink {
                sender: tx,
                bytes_sent: 0,
                bytes_total: Self::convert_track_duration_to_size(track),
            },
            rx,
        )
    }

    fn convert_track_duration_to_size(metadata: &TrackMetadata) -> usize {
        let duration = metadata.duration / 1000;
        let sample_rate = 44100;
        let channels = 2;
        let bits_per_sample = 16;
        let bytes_per_sample = bits_per_sample / 8;
        (duration as usize) * sample_rate * channels * bytes_per_sample * 2
    }

    pub fn get_approximate_size(&self) -> usize {
        self.bytes_total
    }
}

impl Sink for ChannelSink {
    fn start(&mut self) -> Result<(), SinkError> {
        Ok(())
    }

    fn stop(&mut self) -> Result<(), SinkError> {
        tracing::info!("Finished sending song");

        self.sender
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
        let data32: Vec<i32> = data.iter().map(|el| i32::from(*el)).collect();
        self.bytes_sent += data32.len() * std::mem::size_of::<i32>();

        self.sender
            .send(SinkEvent::Write {
                bytes: self.bytes_sent,
                total: self.bytes_total,
                content: data32,
            })
            .map_err(|_| SinkError::OnWrite("Failed to send event".to_string()))?;

        Ok(())
    }
}
