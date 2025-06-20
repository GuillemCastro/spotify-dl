use flacenc::component::BitRepr;
use flacenc::error::Verify;

use bytes::Bytes;
use super::Encoder;
use super::Samples;

#[derive(Debug)]
pub struct FlacEncoder;

#[async_trait::async_trait]
impl Encoder for FlacEncoder {
    async fn encode(&self, samples: &Samples, metadata: &crate::track::TrackMetadata, cover_image_bytes: Bytes, output_path: &str) -> anyhow::Result<()> {

        if !cover_image_bytes.is_empty() {
            tracing::info!("Cover image found but not implemented in flac encoder");
        }

        let file_name = &metadata.track_name;
        tracing::info!("Writing track: {:?} to file: {}", file_name, output_path);
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

        rayon::spawn(super::execute_with_result(
            move || {
                let flac_stream = flacenc::encode_with_fixed_block_size(
                    &config,
                    source,
                    config.block_size,
                )
                .map_err(|e| anyhow::anyhow!("Failed to encode flac: {:?}", e))?;

                let mut byte_sink = flacenc::bitsink::ByteSink::new();
                flac_stream
                    .write(&mut byte_sink)
                    .map_err(|e| anyhow::anyhow!("Failed to write flac stream: {:?}", e))?;

                Ok(byte_sink.into_inner())
            },
            tx,
        ));

        let byte_sink: Vec<u8> = rx.await??;

        let stream = super::EncodedStream::new(byte_sink);
        stream.write_to_file(output_path).await
    }
}
