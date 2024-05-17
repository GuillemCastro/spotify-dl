use flacenc::bitsink::ByteSink;
use flacenc::component::BitRepr;
use flacenc::error::Verify;

use super::execute_with_result;
use super::EncodedStream;
use super::Encoder;
use super::Samples;

#[derive(Debug)]
pub struct FlacEncoder;

impl FlacEncoder {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self)
    }
}

#[async_trait::async_trait]
impl Encoder for FlacEncoder {
    async fn encode(&self, samples: Samples) -> anyhow::Result<EncodedStream> {
        let source = flacenc::source::MemSource::from_samples(
            &samples.samples,
            samples.channels as usize,
            samples.bits_per_sample as usize,
            samples.sample_rate as usize,
        );

        let config = flacenc::config::Encoder::default()
            .into_verified()
            .map_err(|e| anyhow::anyhow!("Failed to verify encoder config: {:?}", e))?;

        let (tx, rx) = tokio::sync::oneshot::channel();

        rayon::spawn(execute_with_result(
            move || {
                let flac_stream = flacenc::encode_with_fixed_block_size(
                    &config,
                    source,
                    config.block_size,
                )
                .map_err(|e| anyhow::anyhow!("Failed to encode flac: {:?}", e))?;

                let mut byte_sink = ByteSink::new();
                flac_stream
                    .write(&mut byte_sink)
                    .map_err(|e| anyhow::anyhow!("Failed to write flac stream: {:?}", e))?;

                Ok(byte_sink.into_inner())
            },
            tx,
        ));

        let byte_sink: Vec<u8> = rx.await??;

        Ok(EncodedStream::new(byte_sink))
    }
}
