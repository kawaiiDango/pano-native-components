use crate::media_info_structs::PlaybackState;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
/// A [`Player`]'s looping status.
///
/// See: [MPRIS2 specification about `Loop_Status`][loop_status]
///
/// [loop_status]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Enum:Loop_Status
pub enum LoopStatus {
    /// The playback will stop when there are no more tracks to play
    None,

    /// The current track will start again from the begining once it has finished playing
    Track,

    /// The playback loops through a list of tracks
    Playlist,
}

#[derive(Debug)]
pub struct ParseErr(pub String);

impl std::fmt::Display for ParseErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Couldn't parse `{}`",
            self.0
        )
    }
}

impl ::std::str::FromStr for PlaybackState {
    type Err = ParseErr;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string {
            "Playing" => Ok(PlaybackState::Playing),
            "Paused" => Ok(PlaybackState::Paused),
            "Stopped" => Ok(PlaybackState::Stopped),
            other => Err(ParseErr(other.to_string())),
        }
    }
}


impl std::error::Error for ParseErr {}

impl ::std::str::FromStr for LoopStatus {
    type Err = ParseErr;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string {
            "None" => Ok(LoopStatus::None),
            "Track" => Ok(LoopStatus::Track),
            "Playlist" => Ok(LoopStatus::Playlist),
            other => Err(ParseErr(other.to_string())),
        }
    }
}
