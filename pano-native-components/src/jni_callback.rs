use crate::media_events::{MetadataInfo, PlaybackInfo, SessionInfo};

#[derive(Debug)]
pub enum JniCallback {
    #[cfg(target_os = "linux")]
    TrayItemClicked(String),
    FilePicked(i32, String),
    SessionsChanged(Vec<SessionInfo>),
    MetadataChanged(String, MetadataInfo),
    PlaybackStateChanged(String, PlaybackInfo),
    IpcCallback(String, String),
    DarkModeChanged(bool),
    IsAppIdAllowed(String),
}
