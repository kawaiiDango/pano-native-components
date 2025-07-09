use strum::EnumString;

#[derive(Debug, Clone)]
pub struct MetadataInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub track_number: i32,
    pub duration: i64,
    pub art_url: String,
    pub art_bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct PlaybackInfo {
    pub state: PlaybackState,
    pub position: i64,
    pub can_skip: bool,
}

#[derive(Debug, Clone)]
pub struct TimelineInfo {
    pub duration: i64,
    pub position: i64,
    pub last_updated: i64,
}

#[derive(EnumString, strum::Display, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PlaybackState {
    None,
    Stopped,
    Paused,
    Playing,
    Waiting,
    Other,
}

#[cfg(target_os = "macos")]
use crate::media_listener::MediaRemoteEvent;

#[derive(Debug, Clone)]
pub enum IncomingPlayerEvent {
    Skip(String),
    Mute(String),
    Unmute(String),
    AlbumArtToggled(bool),
    RefreshSessions,
    Shutdown,
    #[cfg(target_os = "macos")]
    MediaRemote(MediaRemoteEvent),
}
