use librespot::playback::audio_backend::{Open, Sink};
use std::io::{self};

extern crate flac_bound;

use flac_bound::{FlacEncoder};

pub struct FileSink {
    sink: String,
    content: Vec<i32>
}

impl Open for FileSink {
    fn open(path: Option<String>) -> Self {
        if let Some(path) = path {
            let file = path;
            FileSink {
                sink: file,
                content: Vec::new()
            }
        } else {
            panic!();
        }
    }
}

impl Sink for FileSink {
    fn start(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn stop(&mut self) -> io::Result<()> {
        let mut encoder = FlacEncoder::new().unwrap().channels(2).bits_per_sample(16).compression_level(0).init_file(&self.sink).unwrap();
        encoder.process_interleaved(self.content.as_slice(), (self.content.len()/2) as u32).unwrap();
        encoder.finish().unwrap();
        Ok(())
    }

    fn write(&mut self, data: &[i16]) -> io::Result<()> {
        let mut input: Vec<i32> = data.iter().map(|el| i32::from(*el)).collect();
        self.content.append(&mut input);
        Ok(())
    }
}