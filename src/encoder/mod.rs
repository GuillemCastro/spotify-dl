mod flac;
#[cfg(feature = "mp3")]
mod mp3;

use std::str::FromStr;

use anyhow::Result;
use lazy_static::lazy_static;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Format {
    Flac,
    #[cfg(feature = "mp3")]
    Mp3,
}

impl FromStr for Format {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "flac" => Ok(Format::Flac),
            #[cfg(feature = "mp3")]
            "mp3" => Ok(Format::Mp3),
            _ => Err(anyhow::anyhow!("Unsupported format")),
        }
    }
}

impl Format {

    pub fn extension(&self) -> &'static str {
        match self {
            Format::Flac => "flac",
            #[cfg(feature = "mp3")]
            Format::Mp3 => "mp3",
        }
    }

}

lazy_static!(
    static ref FLAC_ENCODER : Box<dyn Encoder + Sync> = Box::new(flac::FlacEncoder::new().unwrap());
    #[cfg(feature = "mp3")]
    static ref MP3_ENCODER : Box<dyn Encoder + Sync> = Box::new(mp3::Mp3Encoder {});
);

pub fn get_encoder(format: Format) -> anyhow::Result<&'static Box<dyn Encoder + Sync>> {
    match format {
        Format::Flac => Ok(&FLAC_ENCODER),
        #[cfg(feature = "mp3")]
        Format::Mp3 => Ok(&MP3_ENCODER),
        _ => Err(anyhow::anyhow!("Unsupported format")),
    }
}

pub trait Encoder {
    fn encode(&self, samples: Samples) -> Result<EncodedStream>;
}

pub struct Samples {
    pub samples: Vec<i32>,
    pub sample_rate: u32,
    pub channels: u32,
    pub bits_per_sample: u32,
}

impl Samples {
    pub fn new(samples: Vec<i32>, sample_rate: u32, channels: u32, bits_per_sample: u32) -> Self {
        Samples {
            samples,
            sample_rate,
            channels,
            bits_per_sample,
        }
    }
}

pub struct EncodedStream {
    pub stream: Vec<u8>,
}

impl EncodedStream {
    pub fn new(stream: Vec<u8>) -> Self {
        EncodedStream { stream }
    }

    pub fn write_to_file<P: std::convert::AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        if !path.as_ref().exists() {
            std::fs::create_dir_all(
                path.as_ref()
                    .parent()
                    .ok_or(anyhow::anyhow!("Could not create path"))?,
            )?;
        }
        std::fs::write(path, &self.stream)?;
        Ok(())
    }
}
