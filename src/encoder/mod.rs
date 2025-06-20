mod flac;
#[cfg(feature = "mp3")]
mod mp3;

use bytes::Bytes;
use std::{path::Path, str::FromStr};
use anyhow::Result;
use tokio::sync::oneshot::Sender;

use self::{flac::FlacEncoder, mp3::Mp3Encoder};

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

const FLAC_ENCODER: &FlacEncoder = &FlacEncoder;
#[cfg(feature = "mp3")]
const MP3_ENCODER: &Mp3Encoder = &Mp3Encoder;

pub fn get_encoder(format: Format) -> &'static dyn Encoder {
    match format {
        Format::Flac => FLAC_ENCODER,
        #[cfg(feature = "mp3")]
        Format::Mp3 => MP3_ENCODER,
    }
}

#[async_trait::async_trait]
pub trait Encoder: Sync {
    async fn encode(&self, samples: &Samples, metadata: &crate::track::TrackMetadata, cover_image_bytes: Bytes, output_path: &str) -> Result<()>;
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

    pub async fn write_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        if !path.as_ref().exists() {
            tokio::fs::create_dir_all(
                path.as_ref()
                    .parent()
                    .ok_or(anyhow::anyhow!("Could not create path"))?,
            ).await?;
        }
        tokio::fs::write(path, &self.stream).await?;
        Ok(())
    }
}

pub fn execute_with_result<F, T>(func: F, tx: Sender<anyhow::Result<T>>) -> impl FnOnce()
where
    F: FnOnce() -> anyhow::Result<T> + Send + 'static,
    T: Send + 'static,
{
    move || {
        let result = func();
        // Ignore the error if the receiver has been dropped
        let _ = tx.send(result);
    }
}