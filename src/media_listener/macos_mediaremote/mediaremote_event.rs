#[allow(non_camel_case_types)]
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone)]
pub enum MediaRemoteEvent {
    kMRMediaRemoteNowPlayingApplicationDidChangeNotification,
    kMRMediaRemoteNowPlayingApplicationClientStateDidChange,
    kMRNowPlayingPlaybackQueueChangedNotification,
    kMRPlaybackQueueContentItemsChangedNotification,
    kMRMediaRemoteNowPlayingApplicationIsPlayingDidChangeNotification,
}