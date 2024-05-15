use flacenc::bitsink::ByteSink;
use flacenc::component::BitRepr;
use flacenc::error::Verified;
use flacenc::error::Verify;

use super::EncodedStream;
use super::Encoder;
use super::Samples;

#[derive(Debug)]
pub struct FlacEncoder {
    config: Verified<flacenc::config::Encoder>,
}

impl FlacEncoder {
    pub fn new() -> anyhow::Result<Self> {
        let config = flacenc::config::Encoder::default()
            .into_verified()
            .map_err(|e| anyhow::anyhow!("Failed to verify encoder config: {:?}", e))?;
        Ok(FlacEncoder { config })
    }
}

impl Encoder for FlacEncoder {
    fn encode(&self, samples: Samples) -> anyhow::Result<EncodedStream> {
        let source = flacenc::source::MemSource::from_samples(
            &samples.samples,
            samples.channels as usize,
            samples.bits_per_sample as usize,
            samples.sample_rate as usize,
        );

        let flac_stream =
            flacenc::encode_with_fixed_block_size(&self.config, source, self.config.block_size)
                .map_err(|e| anyhow::anyhow!("Failed to encode flac: {:?}", e))?;

        let mut byte_sink = ByteSink::new();
        flac_stream
            .write(&mut byte_sink)
            .map_err(|e| anyhow::anyhow!("Failed to write flac stream: {:?}", e))?;

        Ok(EncodedStream::new(byte_sink.into_inner()))
    }
}
