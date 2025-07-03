use crate::media_events::{MetadataInfo, PlaybackInfo};

pub enum JniCallback {
    #[cfg(target_os = "linux")]
    TrayItemClicked(String),
    SessionsChanged(Vec<(String, String)>),
    MetadataChanged(String, MetadataInfo),
    PlaybackStateChanged(String, PlaybackInfo),
    IpcCallback(String, String),
}
