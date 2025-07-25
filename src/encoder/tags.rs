use anyhow::Result;
use audiotags::Picture;
use audiotags::Tag;
use audiotags::TagType;
use bytes::Bytes;

use crate::encoder::Format;

pub struct Tags {
    pub title: String,
    pub artists: Vec<String>,
    pub album_title: String,
    pub album_cover: Option<Bytes>,
    pub position: Option<usize>,
}

pub async fn store_tags(path: String, tags: &Tags, format: Format) -> Result<()> {
    let tag_type = match format {
        Format::Mp3 => TagType::Id3v2,
        Format::Flac => TagType::Flac,
    };

    if format == Format::Mp3 {
        let tag = id3::Tag::new();
        tag.write_to_path(&path, id3::Version::Id3v24)?;
    }

    let mut tag = Tag::new().with_tag_type(tag_type).read_from_path(&path)?;
    tag.set_title(&tags.title);

    let artists: String = tags.artists
        .first()
        .unwrap_or(&String::new())
        .to_string();
    tag.set_artist(&artists);
    tag.set_album_title(&tags.album_title);

    if let Some(track_number) = &tags.position {
        tag.set_track_number(*track_number as u16);
    }

    if let Some(cover) = &tags.album_cover {
        tag.set_album_cover(Picture::new(
            cover.as_ref(),
            audiotags::MimeType::Jpeg,
        ));
    }

    tag.write_to_path(&path)?;
    Ok(())
}
