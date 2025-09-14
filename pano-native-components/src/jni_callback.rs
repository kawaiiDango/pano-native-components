use crate::media_events::{MetadataInfo, PlaybackInfo};

pub enum JniCallback {
    #[cfg(target_os = "linux")]
    TrayItemClicked(String),
    #[cfg(target_os = "linux")]
    FilePicked(i32, String),
    SessionsChanged(Vec<(String, String)>),
    MetadataChanged(String, MetadataInfo),
    PlaybackStateChanged(String, PlaybackInfo),
    IpcCallback(String, String),
}
