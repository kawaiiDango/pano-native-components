use std::collections::HashMap;
use std::time::Duration;

use zbus::zvariant::{Array, OwnedValue};

/// A structured representation of the [`Player`](crate::player::Player) metadata.
///
/// * [Read more about the MPRIS2 `Metadata_Map` type.][metadata_map]
/// * [Read MPRIS v2 metadata guidelines][metadata_guidelines]
///
/// [metadata_map]: https://specifications.freedesktop.org/mpris-spec/latest/Track_List_Interface.html#Mapping:Metadata_Map
/// [metadata_guidelines]: https://www.freedesktop.org/wiki/Specifications/mpris-spec/metadata/
#[derive(Debug, Default, Clone)]
pub struct Metadata {
    values: HashMap<String, OwnedValue>,
}

impl Metadata {
    /// Get a value from the metadata by key name.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use mpris::{Metadata, MetadataValue};
    /// # let mut metadata = Metadata::new(String::from("1234"));
    /// # let key_name = "foo";
    /// if let Some(MetadataOwnedValue::String(name)) = metadata.get("xesam:composer") {
    ///     println!("Composed by: {}", name);
    /// }
    /// ```
    pub fn get(&self, key: &str) -> Option<&OwnedValue> {
        self.values.get(key)
    }

    /// The track ID.
    ///
    /// If the [`TrackID`] could not be parsed as a proper [`TrackID`], [`None`] will be returned.
    ///
    /// Based on `mpris:trackid`
    /// > A unique identity for this track within the context of an MPRIS object.
    ///
    pub fn track_id(&self) -> Option<&str> {
        self.get("mpris:trackid")
            .and_then(|v| v.downcast_ref::<&str>().ok())
    }

    /// A list of artists of the album the track appears on.
    ///
    /// Based on `xesam:albumArtist`
    /// > The album artist(s).
    ///
    ///   xesam:artist: OwnedValue(Array(Array { elements: [Str("")], signature: Array(Dynamic { child: Str }) }))
    pub fn album_artists(&self) -> Option<Vec<String>> {
        self.get("xesam:albumArtist")
            .and_then(|v| v.downcast_ref::<Array>().ok())
            .and_then(|v| Vec::<String>::try_from(v).ok())
    }

    /// The name of the album the track appears on.
    ///
    /// Based on `xesam:album`
    /// > The album name.
    pub fn album_name(&self) -> Option<&str> {
        self.get("xesam:album")
            .and_then(|v| v.downcast_ref::<&str>().ok())
    }

    /// An URL to album art of the current track.
    ///
    /// Based on `mpris:artUrl`
    /// > The location of an image representing the track or album. Clients should not assume this
    /// > will continue to exist when the media player stops giving out the URL.
    pub fn art_url(&self) -> Option<&str> {
        self.get("mpris:artUrl")
            .and_then(|v| v.downcast_ref::<&str>().ok())
    }

    /// A list of artists of the track.
    ///
    /// Based on `xesam:artist`
    /// > The track artist(s).
    pub fn artists(&self) -> Option<Vec<String>> {
        self.get("xesam:artist")
            .and_then(|v| v.downcast_ref::<Array>().ok())
            .and_then(|v| Vec::<String>::try_from(v).ok())
    }

    /// Based on `xesam:autoRating`
    /// > An automatically-generated rating, based on things such as how often it has been played.
    /// > This should be in the range 0.0 to 1.0.
    pub fn auto_rating(&self) -> Option<f64> {
        self.get("xesam:autoRating")
            .and_then(|v| v.downcast_ref::<f64>().ok())
    }

    /// Based on `xesam:discNumber`
    /// > The disc number on the album that this track is from.
    pub fn disc_number(&self) -> Option<i32> {
        self.get("xesam:discNumber")
            .and_then(|v| v.downcast_ref::<i32>().ok())
    }

    /// The duration of the track, in microseconds
    ///
    /// Based on `mpris:length`
    /// > The duration of the track in microseconds.
    pub fn length_in_microseconds(&self) -> Option<u64> {
        self.get("mpris:length").and_then(|v| {
            if let Ok(val) = v.downcast_ref::<u64>() {
                Some(val)
            } else if let Ok(val) = v.downcast_ref::<i64>() {
                Some(val as u64)
            } else {
                None
            }
        })
    }

    /// The duration of the track, as a [`Duration`]
    ///
    /// Based on `mpris:length`.
    pub fn length(&self) -> Option<Duration> {
        self.length_in_microseconds().map(Duration::from_micros)
    }

    /// The name of the track.
    ///
    /// Based on `xesam:title`
    /// > The track title.
    pub fn title(&self) -> Option<&str> {
        self.get("xesam:title")
            .and_then(|v| v.downcast_ref::<&str>().ok())
    }

    /// The track number on the disc of the album the track appears on.
    ///
    /// Based on `xesam:trackNumber`
    /// > The track number on the album disc.
    pub fn track_number(&self) -> Option<i32> {
        self.get("xesam:trackNumber")
            .and_then(|v| v.downcast_ref::<i32>().ok())
    }

    /// A URL to the media being played.
    ///
    /// Based on `xesam:url`
    /// > The location of the media file.
    pub fn url(&self) -> Option<&str> {
        self.get("xesam:url")
            .and_then(|v| v.downcast_ref::<&str>().ok())
    }

    /// Returns an owned [`HashMap`] of borrowed values from this [`Metadata`]. Useful if you need a
    /// mutable hash but don't have ownership of [`Metadata`] or want to consume it.
    ///
    /// If you want to convert to a [`HashMap`], use [`Into::into`](std::convert::Into::into) instead.
    pub fn as_hashmap(&self) -> HashMap<&str, &OwnedValue> {
        self.iter().collect()
    }

    /// Iterate all metadata keys and values.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &OwnedValue)> {
        self.values.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Iterate all metadata keys.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.values.keys().map(String::as_str)
    }

    /// Returns [`true`] if there is no metadata
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

impl IntoIterator for Metadata {
    type Item = (String, OwnedValue);
    type IntoIter = std::collections::hash_map::IntoIter<String, OwnedValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

// Disable implicit_hasher; suggested code fix does not compile. I think this might be a false
// positive, but I'm not sure.
impl From<Metadata> for HashMap<String, OwnedValue> {
    fn from(metadata: Metadata) -> Self {
        metadata.values
    }
}

impl From<HashMap<String, OwnedValue>> for Metadata {
    fn from(values: HashMap<String, OwnedValue>) -> Self {
        Metadata { values }
    }
}
