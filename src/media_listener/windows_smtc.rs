use crate::media_info_structs::{
    IncomingPlayerEvent, MetadataInfo, PlaybackInfo, PlaybackState, SessionInfo, TimelineInfo,
};
use crate::{
    INCOMING_PLAYER_EVENT_TX, is_app_allowed, log_warn, on_active_sessions_changed,
    on_metadata_changed, on_playback_state_changed,
};
use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};
use tokio::sync::mpsc;
use windows::ApplicationModel::AppInfo;
use windows::Foundation::TypedEventHandler;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession, GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus,
};

pub static PLAYBACK_INFO_CACHE: LazyLock<Mutex<HashMap<String, PlaybackInfo>>> =
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

pub fn listener() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, mut rx) = mpsc::channel(1);

    *INCOMING_PLAYER_EVENT_TX.lock().unwrap() = Some(tx);

    let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?.get()?;

    let sessions_changed_token = manager
        .SessionsChanged(&TypedEventHandler::new(move |m, _| {
            match m {
                Some(m) => {
                    update_sessions(m);
                }
                None => {
                    log_warn("SessionsChanged event handler received None");
                }
            }
            Ok(())
        }))
        .expect("Failed to set SessionsChanged event handler");

    // force update on start
    update_sessions(&manager);

    while let Some(event) = rx.blocking_recv() {
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
                        let _ =
                            session.RemoveMediaPropertiesChanged(tokens.media_properties_changed);
                        let _ = session
                            .RemoveTimelinePropertiesChanged(tokens.timeline_properties_changed);
                    }
                }
                let _ = manager.RemoveSessionsChanged(sessions_changed_token);

                // clear caches
                callback_tokens_map.clear();
                PLAYBACK_INFO_CACHE.lock().unwrap().clear();
                APP_NAMES_CACHE.lock().unwrap().clear();
                PREV_APP_IDS.lock().unwrap().clear();

                break;
            }
            _ => {}
        }
    }

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

    let mut all_session_infos_map: HashMap<String, SessionInfo> = HashMap::new();

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
            if let Ok(app_info) = app_info {
                if let Ok(display_info) = app_info.DisplayInfo() {
                    if let Ok(display_name) = display_info.DisplayName() {
                        app_name = display_name.to_string();
                        app_names_cache.insert(app_id.clone(), app_name.clone());
                    }
                }
            }

            app_name
        };

        let session_info = SessionInfo {
            app_id: app_id.clone(),
            app_name,
        };

        all_session_infos_map.insert(app_id.clone(), session_info);
    }

    let current_app_ids = all_session_infos_map
        .keys()
        .cloned()
        .collect::<HashSet<String>>();

    if current_app_ids != *PREV_APP_IDS.lock().unwrap() {
        on_active_sessions_changed(
            serde_json::to_string(
                &all_session_infos_map
                    .values()
                    .cloned()
                    .collect::<HashSet<SessionInfo>>(),
            )
            .unwrap(),
        );

        *PREV_APP_IDS.lock().unwrap() = current_app_ids;
    }

    for session in manager.GetSessions().unwrap().into_iter() {
        let app_id = session
            .SourceAppUserModelId()
            .expect("Failed to get session ID")
            .to_string();

        // remove listeners for removed sessions
        if !is_app_allowed(&app_id) {
            let tokens = CALLBACK_TOKENS_MAP.lock().unwrap().remove(&app_id);

            if let Some(tokens) = tokens {
                let _ = session.RemovePlaybackInfoChanged(tokens.playback_info_changed);
                let _ = session.RemoveMediaPropertiesChanged(tokens.media_properties_changed);
                let _ = session.RemoveTimelinePropertiesChanged(tokens.timeline_properties_changed);

                PLAYBACK_INFO_CACHE.lock().unwrap().remove(&app_id);
            }

            continue;
        }

        // skip if session is already added or app is not allowed
        if CALLBACK_TOKENS_MAP.lock().unwrap().contains_key(&app_id) {
            continue;
        }

        let playback_info_changed_token = session
            .PlaybackInfoChanged(&TypedEventHandler::new(move |session, _| {
                match session {
                    Some(session) => {
                        handle_playback_info_changed(session);
                    }
                    None => {
                        log_warn("PlaybackInfoChanged event handler received None");
                    }
                }
                Ok(())
            }))
            .expect("Failed to set PlaybackInfoChanged event handler");

        let media_properties_changed_token = session
            .MediaPropertiesChanged(&TypedEventHandler::new(move |session, _| {
                match session {
                    Some(session) => {
                        handle_media_properties_changed(session);
                    }
                    None => {
                        log_warn("MediaPropertiesChanged event handler received None");
                    }
                }
                Ok(())
            }))
            .expect("Failed to set MediaPropertiesChanged event handler");

        let timeline_properties_changed_token = session
            .TimelinePropertiesChanged(&TypedEventHandler::new(move |session, _| {
                match session {
                    Some(session) => {
                        handle_timeline_properties_changed(session);
                    }
                    None => {
                        log_warn("TimelinePropertiesChanged event handler received None");
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
    }
}

fn handle_playback_info_changed(session: &GlobalSystemMediaTransportControlsSession) {
    let playback_info = session
        .GetPlaybackInfo()
        .expect("Failed to get playback info");

    let playback_state_smtc = playback_info
        .PlaybackStatus()
        .expect("Failed to get playback status");

    let state = match playback_state_smtc {
        GlobalSystemMediaTransportControlsSessionPlaybackStatus::Paused => PlaybackState::Paused,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus::Stopped => PlaybackState::Stopped,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus::Changing => PlaybackState::Waiting,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus::Closed
        | GlobalSystemMediaTransportControlsSessionPlaybackStatus::Opened => PlaybackState::None,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing => PlaybackState::Playing,
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

    let position = handle_timeline_properties_changed(session).position;

    let playback_info = PlaybackInfo {
        app_id: app_id.clone(),
        state,
        can_skip,
        position,
    };

    // insert into PLAYBACK_INFO_CACHE

    let mut cache = PLAYBACK_INFO_CACHE.lock().unwrap();
    cache.insert(app_id, playback_info.clone());

    on_playback_state_changed(serde_json::to_string(&playback_info).unwrap());
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

        let duration = handle_timeline_properties_changed(session).duration;

        let metadata_info = MetadataInfo {
            app_id: app_id.clone(),
            title,
            artist,
            album,
            album_artist,
            track_number,
            duration,
        };

        on_metadata_changed(serde_json::to_string(&metadata_info).unwrap());
    } else {
        log_warn("Failed to get media properties");
    }
}

fn handle_timeline_properties_changed(
    session: &GlobalSystemMediaTransportControlsSession,
) -> TimelineInfo {
    let timeline_properties = session
        .GetTimelineProperties()
        .expect("Failed to get timeline properties");

    let app_id = session
        .SourceAppUserModelId()
        .expect("Failed to get app ID")
        .to_string();

    // Duration is in ns, convert to ms
    let duration = (timeline_properties.EndTime().unwrap_or_default().Duration
        - timeline_properties.StartTime().unwrap_or_default().Duration)
        / 10000;
    let position = timeline_properties.Position().unwrap_or_default().Duration / 10000;

    let timeline_info = TimelineInfo {
        app_id: app_id.clone(),
        duration,
        position,
    };

    let cache = PLAYBACK_INFO_CACHE.lock().unwrap();

    let existing_playback_info = cache.get(&app_id);

    let playback_info = if let Some(existing_playback_info) = existing_playback_info {
        PlaybackInfo {
            app_id: app_id.clone(),
            state: existing_playback_info.state.clone(),
            can_skip: existing_playback_info.can_skip,
            position,
        }
    } else {
        PlaybackInfo {
            app_id: app_id.clone(),
            state: PlaybackState::None,
            can_skip: false,
            position,
        }
    };

    on_playback_state_changed(serde_json::to_string(&playback_info).unwrap());

    timeline_info
}
