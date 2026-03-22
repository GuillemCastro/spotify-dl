use std::pin::Pin;
use std::sync::Arc;

use anyhow::Result;
use bytes::Bytes;
use lazy_static::lazy_static;
use librespot::core::session::Session;
use librespot::core::SpotifyUri;
use librespot::core::spotify_id::SpotifyId;
use librespot::metadata::Metadata;
use librespot::metadata::image::Image;
use regex::Regex;

use crate::encoder::tags::Tags;
use crate::utils::clean_invalid_characters;

pub type AsyncFn<T> =
    Arc<dyn Fn() -> Pin<Box<dyn Future<Output = Option<T>> + Send>> + Send + Sync>;

#[async_trait::async_trait]
trait TrackCollection {
    async fn get_tracks(&self, session: &Session) -> Vec<Track>;
}

#[tracing::instrument(name = "get_tracks", skip(session), level = "debug")]
pub async fn get_tracks(spotify_ids: Vec<String>, session: &Session) -> Result<Vec<Track>> {
    let mut tracks: Vec<Track> = Vec::new();
    for id in spotify_ids {
        tracing::debug!("Getting tracks for: {}", id);
        let id = parse_uri_or_url(&id).ok_or(anyhow::anyhow!("Invalid track"))?;
        let new_tracks = match &id {
            SpotifyUri::Track { .. } | SpotifyUri::Episode { .. } => vec![Track::from_id(id.clone())],
            SpotifyUri::Album { .. } => Album::from_id(id.clone()).get_tracks(session).await,
            SpotifyUri::Playlist { .. } => Playlist::from_id(id.clone()).get_tracks(session).await,
            other => {
                tracing::warn!("Unsupported item type: {:?}", other);
                vec![]
            }
        };
        tracks.extend(new_tracks);
    }
    tracing::debug!("Got tracks: {:?}", tracks);
    Ok(tracks)
}

fn parse_uri_or_url(track: &str) -> Option<SpotifyUri> {
    parse_uri(track).or_else(|| parse_url(track))
}

fn parse_uri(track_uri: &str) -> Option<SpotifyUri> {
    if !track_uri.starts_with("spotify:") {
        return None;
    }
    let parts: Vec<&str> = track_uri.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let kind = parts[1];
    let id_str = parts[2];
    let sid = SpotifyId::from_base62(id_str).ok()?;
    let uri = match kind {
        "track" => SpotifyUri::Track { id: sid },
        "episode" => SpotifyUri::Episode { id: sid },
        "album" => SpotifyUri::Album { id: sid },
        "playlist" => SpotifyUri::Playlist { user: None, id: sid },
        _ => return None,
    };
    tracing::info!("Parsed URI: {:?}", uri);
    Some(uri)
}

fn parse_url(track_url: &str) -> Option<SpotifyUri> {
    let results = SPOTIFY_URL_REGEX.captures(track_url)?;
    let kind = results.get(1)?.as_str();
    let id_str = results.get(2)?.as_str();
    let sid = SpotifyId::from_base62(id_str).ok()?;
    match kind {
        "track" => Some(SpotifyUri::Track { id: sid }),
        "episode" => Some(SpotifyUri::Episode { id: sid }),
        "album" => Some(SpotifyUri::Album { id: sid }),
        "playlist" => Some(SpotifyUri::Playlist { user: None, id: sid }),
        _ => None,
    }
}

#[derive(Clone, Debug)]
pub struct Track {
    pub id: SpotifyUri,
}

lazy_static! {
    static ref SPOTIFY_URL_REGEX: Regex =
        Regex::new(r"https://open\.spotify\.com(?:/intl-[a-z]{2})?/(\w+)/([a-zA-Z0-9]+)").unwrap();
}

impl Track {
    pub fn new(track: &str) -> Result<Self> {
        let id = parse_uri_or_url(track).ok_or(anyhow::anyhow!("Invalid track"))?;
        Ok(Track { id })
    }

    pub fn from_id(id: SpotifyUri) -> Self {
        Track { id }
    }

    pub async fn metadata(&self, session: &Session) -> Result<TrackMetadata> {
        let metadata = librespot::metadata::Track::get(session, &self.id)
            .await
            .map_err(|_| anyhow::anyhow!("Failed to get metadata"))?;

        let mut artists = Vec::new();
        for artist in metadata.artists.iter() {
            artists.push(
                librespot::metadata::Artist::get(session, &artist.id)
                    .await
                    .map_err(|_| anyhow::anyhow!("Failed to get artist"))?,
            );
        }

        let album = librespot::metadata::Album::get(session, &metadata.album.id)
            .await
            .map_err(|_| anyhow::anyhow!("Failed to get album"))?;

        let covers = album.covers.clone();
        let session = session.clone();

        let image_retriever: AsyncFn<Bytes> = Arc::new(move || {
            let covers = covers.clone();
            let session = session.clone();

            Box::pin(async move {
                let cover = covers.first()?;
                session.spclient().get_image(&cover.id).await.ok()
            })
        });

        Ok(TrackMetadata::from(
            metadata,
            artists,
            album,
            image_retriever,
        ))
    }
}

#[async_trait::async_trait]
impl TrackCollection for Track {
    async fn get_tracks(&self, _session: &Session) -> Vec<Track> {
        vec![self.clone()]
    }
}

pub struct Album {
    id: SpotifyUri,
}

impl Album {
    pub fn new(album: &str) -> Result<Self> {
        let id = parse_uri_or_url(album).ok_or(anyhow::anyhow!("Invalid album"))?;
        Ok(Album { id })
    }

    pub fn from_id(id: SpotifyUri) -> Self {
        Album { id }
    }

    pub async fn is_album(id: SpotifyUri, session: &Session) -> bool {
        librespot::metadata::Album::get(session, &id).await.is_ok()
    }
}

#[async_trait::async_trait]
impl TrackCollection for Album {
    async fn get_tracks(&self, session: &Session) -> Vec<Track> {
        let album = librespot::metadata::Album::get(session, &self.id)
            .await
            .expect("Failed to get album");
        album.tracks().map(|track| Track::from_id(track.clone())).collect()
    }
}

pub struct Playlist {
    id: SpotifyUri,
}

impl Playlist {
    pub fn new(playlist: &str) -> Result<Self> {
        let id = parse_uri_or_url(playlist).ok_or(anyhow::anyhow!("Invalid playlist"))?;
        Ok(Playlist { id })
    }

    pub fn from_id(id: SpotifyUri) -> Self {
        Playlist { id }
    }

    pub async fn is_playlist(id: SpotifyUri, session: &Session) -> bool {
        librespot::metadata::Playlist::get(session, &id)
            .await
            .is_ok()
    }
}

#[async_trait::async_trait]
impl TrackCollection for Playlist {
    async fn get_tracks(&self, session: &Session) -> Vec<Track> {
        let playlist = librespot::metadata::Playlist::get(session, &self.id)
            .await
            .expect("Failed to get playlist");
        playlist.tracks().map(|track| Track::from_id(track.clone())).collect()
    }
}

#[derive(Clone)]
pub struct TrackMetadata {
    pub artists: Vec<ArtistMetadata>,
    pub track_name: String,
    pub album: AlbumMetadata,
    pub duration: i32,
    image_retriever: AsyncFn<Bytes>,
}

impl TrackMetadata {
    pub fn from(
        track: librespot::metadata::Track,
        artists: Vec<librespot::metadata::Artist>,
        album: librespot::metadata::Album,
        image_retriever: AsyncFn<Bytes>,
    ) -> Self {
        let artists = artists
            .iter()
            .map(|artist| ArtistMetadata::from(artist.clone()))
            .collect();
        let album = AlbumMetadata::from(album);

        TrackMetadata {
            artists,
            track_name: track.name.clone(),
            album,
            duration: track.duration,
            image_retriever,
        }
    }

    pub fn approx_size(&self) -> usize {
        let duration = self.duration / 1000;
        let sample_rate = 44100;
        let channels = 2;
        let bits_per_sample = 32;
        let bytes_per_sample = bits_per_sample / 8;
        (duration as usize) * sample_rate * channels * bytes_per_sample
    }

    pub async fn tags(&self) -> Result<Tags> {
        let tags = Tags {
            title: self.track_name.clone(),
            artists: self.artists.iter().map(|a| a.name.clone()).collect(),
            album_title: self.album.name.clone(),
            album_cover: (self.image_retriever)().await,
        };
        Ok(tags)
    }
}

impl ToString for TrackMetadata {
    fn to_string(&self) -> String {
        if self.artists.len() > 3 {
            let artists_name = self
                .artists
                .iter()
                .take(3)
                .map(|artist| artist.name.clone())
                .collect::<Vec<String>>()
                .join(", ");
            return clean_invalid_characters(format!(
                "{}, ... - {}",
                artists_name, self.track_name
            ));
        }

        let artists_name = self
            .artists
            .iter()
            .map(|artist| artist.name.clone())
            .collect::<Vec<String>>()
            .join(", ");
        clean_invalid_characters(format!("{} - {}", artists_name, self.track_name))
    }
}

#[derive(Clone, Debug)]
pub struct ArtistMetadata {
    pub name: String,
}

impl From<librespot::metadata::Artist> for ArtistMetadata {
    fn from(artist: librespot::metadata::Artist) -> Self {
        ArtistMetadata {
            name: artist.name.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AlbumMetadata {
    pub name: String,
    pub cover: Option<Image>,
}

impl From<librespot::metadata::Album> for AlbumMetadata {
    fn from(album: librespot::metadata::Album) -> Self {
        AlbumMetadata {
            name: album.name.clone(),
            cover: album.covers.first().cloned(),
        }
    }
}
