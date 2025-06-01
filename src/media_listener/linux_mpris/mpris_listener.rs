use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    time::Duration,
};

use futures_util::stream::StreamExt;
use tokio::{
    sync::{
        RwLock, RwLockWriteGuard,
        mpsc::{self, Receiver, Sender},
    },
    task::JoinHandle,
    time::timeout,
};
use zbus::{
    Connection,
    fdo::{DBusProxy, NameOwnerChangedArgs},
    zvariant::{self},
};

use crate::{
    INCOMING_PLAYER_EVENT_TX, is_app_allowed,
    media_info_structs::{IncomingPlayerEvent, MetadataInfo, PlaybackInfo, PlaybackState},
    media_listener::linux_mpris::{media_player2::MediaPlayer2Proxy, player::PlayerProxy},
    on_metadata_changed, on_playback_state_changed,
};
use crate::{
    media_info_structs::SessionInfo, media_listener::linux_mpris::metadata::Metadata,
    on_active_sessions_changed,
};

const MPRIS2_PREFIX: &str = "org.mpris.MediaPlayer2.";

struct PlayerListenerHandle {
    join_handle: JoinHandle<zbus::Result<()>>,
    incoming_player_event_tx: Sender<IncomingPlayerEvent>,
}

async fn get_identity(connection: &Connection, dbus_name: &str) -> String {
    let media_player2_proxy = match MediaPlayer2Proxy::new(connection, dbus_name).await {
        Ok(proxy) => proxy,
        Err(_) => return String::new(),
    };

    // some chromium instances await forever for identity

    match timeout(Duration::from_millis(200), media_player2_proxy.identity()).await {
        Ok(Ok(id)) => id,
        _ => String::new(),
    }
}

#[tokio::main(flavor = "current_thread")]
pub async fn listener() -> zbus::Result<()> {
    let (all_players_tx, mut all_players_rx) = mpsc::channel(100);
    *INCOMING_PLAYER_EVENT_TX.lock().unwrap() = Some(all_players_tx);

    let names_to_handles: RwLock<HashMap<String, PlayerListenerHandle>> =
        RwLock::new(HashMap::new());
    let session_infos: RwLock<HashSet<SessionInfo>> = RwLock::new(HashSet::new());

    let connection = Connection::session().await?;

    let dbus_proxy = DBusProxy::new(&connection).await?;

    // listener just started, poll existing values

    let dbus_names = dbus_proxy
        .list_names()
        .await?
        .into_iter()
        .filter(|name| name.starts_with(MPRIS2_PREFIX));

    for dbus_name in dbus_names {
        if !names_to_handles
            .read()
            .await
            .contains_key(dbus_name.as_str())
        {
            let session_info = SessionInfo {
                app_id: dbus_name.to_string(),
                app_name: get_identity(&connection, &dbus_name).await,
            };
            session_infos.write().await.insert(session_info);

            if is_app_allowed(&normalize_dbus_name(&dbus_name)) {
                start_tracking_player(
                    &connection,
                    dbus_name.to_string(),
                    &mut names_to_handles.write().await,
                );
            }
        }
    }

    let active_players = session_infos
        .read()
        .await
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    send_active_players(&active_players);

    // listen for new players
    let mut name_owner_changed = dbus_proxy.receive_name_owner_changed().await?;

    tokio::try_join!(
        async {
            while let Some(incoming_event) = all_players_rx.recv().await {
                match &incoming_event {
                    IncomingPlayerEvent::Skip(app_id) => {
                        let names_to_handles = names_to_handles.read().await;
                        let handle = names_to_handles.get(app_id);

                        if let Some(handle) = handle {
                            let _ = handle.incoming_player_event_tx.send(incoming_event).await;
                        }
                    }

                    IncomingPlayerEvent::Mute(app_id) => {
                        let names_to_handles = names_to_handles.read().await;
                        let handle = names_to_handles.get(app_id);

                        if let Some(handle) = handle {
                            let _ = handle.incoming_player_event_tx.send(incoming_event).await;
                        }
                    }

                    IncomingPlayerEvent::Unmute(app_id) => {
                        let names_to_handles = names_to_handles.read().await;
                        let handle = names_to_handles.get(app_id);

                        if let Some(handle) = handle {
                            let _ = handle.incoming_player_event_tx.send(incoming_event).await;
                        }
                    }

                    IncomingPlayerEvent::RefreshSessions => {
                        for session_info in session_infos.read().await.iter() {
                            let app_id_normalized = normalize_dbus_name(&session_info.app_id);
                            let is_allowed = is_app_allowed(&app_id_normalized);
                            let is_tracking = names_to_handles
                                .read()
                                .await
                                .contains_key(&app_id_normalized);

                            if is_allowed && !is_tracking {
                                start_tracking_player(
                                    &connection,
                                    session_info.app_id.to_string(),
                                    &mut names_to_handles.write().await,
                                );
                            }

                            if !is_allowed && is_tracking {
                                stop_tracking_player(
                                    names_to_handles.write().await.remove(&session_info.app_id),
                                );
                            }
                        }
                    }

                    IncomingPlayerEvent::Shutdown => {
                        for (_app_id, handle) in names_to_handles.write().await.drain() {
                            stop_tracking_player(Some(handle));
                        }

                        session_infos.write().await.clear();

                        // produce some error to stop the tasks
                        return zbus::Result::Err(zbus::Error::Unsupported);
                    }
                }
            }

            zbus::Result::Ok(())
        },
        async {
            while let Some(name_owner_changed) = name_owner_changed.next().await {
                let name_owner_changed: NameOwnerChangedArgs = name_owner_changed.args()?;
                let dbus_name: &str = name_owner_changed.name();

                if !dbus_name.starts_with(MPRIS2_PREFIX) {
                    continue;
                }

                let old_owner = name_owner_changed.old_owner();
                let new_owner = name_owner_changed.new_owner();

                // handle player added
                if old_owner.is_none()
                    && new_owner.is_some()
                    && !names_to_handles.read().await.contains_key(dbus_name)
                {
                    let session_info = SessionInfo {
                        app_id: dbus_name.to_string(),
                        app_name: get_identity(&connection, dbus_name).await,
                    };
                    session_infos.write().await.insert(session_info);

                    if is_app_allowed(&normalize_dbus_name(dbus_name)) {
                        start_tracking_player(
                            &connection,
                            dbus_name.to_string(),
                            &mut names_to_handles.write().await,
                        );
                    }
                }

                // handle player removed
                if old_owner.is_some() && new_owner.is_none() {
                    stop_tracking_player(names_to_handles.write().await.remove(dbus_name));

                    // remove the entry from session_infos
                    session_infos
                        .write()
                        .await
                        .retain(|session_info| session_info.app_id != dbus_name);
                }

                let active_players = session_infos
                    .read()
                    .await
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>();
                send_active_players(&active_players);
            }
            zbus::Result::Ok(())
        }
    )?;

    Ok(())
}

async fn player_listeners(
    connection: Connection,
    app_id: String,
    mut incoming_player_event_rx: Receiver<IncomingPlayerEvent>,
) -> zbus::Result<()> {
    let player_proxy = PlayerProxy::builder(&connection)
        .uncached_properties(&["Position"])
        .destination(app_id.clone())?
        .build()
        .await?;

    // todo: handle errors

    let mut prev_volume = player_proxy.volume().await.unwrap_or_default();

    let metadata = player_proxy.metadata().await.unwrap_or_default();
    parse_and_send_metadata(&app_id, metadata);

    let playback_status = player_proxy.playback_status().await.unwrap_or_default();
    let can_go_next = player_proxy.can_go_next().await.unwrap_or_default();
    let position = player_proxy
        .position()
        .await
        .map(|x| x / 1000)
        .unwrap_or(-1);
    parse_and_send_playback_state(&app_id, playback_status, can_go_next, position);

    let mut metadata_changed = player_proxy.receive_metadata_changed().await;
    let mut playback_status_changed = player_proxy.receive_playback_status_changed().await;
    let mut can_go_next_changed = player_proxy.receive_can_go_next_changed().await;
    let seek_changed = player_proxy.receive_seeked().await;
    // this does not work
    // let mut position_changed = player_proxy.receive_position_changed().await;

    tokio::try_join!(
        async {
            while let Some(metadata_changed) = metadata_changed.next().await {
                let metadata = metadata_changed.get().await.unwrap_or_default();
                parse_and_send_metadata(&app_id, metadata);
            }

            zbus::Result::Ok(())
        },
        async {
            while let Some(playback_status_changed) = playback_status_changed.next().await {
                let playback_status = playback_status_changed.get().await.unwrap_or_default();
                let position = player_proxy
                    .position()
                    .await
                    .map(|x| x / 1000)
                    .unwrap_or(-1);
                parse_and_send_playback_state(
                    &app_id,
                    playback_status,
                    player_proxy
                        .cached_can_go_next()
                        .unwrap_or_default()
                        .unwrap_or_default(),
                    position,
                );
            }
            zbus::Result::Ok(())
        },
        async {
            while let Some(can_go_next_changed) = can_go_next_changed.next().await {
                let can_go_next = can_go_next_changed.get().await.unwrap_or_default();
                let position = player_proxy
                    .position()
                    .await
                    .map(|x| x / 1000)
                    .unwrap_or(-1);
                parse_and_send_playback_state(
                    &app_id,
                    player_proxy
                        .cached_playback_status()
                        .unwrap_or_default()
                        .unwrap_or_default(),
                    can_go_next,
                    position,
                );
            }
            zbus::Result::Ok(())
        },
        // async {
        //     loop {
        //         if let Ok(position) = player_proxy.position().await {
        //             println!("Position: {:?}", position);
        //         } else {
        //             println!("Position: Error");
        //         }

        //         tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        //     }

        //     zbus::Result::Ok(())
        // },
        async {
            if let Ok(mut seek_changed) = seek_changed {
                while let Some(seek_changed) = seek_changed.next().await {
                    if let Ok(seek_args) = seek_changed.args() {
                        let position = *seek_args.Position() / 1000;
                        parse_and_send_playback_state(
                            &app_id,
                            player_proxy
                                .cached_playback_status()
                                .unwrap_or_default()
                                .unwrap_or_default(),
                            player_proxy
                                .cached_can_go_next()
                                .unwrap_or_default()
                                .unwrap_or_default(),
                            position,
                        );
                    }
                }
            }
            zbus::Result::Ok(())
        },
        async {
            while let Some(incoming_event) = incoming_player_event_rx.recv().await {
                match &incoming_event {
                    IncomingPlayerEvent::Skip(_) => {
                        let _ = player_proxy.next().await;
                    }

                    IncomingPlayerEvent::Mute(_) => {
                        prev_volume = player_proxy.volume().await.unwrap_or_default();
                        let _ = player_proxy.set_volume(0.0).await;
                    }

                    IncomingPlayerEvent::Unmute(_) => {
                        let _ = player_proxy.set_volume(prev_volume).await;
                    }

                    IncomingPlayerEvent::Shutdown => {
                        break;
                    }

                    _ => {}
                }
            }

            zbus::Result::Ok(())
        },
    )?;

    Ok(())
}

fn start_tracking_player(
    connection: &Connection,
    app_id: String,
    names_to_handles: &mut RwLockWriteGuard<'_, HashMap<String, PlayerListenerHandle>>,
) {
    let (tx, rx) = mpsc::channel::<IncomingPlayerEvent>(1);

    let join_handle = tokio::spawn(player_listeners(connection.clone(), app_id.clone(), rx));

    names_to_handles.insert(
        app_id,
        PlayerListenerHandle {
            join_handle,
            incoming_player_event_tx: tx,
        },
    );
}

fn stop_tracking_player(handle: Option<PlayerListenerHandle>) {
    if let Some(handle) = handle {
        handle.join_handle.abort();
    }
}

fn normalize_dbus_name(app_id: &str) -> String {
    let app_id = app_id.to_string();
    let app_id_splits = app_id.rsplit_once(".instance");

    match app_id_splits {
        Some((app_id, _)) => format!("{app_id}.instancen"),
        None => app_id,
    }
}

fn parse_and_send_metadata(app_id: &str, metadata: HashMap<String, zvariant::OwnedValue>) {
    // debug print
    // for (key, value) in &metadata {
    //     println!("  {}: {:?}", key, value);
    // }

    let metadata = Metadata::from(metadata);

    let first_artist = metadata.artists().unwrap_or_default().first().cloned();
    let first_album_artist = metadata
        .album_artists()
        .unwrap_or_default()
        .first()
        .cloned();

    let metadata_info = MetadataInfo {
        app_id: normalize_dbus_name(app_id),
        title: metadata.title().unwrap_or_default().to_string(),
        artist: first_artist.unwrap_or_default(),
        album: metadata.album_name().unwrap_or_default().to_string(),
        album_artist: first_album_artist.unwrap_or_default(),
        track_number: metadata.track_number().unwrap_or_default(),
        duration: metadata
            .length()
            .map(|x| x.as_millis() as i64)
            .unwrap_or(-1),
    };

    on_metadata_changed(serde_json::to_string(&metadata_info).unwrap());
}

fn parse_and_send_playback_state(
    app_id: &str,
    playback_status: String,
    can_go_next: bool,
    position: i64,
) {
    let playback_status = PlaybackState::from_str(&playback_status).unwrap_or(PlaybackState::Other);

    let playback_info = PlaybackInfo {
        app_id: normalize_dbus_name(app_id),
        state: playback_status,
        position,
        can_skip: can_go_next,
    };

    on_playback_state_changed(serde_json::to_string(&playback_info).unwrap());
}

fn send_active_players(active_players: &[SessionInfo]) {
    // replace .instance1234 at the end with .instancen

    let active_players = active_players
        .iter()
        .map(|session_info| {
            let app_id = normalize_dbus_name(&session_info.app_id);
            let app_name = session_info.app_name.clone();

            SessionInfo { app_id, app_name }
        })
        .collect::<HashSet<_>>();

    on_active_sessions_changed(serde_json::to_string(&active_players).unwrap());
}
