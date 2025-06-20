use anyhow::Result;
use lazy_static::lazy_static;
use librespot::core::session::Session;
use librespot::core::spotify_id::SpotifyId;
use librespot::metadata::Metadata;
use regex::Regex;

#[async_trait::async_trait]
trait TrackCollection {
    async fn get_tracks(&self, session: &Session) -> Vec<Track>;
}

#[tracing::instrument(name = "get_tracks", skip(session), level = "debug")]
pub async fn get_tracks(spotify_ids: Vec<String>, session: &Session) -> Result<Vec<Track>> {
    let mut tracks: Vec<Track> = Vec::new();
    for id in spotify_ids {
        tracing::debug!("Getting tracks for: {}", id);
        let id = parse_uri_or_url(&id).ok_or(anyhow::anyhow!("Invalid track `{id}`"))?;
        let new_tracks = match id.item_type {
            librespot::core::spotify_id::SpotifyItemType::Track => vec![Track::from_id(id)],
            librespot::core::spotify_id::SpotifyItemType::Episode => vec![Track::from_id(id)],
            librespot::core::spotify_id::SpotifyItemType::Album => Album::from_id(id).get_tracks(session).await,
            librespot::core::spotify_id::SpotifyItemType::Playlist => Playlist::from_id(id).get_tracks(session).await,
            librespot::core::spotify_id::SpotifyItemType::Show => vec![],
            librespot::core::spotify_id::SpotifyItemType::Artist => vec![],
            librespot::core::spotify_id::SpotifyItemType::Local => vec![],
            librespot::core::spotify_id::SpotifyItemType::Unknown => vec![],
        };
        tracks.extend(new_tracks);
    }
    tracing::debug!("Got tracks: {:?}", tracks);
    Ok(tracks)
}

fn parse_uri_or_url(track: &str) -> Option<SpotifyId> {
    parse_uri(track).or_else(|| parse_url(track))
}

fn parse_uri(track_uri: &str) -> Option<SpotifyId> {
    let res = SpotifyId::from_uri(track_uri);
    tracing::info!("Parsed URI: {:?}", res);
    res.ok()
}

fn parse_url(track_url: &str) -> Option<SpotifyId> {
    let results = SPOTIFY_URL_REGEX.captures(track_url)?;
    let uri = format!(
        "spotify:{}:{}",
        results.get(1)?.as_str(),
        results.get(2)?.as_str()
    );
    SpotifyId::from_uri(&uri).ok()
}

#[derive(Clone, Debug)]
pub struct Track {
    pub id: SpotifyId,
}

lazy_static! {
    static ref SPOTIFY_URL_REGEX: Regex =
        Regex::new(r"https://open.spotify.com/(\w+)/(.*)\?").unwrap();
}

impl Track {
    pub fn new(track: &str) -> Result<Self> {
        let id = parse_uri_or_url(track).ok_or(anyhow::anyhow!("Invalid track"))?;
        Ok(Track { id })
    }

    pub fn from_id(id: SpotifyId) -> Self {
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

        Ok(TrackMetadata::from(metadata, artists, album))
    }
}

#[async_trait::async_trait]
impl TrackCollection for Track {
    async fn get_tracks(&self, _session: &Session) -> Vec<Track> {
        vec![self.clone()]
    }
}

pub struct Album {
    id: SpotifyId,
}

impl Album {
    pub fn new(album: &str) -> Result<Self> {
        let id = parse_uri_or_url(album).ok_or(anyhow::anyhow!("Invalid album"))?;
        Ok(Album { id })
    }

    pub fn from_id(id: SpotifyId) -> Self {
        Album { id }
    }

    pub async fn is_album(id: SpotifyId, session: &Session) -> bool {
        librespot::metadata::Album::get(session, &id).await.is_ok()
    }
}

#[async_trait::async_trait]
impl TrackCollection for Album {
    async fn get_tracks(&self, session: &Session) -> Vec<Track> {
        let album = librespot::metadata::Album::get(session, &self.id)
            .await
            .expect("Failed to get album");
        album
            .tracks()
            .map(|track| Track::from_id(*track))
            .collect()
    }
}

pub struct Playlist {
    id: SpotifyId,
}

impl Playlist {
    pub fn new(playlist: &str) -> Result<Self> {
        let id = parse_uri_or_url(playlist).ok_or(anyhow::anyhow!("Invalid playlist"))?;
        Ok(Playlist { id })
    }

    pub fn from_id(id: SpotifyId) -> Self {
        Playlist { id }
    }

    pub async fn is_playlist(id: SpotifyId, session: &Session) -> bool {
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
        playlist
            .tracks()
            .map(|track| Track::from_id(*track))
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct TrackMetadata {
    pub artists: Vec<ArtistMetadata>,
    pub track_name: String,
    pub album: AlbumMetadata,
    pub duration: i32,
}

impl TrackMetadata {
    pub fn from(
        track: librespot::metadata::Track,
        artists: Vec<librespot::metadata::Artist>,
        album: librespot::metadata::Album,
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
        }
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
    pub year: i32,
    pub cover: Option<librespot::metadata::image::Image>,
}

impl From<librespot::metadata::Album> for AlbumMetadata {
    fn from(album: librespot::metadata::Album) -> Self {
        AlbumMetadata {
            name: album.name.clone(),
            year: album.date.as_utc().year(),
            cover: album.covers.first().cloned()
        }
    }
}
