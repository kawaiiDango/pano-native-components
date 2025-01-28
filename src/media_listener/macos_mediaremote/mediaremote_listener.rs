use std::{
    mem::transmute, ops::Deref, path::Path, sync::{LazyLock, Mutex, OnceLock}
};

use block2::RcBlock;
use core_foundation::bundle::CFBundle;
use core_foundation::string::CFString;
use core_foundation::url::CFURL;
use core_services::{CFArray, LSCopyApplicationURLsForBundleIdentifier, TCFType};
use dispatch2::ffi::{dispatch_get_global_queue, dispatch_queue_global_t};
use objc2::{
    rc::Retained,
    runtime::{AnyObject, Bool, NSObject, ProtocolObject},
};
use objc2_foundation::{NSDictionary, NSNotificationCenter, NSObjectProtocol, NSString};
use tokio::sync::mpsc::{self};

use crate::{
    INCOMING_PLAYER_EVENT_TX, is_app_allowed,
    media_info_structs::{
        IncomingPlayerEvent, MetadataInfo, PlaybackInfo, PlaybackState, SessionInfo,
    },
    media_listener::{
        MediaRemoteEvent, macos_mediaremote::ns_dictionary_extensions::NSDictionaryExtensions,
    },
    on_active_sessions_changed, on_metadata_changed, on_playback_state_changed,
    send_incoming_player_event,
};

#[allow(improper_ctypes_definitions)]
type MRMediaRemoteGetNowPlayingInfoFunction = extern "C" fn(
    dispatch_queue_global_t,
    RcBlock<dyn Fn(*const NSDictionary<NSString, NSObject>)>,
);

#[allow(improper_ctypes_definitions)]
type MRMediaRemoteRegisterForNowPlayingNotifications = extern "C" fn(dispatch_queue_global_t);

#[allow(improper_ctypes_definitions)]
type MRMediaRemoteUnregisterForNowPlayingNotifications = extern "C" fn();

#[allow(improper_ctypes_definitions)]
type MRMediaRemoteGetNowPlayingApplicationIsPlaying =
    extern "C" fn(dispatch_queue_global_t, RcBlock<dyn Fn(Bool)>);

#[allow(improper_ctypes_definitions)]
type MRMediaRemoteGetNowPlayingClient =
    extern "C" fn(dispatch_queue_global_t, RcBlock<dyn Fn(*const NSObject)>);

#[allow(improper_ctypes_definitions)]
type MRNowPlayingClientGetBundleIdentifier = extern "C" fn(*const NSObject) -> *const NSString;

#[allow(improper_ctypes_definitions)]
type MRMediaRemoteSendCommand = extern "C" fn(MRMediaRemoteCommand, *const NSDictionary) -> Bool;

static MR_FUNCTIONS: OnceLock<MediaRemoteFunctions> = OnceLock::new();
static APP_ID_CACHED: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));

#[allow(non_camel_case_types)]
#[allow(clippy::enum_variant_names)]
#[repr(C)]
#[derive(Debug)]
enum MRMediaRemoteCommand {
    MRMediaRemoteCommandPlay,
    MRMediaRemoteCommandPause,
    MRMediaRemoteCommandTogglePlayPause,
    MRMediaRemoteCommandStop,
    MRMediaRemoteCommandNextTrack,
    MRMediaRemoteCommandPreviousTrack,
    // Add other commands as needed
}

struct MediaRemoteFunctions {
    get_now_playing_info_function: MRMediaRemoteGetNowPlayingInfoFunction,
    get_now_playing_application_is_playing_function: MRMediaRemoteGetNowPlayingApplicationIsPlaying,
    get_now_playing_client: MRMediaRemoteGetNowPlayingClient,
    get_now_playing_client_bundle_identifier: MRNowPlayingClientGetBundleIdentifier,
    send_command: MRMediaRemoteSendCommand,
    register_for_now_playing_notifications_function:
        MRMediaRemoteRegisterForNowPlayingNotifications,
    unregister_now_playing_notifications_function:
        MRMediaRemoteUnregisterForNowPlayingNotifications,
}

impl MediaRemoteFunctions {
    fn new() -> Self {
        unsafe {
            let bundle_url = CFURL::from_path(
                "/System/Library/PrivateFrameworks/MediaRemote.framework",
                true,
            )
            .unwrap();
            let bundle = CFBundle::new(bundle_url).unwrap();

            MediaRemoteFunctions {
                get_now_playing_info_function: transmute::<
                    *const std::ffi::c_void,
                    MRMediaRemoteGetNowPlayingInfoFunction,
                >(bundle.function_pointer_for_name(
                    CFString::from_static_string("MRMediaRemoteGetNowPlayingInfo"),
                )),

                get_now_playing_application_is_playing_function: transmute::<
                    *const std::ffi::c_void,
                    MRMediaRemoteGetNowPlayingApplicationIsPlaying,
                >(
                    bundle.function_pointer_for_name(CFString::from_static_string(
                        "MRMediaRemoteGetNowPlayingApplicationIsPlaying",
                    )),
                ),

                get_now_playing_client: transmute::<
                    *const std::ffi::c_void,
                    MRMediaRemoteGetNowPlayingClient,
                >(bundle.function_pointer_for_name(
                    CFString::from_static_string("MRMediaRemoteGetNowPlayingClient"),
                )),

                get_now_playing_client_bundle_identifier: transmute::<
                    *const std::ffi::c_void,
                    MRNowPlayingClientGetBundleIdentifier,
                >(
                    bundle.function_pointer_for_name(CFString::from_static_string(
                        "MRNowPlayingClientGetBundleIdentifier",
                    )),
                ),

                send_command: transmute::<*const std::ffi::c_void, MRMediaRemoteSendCommand>(
                    bundle.function_pointer_for_name(CFString::from_static_string(
                        "MRMediaRemoteSendCommand",
                    )),
                ),

                register_for_now_playing_notifications_function: transmute::<
                    *const std::ffi::c_void,
                    MRMediaRemoteRegisterForNowPlayingNotifications,
                >(
                    bundle.function_pointer_for_name(CFString::from_static_string(
                        "MRMediaRemoteRegisterForNowPlayingNotifications",
                    )),
                ),

                unregister_now_playing_notifications_function: transmute::<
                    *const std::ffi::c_void,
                    MRMediaRemoteUnregisterForNowPlayingNotifications,
                >(
                    bundle.function_pointer_for_name(CFString::from_static_string(
                        "MRMediaRemoteUnregisterForNowPlayingNotifications",
                    )),
                ),
            }
        }
    }

    fn poll_now_playing_info(&self) {
        let callback_block = block2::StackBlock::new(
            move |raw_info_dictionary: *const NSDictionary<NSString, NSObject>| unsafe {
                let app_id = APP_ID_CACHED.lock().unwrap();

                if !is_app_allowed(app_id.clone().unwrap_or_default().as_str()) {
                    return;
                }

                let info_dictionary = match raw_info_dictionary.as_ref() {
                    Some(x) => x,
                    None => return,
                };

                let title = info_dictionary
                    .get_string_for_key("kMRMediaRemoteNowPlayingInfoTitle")
                    .unwrap_or_default();
                let artist = info_dictionary
                    .get_string_for_key("kMRMediaRemoteNowPlayingInfoArtist")
                    .unwrap_or_default();

                let album = info_dictionary
                    .get_string_for_key("kMRMediaRemoteNowPlayingInfoAlbum")
                    .unwrap_or_default();

                let duration_secs: f64 = info_dictionary
                    .get_f64_for_key("kMRMediaRemoteNowPlayingInfoDuration")
                    .unwrap_or_default();

                let duration: i64 = (duration_secs * 1000f64) as i64;

                let track_number = info_dictionary
                    .get_i32_for_key("kMRMediaRemoteNowPlayingInfoTrackNumber")
                    .unwrap_or_default();

                let playback_rate = info_dictionary
                    .get_f64_for_key("kMRMediaRemoteNowPlayingInfoPlaybackRate")
                    .unwrap_or_default();
                let is_playing = playback_rate > 0f64;

                let position_secs: f64 = info_dictionary
                    .get_f64_for_key("kMRMediaRemoteNowPlayingInfoElapsedTime")
                    .unwrap_or_default();

                let position: i64 = (position_secs * 1000f64) as i64;

                if let Some(app_id) = app_id.as_deref() {
                    let metadata_info = MetadataInfo {
                        app_id: app_id.to_string(),
                        title,
                        artist,
                        album,
                        album_artist: "".to_string(),
                        track_number,
                        duration,
                    };

                    let playback_info = PlaybackInfo {
                        app_id: app_id.to_string(),
                        state: if is_playing {
                            PlaybackState::Playing
                        } else {
                            PlaybackState::Paused
                        },
                        position,
                        can_skip: true,
                    };

                    on_metadata_changed(serde_json::to_string(&metadata_info).unwrap());
                    on_playback_state_changed(serde_json::to_string(&playback_info).unwrap());
                }
            },
        );

        let queue = unsafe { dispatch_get_global_queue(0, 0) };
        let _ = &(self.get_now_playing_info_function)(queue, callback_block.copy());
    }

    // fn poll_is_playing(&self) {
    //     let callback_block = block2::StackBlock::new(move |is_playing: Bool| {
    //         println!("is_playing: {}", is_playing.as_bool());

    //         let app_id = APP_ID_CACHED.lock().unwrap();

    //         if !is_app_allowed(app_id.clone().unwrap_or_default().as_str()) {
    //             return;
    //         }

    //         let playback_info = PlaybackInfo {
    //             app_id: app_id.clone().unwrap_or_default(),
    //             state: if is_playing.as_bool() {
    //                 PlaybackState::Playing
    //             } else {
    //                 PlaybackState::Paused
    //             },
    //             position: 0, // todo implement
    //             can_skip: true,
    //         };

    //         on_playback_state_changed(serde_json::to_string(&playback_info).unwrap());
    //     });

    //     let queue = unsafe { dispatch_get_global_queue(0, 0) };
    //     let _ =
    //         &(self.get_now_playing_application_is_playing_function)(queue, callback_block.copy());
    // }

    fn poll_app_info(&self) {
        let get_now_playing_client_bundle_identifier =
            self.get_now_playing_client_bundle_identifier;

        let callback_block = RcBlock::new(move |client: *const NSObject| {
            let bundle_identifier = (get_now_playing_client_bundle_identifier)(client);
            let bundle_identifier = unsafe { bundle_identifier.as_ref().map(|x| x.to_string()) };
            *APP_ID_CACHED.lock().unwrap() = bundle_identifier.clone();

            let session_infos = match bundle_identifier {
                Some(identifier) => {
                    let app_name = fetch_app_name(&identifier);
                    vec![SessionInfo {
                        app_id: identifier,
                        app_name: app_name.unwrap_or_default(),
                    }]
                }

                None => vec![],
            };

            on_active_sessions_changed(serde_json::to_string(&session_infos).unwrap());
        });

        let queue = unsafe { dispatch_get_global_queue(0, 0) };
        (self.get_now_playing_client)(queue, callback_block);
    }

    fn send_skip(&self, app_id: String) {
        let app_id_cached = APP_ID_CACHED.lock().unwrap();

        if app_id_cached.as_deref() != Some(&app_id) {
            return;
        }

        let nil_ptr = std::ptr::null();
        let res = (self.send_command)(MRMediaRemoteCommand::MRMediaRemoteCommandNextTrack, nil_ptr);
        println!("send_command result: {}", res.as_bool());
    }

    fn register_notifications(
        &self,
        notification_center: &NSNotificationCenter,
    ) -> Vec<Retained<ProtocolObject<dyn NSObjectProtocol>>> {
        let mut observers: Vec<Retained<ProtocolObject<dyn NSObjectProtocol>>> = Vec::new();

        unsafe {
            let o = notification_center.addObserverForName_object_queue_usingBlock(
                Some(&NSString::from_str(
                    "kMRMediaRemoteNowPlayingApplicationDidChangeNotification",
                )),
                None,
                None,
                &block2::StackBlock::new(|_| {
                    send_incoming_player_event(IncomingPlayerEvent::MediaRemote(
                        MediaRemoteEvent::kMRMediaRemoteNowPlayingApplicationDidChangeNotification,
                    ))
                }),
            );

            observers.push(o);

            let o = notification_center.addObserverForName_object_queue_usingBlock(
                Some(&NSString::from_str(
                    "kMRMediaRemoteNowPlayingApplicationIsPlayingDidChangeNotification",
                )),
                None,
                None,
                &block2::StackBlock::new(|_| {
                    send_incoming_player_event(
                        IncomingPlayerEvent::MediaRemote(
                            MediaRemoteEvent::kMRMediaRemoteNowPlayingApplicationIsPlayingDidChangeNotification,
                        ),
                    );
                }),
            );
            observers.push(o);

            let o = notification_center.addObserverForName_object_queue_usingBlock(
                Some(&NSString::from_str(
                    "kMRMediaRemoteNowPlayingApplicationClientStateDidChange",
                )),
                None,
                None,
                &block2::StackBlock::new(|_| {
                    send_incoming_player_event(IncomingPlayerEvent::MediaRemote(
                        MediaRemoteEvent::kMRMediaRemoteNowPlayingApplicationClientStateDidChange,
                    ));
                }),
            );
            observers.push(o);

            let o = notification_center.addObserverForName_object_queue_usingBlock(
                Some(&NSString::from_str(
                    "kMRNowPlayingPlaybackQueueChangedNotification",
                )),
                None,
                None,
                &block2::StackBlock::new(|_| {
                    send_incoming_player_event(IncomingPlayerEvent::MediaRemote(
                        MediaRemoteEvent::kMRNowPlayingPlaybackQueueChangedNotification,
                    ));
                }),
            );
            observers.push(o);

            let o = notification_center.addObserverForName_object_queue_usingBlock(
                Some(&NSString::from_str(
                    "kMRPlaybackQueueContentItemsChangedNotification",
                )),
                None,
                None,
                &block2::StackBlock::new(|_| {
                    send_incoming_player_event(IncomingPlayerEvent::MediaRemote(
                        MediaRemoteEvent::kMRPlaybackQueueContentItemsChangedNotification,
                    ));
                }),
            );
            observers.push(o);
        }

        let queue = unsafe { dispatch_get_global_queue(0, 0) };

        let _ = &(self.register_for_now_playing_notifications_function)(queue);

        observers
    }

    fn unregister_notifications(
        &self,
        notification_center: &NSNotificationCenter,
        observers: &mut Vec<Retained<ProtocolObject<dyn NSObjectProtocol>>>,
    ) {
        unsafe {
            for observer in observers.drain(..) {
                let any_object = Retained::cast_unchecked::<AnyObject>(observer);
                notification_center.removeObserver(&any_object);
            }
        }

        (self.unregister_now_playing_notifications_function)();
    }
}

fn fetch_app_name(bundle_id: &str) -> Option<String> {
    let bundle_id = CFString::new(bundle_id);

    unsafe {
        let urls = LSCopyApplicationURLsForBundleIdentifier(
            bundle_id.as_concrete_TypeRef(),
            std::ptr::null_mut(),
        );

        if urls.is_null() {
            None
        } else {
            let cf_array: CFArray<CFURL> = CFArray::wrap_under_get_rule(urls);
            if cf_array.is_empty() {
                None
            } else {
                let first_url = cf_array.get(0).unwrap();
                let url_string = first_url.absolute().get_string().to_string();
                let path = Path::new(&url_string);
                path.file_stem()
                    .map(|file_name| file_name.to_string_lossy().into_owned())
            }
        }
    }
}

pub fn listener() -> Result<(), Box<dyn std::error::Error>> {
    let (tx_incoming_event, mut rx_incoming_event) = mpsc::channel(1);

    *INCOMING_PLAYER_EVENT_TX.lock().unwrap() = Some(tx_incoming_event);

    let mr_functions = MR_FUNCTIONS.get_or_init(MediaRemoteFunctions::new);

    let notification_center = unsafe { NSNotificationCenter::defaultCenter() };
    let mut observers = mr_functions.register_notifications(&notification_center);

    // force update at start
    mr_functions.poll_app_info();
    mr_functions.poll_now_playing_info();
    // mr_functions.poll_is_playing();

    loop {
        match rx_incoming_event.blocking_recv() {
            Some(IncomingPlayerEvent::MediaRemote(event)) => match event {
                MediaRemoteEvent::kMRMediaRemoteNowPlayingApplicationDidChangeNotification => {
                    mr_functions.poll_app_info();
                }
                MediaRemoteEvent::kMRMediaRemoteNowPlayingApplicationIsPlayingDidChangeNotification => {
                    println!("kMRMediaRemoteNowPlayingApplicationIsPlayingDidChangeNotification");
                    mr_functions.poll_now_playing_info();
                }
                MediaRemoteEvent::kMRMediaRemoteNowPlayingApplicationClientStateDidChange => {
                    println!("kMRMediaRemoteNowPlayingApplicationClientStateDidChange");
                    mr_functions.poll_now_playing_info();
                }
                MediaRemoteEvent::kMRNowPlayingPlaybackQueueChangedNotification => {
                    println!("kMRNowPlayingPlaybackQueueChangedNotification");
                    // todo check if this is required
                }
                MediaRemoteEvent::kMRPlaybackQueueContentItemsChangedNotification => {
                    println!("kMRPlaybackQueueContentItemsChangedNotification");
                    mr_functions.poll_now_playing_info();
                }
            }

            Some(IncomingPlayerEvent::RefreshSessions) => {
                mr_functions.poll_app_info();
                mr_functions.poll_now_playing_info();
                // mr_functions.poll_is_playing();
            }

            Some(IncomingPlayerEvent::Skip(app_id)) => {
                mr_functions.send_skip(app_id);
            }

            Some(IncomingPlayerEvent::Mute(_)) | Some(IncomingPlayerEvent::Unmute(_)) => {
                // not implemented on macOS
            }

            Some(IncomingPlayerEvent::Shutdown) |
            None => {
                mr_functions.unregister_notifications(&notification_center, &mut observers);
                break;
            }
        }
    }

    Ok(())
}
