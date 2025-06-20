use anyhow::anyhow;
use anyhow::Ok;
use mp3lame_encoder::Builder;
use mp3lame_encoder::FlushNoGap;
use mp3lame_encoder::InterleavedPcm;
use id3::{Version, frame::{Picture, PictureType, Frame, Content}};
use bytes::Bytes;

use super::execute_with_result;
use super::EncodedStream;
use super::Encoder;
use super::Samples;

pub struct Mp3Encoder;

unsafe impl Sync for Mp3Encoder {}

impl Mp3Encoder {
    fn build_encoder(
        &self,
        sample_rate: u32,
        channels: u32,
    ) -> anyhow::Result<mp3lame_encoder::Encoder> {
        let mut builder = Builder::new().ok_or(anyhow::anyhow!("Failed to create mp3 encoder"))?;

        builder
            .set_sample_rate(sample_rate)
            .map_err(|e| anyhow::anyhow!("Failed to set sample rate for mp3 encoder: {}", e))?;
        builder.set_num_channels(channels as u8).map_err(|e| {
            anyhow::anyhow!("Failed to set number of channels for mp3 encoder: {}", e)
        })?;
        builder
            .set_brate(mp3lame_encoder::Birtate::Kbps320)
            .map_err(|e| anyhow::anyhow!("Failed to set bitrate for mp3 encoder: {}", e))?;

        builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build mp3 encoder: {}", e))
    }
}

#[async_trait::async_trait]
impl Encoder for Mp3Encoder {
    async fn encode(&self, samples: &Samples, metadata: &crate::track::TrackMetadata, cover_image_bytes: Bytes, output_path: &str) -> anyhow::Result<()> {
        let file_name = &metadata.track_name;
        tracing::info!("Writing track: {:?} to file: {}", file_name, output_path);
        let stream = self.encode_raw(samples).await?;
        stream.write_to_file(output_path).await?;

        // Embed tags using id3 crate
        let mut tag = id3::Tag::read_from_path(output_path).unwrap_or_else(|_| id3::Tag::new());
        tag.set_title(file_name);

        let artists = metadata.artists.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join("\0");
        tag.set_artist(&artists);

        tag.set_album(&metadata.album.name);

        tag.add_frame(Frame::with_content("TDRC", Content::Text(metadata.album.year.to_string())));

        // Embed cover image
        if !cover_image_bytes.is_empty() {
            let picture = Picture {
                mime_type: "image/jpeg".to_string(),
                picture_type: PictureType::CoverFront,
                description: "cover".to_string(),
                data: cover_image_bytes.to_vec(),
            };
            tag.add_frame(Frame::with_content("APIC", Content::Picture(picture)));
        }

        tag.write_to_path(output_path, Version::Id3v24)?;
        Ok(())
    }
}

impl Mp3Encoder {
    async fn encode_raw(&self, samples: &Samples) -> anyhow::Result<EncodedStream> {
        let mut mp3_encoder = self.build_encoder(samples.sample_rate, samples.channels)?;

        let (tx, rx) = tokio::sync::oneshot::channel();

        let samples_vec = samples.samples.clone();

        rayon::spawn(execute_with_result(
            move || {
                let samples: Vec<i16> = samples_vec.iter().map(|&x| x as i16).collect();
                let input = InterleavedPcm(samples.as_slice());
                let mut mp3_out_buffer = Vec::with_capacity(mp3lame_encoder::max_required_buffer_size(samples.len()));
                let encoded_size = mp3_encoder
                    .encode(input, mp3_out_buffer.spare_capacity_mut())
                    .map_err(|e| anyhow!("Failed to encode mp3: {}", e))?;
                unsafe {
                    mp3_out_buffer.set_len(mp3_out_buffer.len().wrapping_add(encoded_size));
                }

                let encoded_size = mp3_encoder
                    .flush::<FlushNoGap>(mp3_out_buffer.spare_capacity_mut())
                    .map_err(|e| anyhow!("Failed to flush mp3 encoder: {}", e))?;
                unsafe {
                    mp3_out_buffer.set_len(mp3_out_buffer.len().wrapping_add(encoded_size));
                }
                Ok(mp3_out_buffer)
            },
            tx,
        ));

        let mp3_out_buffer = rx.await??;

        Ok(EncodedStream::new(mp3_out_buffer))
    }
}
