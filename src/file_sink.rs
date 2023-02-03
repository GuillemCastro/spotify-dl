use std::path::Path;

use audiotags::{Tag, TagType};
use librespot::{playback::{audio_backend::{Open, Sink, SinkError}, config::AudioFormat, decoder::AudioPacket, convert::Converter}};

// extern crate flac_bound;

use flac_bound::{FlacEncoder};

use crate::TrackMetadata;

pub struct FileSink {
    sink: String,
    content: Vec<i32>,
    metadata: Option<TrackMetadata>
}

impl FileSink {
    pub fn add_metadata(&mut self, meta: TrackMetadata) {
        self.metadata = Some(meta);
    }
}

impl Open for FileSink {
    fn open(path: Option<String>, _audio_format: AudioFormat) -> Self {
        let file_path = path.unwrap_or_else(|| panic!());
        FileSink {
            sink: file_path,
            content: Vec::new(),
            metadata: None
        }
    }
}

impl Sink for FileSink {
    fn start(&mut self) -> Result<(), SinkError> {
        Ok(())
    }

    fn stop(&mut self) -> Result<(), SinkError> {
        let mut encoder = FlacEncoder::new().unwrap().channels(2).bits_per_sample(16).compression_level(4).init_file(&self.sink).unwrap();
        encoder.process_interleaved(self.content.as_slice(), (self.content.len()/2) as u32).unwrap();
        encoder.finish().unwrap();

        match &self.metadata {
            Some(meta) => {
                let mut tag = Tag::new().with_tag_type(TagType::Flac).read_from_path(Path::new(&self.sink)).unwrap();

                tag.set_album_title(&meta.album);
                for artist in &meta.artists {
                    tag.add_artist(artist);
                }
                tag.set_title(&meta.track_name);
                tag.write_to_path(&self.sink).expect("Failed to write metadata");
            },
            None => (),
        }
        Ok(())
    }

    fn write(&mut self, packet: &AudioPacket, converter: &mut Converter) -> Result<(), SinkError> {
        let data = converter.f64_to_s16(packet.samples().unwrap());
        let mut data32: Vec<i32> = data.iter().map(|el| i32::from(*el)).collect();
        self.content.append(&mut data32);
        Ok(())
    }
}
