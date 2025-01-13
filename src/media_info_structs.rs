use serde::Serialize;

#[derive(Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionInfo {
    pub app_id: String,
    pub app_name: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct MetadataInfo {
    pub app_id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub track_number: i32,
    pub duration: i64,
}

#[derive(Serialize, Debug, Clone)]
pub struct PlaybackInfo {
    pub app_id: String,
    pub state: PlaybackState,
    pub position: i64,
    pub can_skip: bool,
}

#[derive(Serialize, Debug, Clone)]
pub struct TimelineInfo {
    pub app_id: String,
    pub duration: i64,
    pub position: i64,
}

#[derive(Serialize, Debug, Clone)]
pub enum PlaybackState {
    None,
    Stopped,
    Paused,
    Playing,
    Waiting,
    Other,
}

#[derive(Debug, Clone)]
pub enum IncomingPlayerEvent {
    Skip(String),
    Mute(String),
    Unmute(String),
    RefreshSessions,
    Shutdown,
}
