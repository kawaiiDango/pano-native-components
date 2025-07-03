use crate::ipc;
use crate::jni_callback::JniCallback;
use crate::media_events::{IncomingPlayerEvent, MetadataInfo, PlaybackInfo, PlaybackState};
use crate::{INCOMING_PLAYER_EVENT_TX, is_app_allowed};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, LazyLock, Mutex, OnceLock};
use tokio::sync::mpsc;
use windows::ApplicationModel::AppInfo;
use windows::Foundation::TypedEventHandler;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession, GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus, MediaPropertiesChangedEventArgs,
    PlaybackInfoChangedEventArgs, SessionsChangedEventArgs, TimelinePropertiesChangedEventArgs,
};

static PLAYBACK_INFO_CACHE: LazyLock<Mutex<HashMap<String, PlaybackInfo>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static METADATA_INFO_CACHE: LazyLock<Mutex<HashMap<String, MetadataInfo>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static APP_NAMES_CACHE: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static PREV_APP_IDS: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

struct CallbackTokens {
    playback_info_changed: i64,
    media_properties_changed: i64,
    timeline_properties_changed: i64,
}

static CALLBACK_TOKENS_MAP: LazyLock<Mutex<HashMap<String, CallbackTokens>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// based on https://github.com/KDE/kdeconnect-kde/blob/master/plugins/mpriscontrol/mpriscontrolplugin-win.cpp

static OUTGOING_PLAYER_EVENT_TX: OnceLock<mpsc::Sender<JniCallback>> = OnceLock::new();

#[tokio::main(flavor = "current_thread")]
pub async fn listener(
    jni_callback: impl Fn(JniCallback) + Send + Sync + 'static,
) -> Result<(), Box<dyn std::error::Error>> {
    let (incoming_tx, mut incoming_rx) = mpsc::channel(10);
    *INCOMING_PLAYER_EVENT_TX.lock().unwrap() = Some(incoming_tx);

    let (outgoing_tx, mut outgoing_rx) = mpsc::channel(10);
    OUTGOING_PLAYER_EVENT_TX.set(outgoing_tx).unwrap();

    let jni_callback_arc = Arc::new(jni_callback);

    let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?.get()?;

    let sessions_changed_token = manager
        .SessionsChanged(&TypedEventHandler::<
            GlobalSystemMediaTransportControlsSessionManager,
            SessionsChangedEventArgs,
        >::new(move |m, _| {
            match m.as_ref() {
                Some(m) => {
                    update_sessions(m);
                }
                None => {
                    eprintln!("SessionsChanged event handler received None");
                }
            }
            Ok(())
        }))
        .expect("Failed to set SessionsChanged event handler");

    // force update on start
    update_sessions(&manager);

    let session_events = async {
        while let Some(event) = incoming_rx.recv().await {
            match event {
                IncomingPlayerEvent::Skip(app_id) => {
                    for session in manager.GetSessions().unwrap().into_iter() {
                        let session_app_id = session
                            .SourceAppUserModelId()
                            .expect("Failed to get app ID")
                            .to_string();

                        if session_app_id == app_id {
                            let _ = session.TrySkipNextAsync();

                            break;
                        }
                    }
                }
                IncomingPlayerEvent::RefreshSessions => {
                    update_sessions(&manager);
                }

                IncomingPlayerEvent::Shutdown => {
                    INCOMING_PLAYER_EVENT_TX.lock().unwrap().take();

                    let mut callback_tokens_map = CALLBACK_TOKENS_MAP.lock().unwrap();

                    // remove listeners

                    for session in manager.GetSessions().unwrap().into_iter() {
                        let session_app_id = session
                            .SourceAppUserModelId()
                            .expect("Failed to get app ID")
                            .to_string();

                        if let Some(tokens) = callback_tokens_map.remove(&session_app_id) {
                            let _ = session.RemovePlaybackInfoChanged(tokens.playback_info_changed);
                            let _ = session
                                .RemoveMediaPropertiesChanged(tokens.media_properties_changed);
                            let _ = session.RemoveTimelinePropertiesChanged(
                                tokens.timeline_properties_changed,
                            );
                        }
                    }
                    let _ = manager.RemoveSessionsChanged(sessions_changed_token);

                    // clear caches
                    callback_tokens_map.clear();
                    PLAYBACK_INFO_CACHE.lock().unwrap().clear();
                    METADATA_INFO_CACHE.lock().unwrap().clear();
                    APP_NAMES_CACHE.lock().unwrap().clear();
                    PREV_APP_IDS.lock().unwrap().clear();

                    break;
                }
                _ => {}
            }
        }

        Ok::<(), Box<dyn std::error::Error>>(())
    };

    let outgoing_events = async {
        while let Some(event) = outgoing_rx.recv().await {
            jni_callback_arc(event);
        }

        Ok(())
    };

    // other listeners
    let jni_callback = jni_callback_arc.clone();
    let ipc_commands = ipc::commands_listener(move |command: String, arg: String| {
        let event = JniCallback::IpcCallback(command, arg);
        jni_callback(event);
    });

    tokio::try_join!(session_events, ipc_commands, outgoing_events)?;

    Ok(())
}

fn update_sessions(manager: &GlobalSystemMediaTransportControlsSessionManager) {
    let mut app_names_cache = APP_NAMES_CACHE.lock().unwrap();

    // let old_session_infos = SESSION_INFOS.lock().unwrap().clone();

    // let old_session_infos_map = old_session_infos
    //     .clone()
    //     .into_iter()
    //     .map(|x| (x.app_id.clone(), x))
    //     .collect::<HashMap<String, SessionInfo>>();

    let mut all_session_infos_map: HashMap<String, String> = HashMap::new();

    for session in manager.GetSessions().unwrap().into_iter() {
        let app_id_hstring = session
            .SourceAppUserModelId()
            .expect("Failed to get app ID");

        let app_id = app_id_hstring.to_string();

        let app_name = if let Some(app_name) = app_names_cache.get(&app_id) {
            app_name.to_string()
        } else {
            let app_info = AppInfo::GetFromAppUserModelId(&app_id_hstring);
            let mut app_name = "".to_string();
            if let Ok(app_info) = app_info
                && let Ok(display_info) = app_info.DisplayInfo()
                && let Ok(display_name) = display_info.DisplayName()
            {
                app_name = display_name.to_string();
                app_names_cache.insert(app_id.clone(), app_name.clone());
            }

            app_name
        };

        all_session_infos_map.insert(app_id.clone(), app_name);
    }

    let current_app_ids = all_session_infos_map
        .keys()
        .cloned()
        .collect::<HashSet<String>>();

    let mut prev_app_ids = PREV_APP_IDS.lock().unwrap();
    if current_app_ids != *prev_app_ids {
        let sessions = all_session_infos_map
            .iter()
            .map(|(app_id, app_name)| (app_id.clone(), app_name.clone()))
            .collect::<Vec<_>>();

        let sessions_media_event = JniCallback::SessionsChanged(sessions);

        // send the updated sessions to the JNI callback
        let _ = OUTGOING_PLAYER_EVENT_TX
            .get()
            .unwrap()
            .try_send(sessions_media_event);

        // remove cache for removed sessions
        // todo: also remove session listeners
        for app_id in prev_app_ids.difference(&current_app_ids) {
            CALLBACK_TOKENS_MAP.lock().unwrap().remove(app_id);
            PLAYBACK_INFO_CACHE.lock().unwrap().remove(app_id);
            METADATA_INFO_CACHE.lock().unwrap().remove(app_id);
        }

        *prev_app_ids = current_app_ids;
    }

    for session in manager.GetSessions().unwrap().into_iter() {
        let app_id = session
            .SourceAppUserModelId()
            .expect("Failed to get session ID")
            .to_string();

        // remove listeners for removed sessions
        if !is_app_allowed(&app_id) {
            remove_session(&session, &app_id);
            continue;
        }

        // skip if session is already added or app is not allowed
        if CALLBACK_TOKENS_MAP.lock().unwrap().contains_key(&app_id) {
            continue;
        }

        let playback_info_changed_token = session
            .PlaybackInfoChanged(&TypedEventHandler::<
                GlobalSystemMediaTransportControlsSession,
                PlaybackInfoChangedEventArgs,
            >::new(move |session, _| {
                match session.as_ref() {
                    Some(session) => {
                        handle_playback_info_changed(session);
                    }
                    None => {
                        eprintln!("PlaybackInfoChanged event handler received None");
                    }
                }
                Ok(())
            }))
            .expect("Failed to set PlaybackInfoChanged event handler");

        let media_properties_changed_token = session
            .MediaPropertiesChanged(&TypedEventHandler::<
                GlobalSystemMediaTransportControlsSession,
                MediaPropertiesChangedEventArgs,
            >::new(move |session, _| {
                match session.as_ref() {
                    Some(session) => {
                        handle_media_properties_changed(session);
                    }
                    None => {
                        eprintln!("MediaPropertiesChanged event handler received None");
                    }
                }
                Ok(())
            }))
            .expect("Failed to set MediaPropertiesChanged event handler");

        let timeline_properties_changed_token = session
            .TimelinePropertiesChanged(&TypedEventHandler::<
                GlobalSystemMediaTransportControlsSession,
                TimelinePropertiesChangedEventArgs,
            >::new(move |session, _| {
                match session.as_ref() {
                    Some(session) => {
                        handle_timeline_properties_changed(session);
                    }
                    None => {
                        eprintln!("TimelinePropertiesChanged event handler received None");
                    }
                }
                Ok(())
            }))
            .expect("Failed to set TimelinePropertiesChanged event handler");

        let callback_tokens = CallbackTokens {
            playback_info_changed: playback_info_changed_token,
            media_properties_changed: media_properties_changed_token,
            timeline_properties_changed: timeline_properties_changed_token,
        };
        CALLBACK_TOKENS_MAP
            .lock()
            .unwrap()
            .insert(app_id.clone(), callback_tokens);

        // force update on start

        handle_media_properties_changed(&session);
        handle_playback_info_changed(&session);
        handle_timeline_properties_changed(&session);
    }
}

fn remove_session(session: &GlobalSystemMediaTransportControlsSession, app_id: &str) {
    let tokens = CALLBACK_TOKENS_MAP.lock().unwrap().remove(app_id);

    if let Some(tokens) = tokens {
        let _ = session.RemovePlaybackInfoChanged(tokens.playback_info_changed);
        let _ = session.RemoveMediaPropertiesChanged(tokens.media_properties_changed);
        let _ = session.RemoveTimelinePropertiesChanged(tokens.timeline_properties_changed);

        PLAYBACK_INFO_CACHE.lock().unwrap().remove(app_id);
        METADATA_INFO_CACHE.lock().unwrap().remove(app_id);
    }
}

fn handle_playback_info_changed(session: &GlobalSystemMediaTransportControlsSession) {
    let playback_info = session
        .GetPlaybackInfo()
        .expect("Failed to get playback info");

    let playback_state_smtc = playback_info.PlaybackStatus();

    if let Ok(playback_state_smtc) = playback_state_smtc {
        let state = match playback_state_smtc {
            GlobalSystemMediaTransportControlsSessionPlaybackStatus::Paused => {
                PlaybackState::Paused
            }
            GlobalSystemMediaTransportControlsSessionPlaybackStatus::Stopped => {
                PlaybackState::Stopped
            }
            GlobalSystemMediaTransportControlsSessionPlaybackStatus::Changing
            | GlobalSystemMediaTransportControlsSessionPlaybackStatus::Opened => {
                PlaybackState::Waiting
            }
            GlobalSystemMediaTransportControlsSessionPlaybackStatus::Closed => PlaybackState::None,
            GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing => {
                PlaybackState::Playing
            }
            _ => PlaybackState::Other,
        };

        let playback_controls = playback_info
            .Controls()
            .expect("Failed to get playback controls");

        let can_skip = playback_controls.IsNextEnabled().unwrap_or_default();

        let app_id = session
            .SourceAppUserModelId()
            .unwrap_or_default()
            .to_string();

        let playback_info = PlaybackInfo {
            state,
            can_skip,
            position: -1, // this will be updated later from timeline properties
        };

        // insert into PLAYBACK_INFO_CACHE

        let mut cache = PLAYBACK_INFO_CACHE.lock().unwrap();
        cache.insert(app_id.clone(), playback_info.clone());

        let _ = OUTGOING_PLAYER_EVENT_TX
            .get()
            .unwrap()
            .try_send(JniCallback::PlaybackStateChanged(app_id, playback_info));
    } else {
        eprintln!("Failed to get playback state");
    }
}

fn handle_media_properties_changed(session: &GlobalSystemMediaTransportControlsSession) {
    let media_properties = session
        .TryGetMediaPropertiesAsync()
        .expect("TryGetMediaPropertiesAsync failed");

    if let Ok(media_properties) = media_properties.get() {
        let app_id = session
            .SourceAppUserModelId()
            .expect("Failed to get session ID")
            .to_string();

        let title = media_properties.Title().unwrap_or_default().to_string();
        let artist = media_properties.Artist().unwrap_or_default().to_string();
        let album = media_properties
            .AlbumTitle()
            .unwrap_or_default()
            .to_string();
        let album_artist = media_properties
            .AlbumArtist()
            .unwrap_or_default()
            .to_string();
        let track_number = media_properties.TrackNumber().unwrap_or_default();
        // let genres = media_properties
        //     .Genres()
        //     .map(|x| x.GetAt(0).unwrap_or_default().to_string())
        //     .ok();
        // let playback_type = media_properties
        //     .PlaybackType()
        //     .map(|x| x.Value().unwrap_or(windows::Media::MediaPlaybackType(0)))
        //     .ok();

        // println!("Genres: {:?}", genres);
        // println!("Playback Type: {:?}", playback_type);

        // get the existing duration if available
        let mut cache = METADATA_INFO_CACHE.lock().unwrap();
        let existing_duration = cache.get(&app_id).map(|x| x.duration).unwrap_or(-1);

        let metadata_info = MetadataInfo {
            title,
            artist,
            album,
            album_artist,
            track_number,
            duration: existing_duration, // this will be updated later from timeline properties
        };

        cache.insert(app_id.clone(), metadata_info.clone());

        let _ = OUTGOING_PLAYER_EVENT_TX
            .get()
            .unwrap()
            .try_send(JniCallback::MetadataChanged(app_id, metadata_info));
    } else {
        eprintln!("Failed to get media properties");
    }
}

fn handle_timeline_properties_changed(session: &GlobalSystemMediaTransportControlsSession) {
    let timeline_properties = session.GetTimelineProperties();

    if let Ok(timeline_properties) = timeline_properties {
        let app_id = session
            .SourceAppUserModelId()
            .expect("Failed to get app ID")
            .to_string();

        // Duration is in ns, convert to ms
        // some players dont report timeline properties, handle that with -1
        let end_time = timeline_properties
            .EndTime()
            .map(|x| x.Duration / 10000)
            .unwrap_or(-1);
        let start_time = timeline_properties
            .StartTime()
            .map(|x| x.Duration / 10000)
            .unwrap_or_default();

        let duration = if end_time == -1 {
            -1
        } else {
            end_time - start_time
        };

        let position = timeline_properties
            .Position()
            .map(|x| x.Duration / 10000)
            .unwrap_or(-1);

        // println!("position: {}, duration: {}", position, duration);

        // on windows, the duration may get updated much later than the media properties
        if duration != -1 {
            let mut cache = METADATA_INFO_CACHE.lock().unwrap();
            let existing_metadata_info = cache.get(&app_id);
            if let Some(existing_metadata_info) = existing_metadata_info
                && existing_metadata_info.duration != duration
            {
                let mut metadata_info = existing_metadata_info.clone();
                metadata_info.duration = duration;

                // update the cache
                cache.insert(app_id.clone(), metadata_info.clone());

                // report the updated metadata
                let _ = OUTGOING_PLAYER_EVENT_TX
                    .get()
                    .unwrap()
                    .try_send(JniCallback::MetadataChanged(app_id.clone(), metadata_info));
            }
        }

        if position != -1 {
            let mut cache = PLAYBACK_INFO_CACHE.lock().unwrap();

            let existing_playback_info = cache.get(&app_id);

            if let Some(existing_playback_info) = existing_playback_info {
                if existing_playback_info.position == -1 || position < 1500 {
                    // todo figure something out to prevent the spam
                    let mut playback_info = existing_playback_info.clone();
                    playback_info.position = position;

                    // update the cache
                    cache.insert(app_id.clone(), playback_info.clone());

                    // report the updated playback info
                    let _ = OUTGOING_PLAYER_EVENT_TX
                        .get()
                        .unwrap()
                        .try_send(JniCallback::PlaybackStateChanged(app_id, playback_info));
                }
            } else {
                let playback_info = PlaybackInfo {
                    state: PlaybackState::None,
                    can_skip: false,
                    position,
                };
                let _ = OUTGOING_PLAYER_EVENT_TX
                    .get()
                    .unwrap()
                    .try_send(JniCallback::PlaybackStateChanged(app_id, playback_info));
            }
        }
    } else {
        eprintln!("Failed to get timeline properties");
    }
}
