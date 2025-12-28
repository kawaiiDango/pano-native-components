mod machine_uid;
mod media_events;
mod media_listener;

#[cfg(target_os = "linux")]
mod file_picker;
#[cfg(target_os = "linux")]
mod tray;

mod discord;
mod ipc;
mod jni_callback;
mod theme_observer;
mod windows_utils;

use jni::sys::{jboolean, jint, jlong};

use jni::JNIEnv;
use jni::objects::{JClass, JIntArray, JObject, JObjectArray, JString};

use jni_callback::JniCallback;
use media_events::IncomingEvent;
use media_listener::listener;
use tokio::sync::mpsc;

use std::env;
use std::sync::{LazyLock, Mutex};

use crate::discord::DiscordActivity;
use crate::media_events::{MetadataInfo, PlaybackInfo};

static INCOMING_PLAYER_EVENT_TX: LazyLock<Mutex<Option<mpsc::Sender<IncomingEvent>>>> =
    LazyLock::new(|| Mutex::new(None));

// This `#[no_mangle]` keeps rust from "mangling" the name and making it unique
// for this crate. The name follow a strict naming convention so that the
// JNI implementation will be able to automatically find the implementation
// of a native method based on its name.
//
// The `'local` lifetime here represents the local frame within which any local
// (temporary) references to Java objects will remain valid.
//
// It's usually not necessary to explicitly name the `'local` input lifetimes but
// in this case we want to return a reference and show the compiler what
// local frame lifetime it is associated with.
//
// Alternatively we could instead return the `jni::sys::jstring` type instead
// which would represent the same thing as a raw pointer, without any lifetime,
// and at the end use `.into_raw()` to convert a local reference with a lifetime
// into a raw pointer.

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_PanoNativeComponents_ping<'local>(
    // Notice that this `env` argument is mutable. Any `JNIEnv` API that may
    // allocate new object references will take a mutable reference to the
    // environment.
    mut env: JNIEnv<'local>,
    // this is the class that owns our static method. Not going to be used, but
    // still needs to have an argument slot
    _class: JClass<'local>,
    input: JString<'local>,
) -> JString<'local> {
    // First, we have to get the string out of java. Check out the `strings`
    // module for more info on how this works.
    let input: String = env
        .get_string(&input)
        .expect("Couldn't get java string!")
        .into();

    // Then we have to create a new java string to return. Again, more info
    // in the `strings` module.

    println!("Ping: {input}");

    env.new_string(input).expect("Couldn't create java string!")
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_PanoNativeComponents_refreshSessions(
    _env: JNIEnv,
    _class: JClass,
) {
    send_incoming_event(IncomingEvent::RefreshSessions);
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_PanoNativeComponents_stopListeningMedia(
    _env: JNIEnv,
    _class: JClass,
) {
    send_incoming_event(IncomingEvent::Shutdown);
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_PanoNativeComponents_setEnvironmentVariable(
    mut env: JNIEnv,
    _class: JClass,
    key: JString,
    value: JString,
) {
    let key: String = env.get_string(&key).unwrap().into();
    let value: String = env.get_string(&value).unwrap().into();
    unsafe {
        env::set_var(key, value);
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_PanoNativeComponents_skip(
    mut env: JNIEnv,
    _class: JClass,
    app_id: JString,
) {
    let app_id: String = env
        .get_string(&app_id)
        .expect("Couldn't get java string!")
        .into();

    send_incoming_event(IncomingEvent::Skip(app_id));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_PanoNativeComponents_mute(
    mut env: JNIEnv,
    _class: JClass,
    app_id: JString,
) {
    let app_id: String = env
        .get_string(&app_id)
        .expect("Couldn't get java string!")
        .into();

    send_incoming_event(IncomingEvent::Mute(app_id));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_PanoNativeComponents_unmute(
    mut env: JNIEnv,
    _class: JClass,
    app_id: JString,
) {
    let app_id: String = env
        .get_string(&app_id)
        .expect("Couldn't get java string!")
        .into();
    send_incoming_event(IncomingEvent::Unmute(app_id));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_PanoNativeComponents_notify(
    mut env: JNIEnv,
    _class: JClass,
    title: JString,
    body: JString,
) {
    let title: String = env.get_string(&title).unwrap().into();
    let body: String = env.get_string(&body).unwrap().into();

    let event = IncomingEvent::Notification(title, body);
    send_incoming_event(event);
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_PanoNativeComponents_setTray(
    mut env: JNIEnv,
    _class: JClass,
    tooltip: JString,
    argb: JIntArray,
    icon_dim: jint,
    menu_item_ids: JObjectArray,
    menu_item_texts: JObjectArray,
) {
    #[cfg(target_os = "linux")]
    {
        use jni::objects::ReleaseMode;

        use crate::tray::{PanoTrayData, update_tray};

        let tooltip: String = env
            .get_string(&tooltip)
            .expect("Couldn't get java string!")
            .into();

        let len: usize = env.get_array_length(&argb).unwrap().try_into().unwrap();

        let argb_rust = unsafe {
            env.get_array_elements(&argb, ReleaseMode::NoCopyBack)
                .unwrap()
        };

        let mut icon_argb = Vec::<u8>::with_capacity(len * 4);

        for &argb in argb_rust.iter() {
            let a = (argb >> 24) as u8;
            let r = (argb >> 16) as u8;
            let g = (argb >> 8) as u8;
            let b = argb as u8;

            icon_argb.push(a);
            icon_argb.push(r);
            icon_argb.push(b);
            icon_argb.push(g);
        }

        let icon_dim: u32 = icon_dim as u32;

        let len = env.get_array_length(&menu_item_ids).unwrap();
        let mut menu_items = Vec::<(String, String)>::with_capacity(len.try_into().unwrap());

        for i in 0..len {
            let id = env.get_object_array_element(&menu_item_ids, i);
            let id = id.unwrap();
            let id = env.get_string(&JString::from(id)).unwrap().into();

            let text = env.get_object_array_element(&menu_item_texts, i);
            let text = text.unwrap();
            let text = env.get_string(&JString::from(text)).unwrap().into();

            menu_items.push((id, text));
        }

        update_tray(PanoTrayData {
            tooltip,
            icon_argb,
            icon_dim,
            menu_items,
        });
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_PanoNativeComponents_getMachineId<'a>(
    env: JNIEnv<'a>,
    _class: JClass<'a>,
) -> JString<'a> {
    let id = machine_uid::get().unwrap();
    env.new_string(id).expect("Couldn't create java string!")
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_PanoNativeComponents_applyDarkModeToWindow(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    #[cfg(target_os = "windows")]
    windows_utils::apply_dark_mode_to_window(handle);
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_PanoNativeComponents_sendIpcCommand(
    mut env: JNIEnv,
    _class: JClass,
    command: JString,
    arg: JString,
) -> jboolean {
    let command: String = env
        .get_string(&command)
        .expect("Couldn't get java string!")
        .into();

    let arg: String = env
        .get_string(&arg)
        .expect("Couldn't get java string!")
        .into();

    match ipc::send_command(&command, &arg) {
        Ok(_) => 1, // true
        Err(e) => {
            if command != "focus-existing" {
                eprintln!("Error sending ipc command: {e}");
            }
            0 // false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_PanoNativeComponents_isFileLocked(
    mut env: JNIEnv,
    _class: JClass,
    path: JString,
) -> jboolean {
    #[cfg(target_os = "windows")]
    {
        let path: String = env
            .get_string(&path)
            .expect("Couldn't get java string!")
            .into();

        if windows_utils::is_file_locked(&path) {
            1 // true
        } else {
            0 // false
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        0
    }
}

// #[unsafe(no_mangle)]
// pub extern "system" fn  Java_com_arn_scrobble_media_DesktopMediaListenerWrapper_asyncComputation(
//     env: JNIEnv,
//     _class: JClass,
//     callback: JObject,
// ) {
//     // `JNIEnv` cannot be sent across thread boundaries. To be able to use JNI
//     // functions in other threads, we must first obtain the `JavaVM` interface
//     // which, unlike `JNIEnv` is `Send`.
//     let jvm = env.get_java_vm().unwrap();

//     // We need to obtain global reference to the `callback` object before sending
//     // it to the thread, to prevent it from being collected by the GC.
//     let callback = env.new_global_ref(callback).unwrap();

//     // Use channel to prevent the Java program to finish before the thread
//     // has chance to start.
//     let (tx, rx) = mpsc::channel();

//     let _ = thread::spawn(move || {
//         // Signal that the thread has started.
//         tx.send(()).unwrap();

//         // Use the `JavaVM` interface to attach a `JNIEnv` to the current thread.
//         let mut env = jvm.attach_current_thread().unwrap();

//         for i in 0..11 {
//             let progress = (i * 10) as jint;
//             // Now we can use all available `JNIEnv` functionality normally.
//             env.call_method(&callback, "asyncCallback", "(I)V", &[progress.into()])
//                 .unwrap();
//             thread::sleep(Duration::from_millis(100));
//         }

//         // The current thread is detached automatically when `env` goes out of scope.
//     });

//     // Wait until the thread has started.
//     rx.recv().unwrap();
// }

pub fn send_incoming_event(incoming_event: IncomingEvent) {
    let tx = INCOMING_PLAYER_EVENT_TX.lock().unwrap();

    if let Some(ref sender) = *tx {
        match sender.try_send(incoming_event) {
            Ok(_) => {}
            Err(e) => eprintln!("Error sending message to channel: {e}"),
        }
    } else {
        eprintln!("Sender not initialized, did not send {incoming_event:?}");
    }
}

fn call_java_fn(env: &mut JNIEnv, event: &JniCallback) -> Option<bool> {
    let result = match event {
        JniCallback::SessionsChanged(app_ids_to_names) => {
            let string_class = env.find_class("java/lang/String").unwrap();

            // Create a Java String array
            let app_ids = env
                .new_object_array(
                    app_ids_to_names.len() as jint,
                    &string_class,
                    JObject::null(),
                )
                .unwrap();
            let app_names = env
                .new_object_array(
                    app_ids_to_names.len() as jint,
                    &string_class,
                    JObject::null(),
                )
                .unwrap();

            // Populate the array
            for (i, (app_id, app_name)) in app_ids_to_names.iter().enumerate() {
                let j_app_id = env.new_string(app_id).unwrap();
                env.set_object_array_element(&app_ids, i as jint, j_app_id)
                    .unwrap();

                let j_app_name = env.new_string(app_name).unwrap();
                env.set_object_array_element(&app_names, i as jint, j_app_name)
                    .unwrap();
            }

            env.call_static_method(
                "com/arn/scrobble/PanoNativeComponents",
                "onActiveSessionsChanged",
                "([Ljava/lang/String;[Ljava/lang/String;)V",
                &[(&app_ids).into(), (&app_names).into()],
            )
        }

        JniCallback::MetadataChanged(
            app_id,
            MetadataInfo {
                track_id,
                title,
                artist,
                album,
                album_artist,
                track_number,
                duration,
                art_url,
            },
        ) => {
            let app_id = env.new_string(app_id).unwrap();
            let track_id = env.new_string(track_id).unwrap();
            let title = env.new_string(title).unwrap();
            let artist = env.new_string(artist).unwrap();
            let album = env.new_string(album).unwrap();
            let album_artist = env.new_string(album_artist).unwrap();
            let track_number = *track_number as jint;
            let duration = *duration as jlong;
            let art_url = env.new_string(art_url).unwrap();
            env.call_static_method(
                    "com/arn/scrobble/PanoNativeComponents",
                    "onMetadataChanged",
                    "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;IJLjava/lang/String;)V",
                    &[(&app_id).into(), (&track_id).into(), (&title).into(), (&artist).into(), (&album).into(), (&album_artist).into(), track_number.into(), duration.into(), (&art_url).into()],
                )
        }
        JniCallback::PlaybackStateChanged(
            app_id,
            PlaybackInfo {
                state,
                position,
                can_skip,
            },
        ) => {
            let app_id = env.new_string(app_id).unwrap();
            let state = env.new_string(state.to_string()).unwrap();
            let position = *position as jlong;
            let can_skip = *can_skip as jboolean;
            env.call_static_method(
                "com/arn/scrobble/PanoNativeComponents",
                "onPlaybackStateChanged",
                "(Ljava/lang/String;Ljava/lang/String;JZ)V",
                &[
                    (&app_id).into(),
                    (&state).into(),
                    position.into(),
                    can_skip.into(),
                ],
            )
        }

        JniCallback::IpcCallback(command, arg) => {
            let command = env.new_string(command).unwrap();
            let arg = env.new_string(arg).unwrap();
            env.call_static_method(
                "com/arn/scrobble/PanoNativeComponents",
                "onReceiveIpcCommand",
                "(Ljava/lang/String;Ljava/lang/String;)V",
                &[(&command).into(), (&arg).into()],
            )
        }

        #[cfg(target_os = "linux")]
        JniCallback::TrayItemClicked(item_id) => {
            let item_id = env.new_string(item_id).unwrap();
            env.call_static_method(
                "com/arn/scrobble/PanoNativeComponents",
                "onTrayMenuItemClicked",
                "(Ljava/lang/String;)V",
                &[(&item_id).into()],
            )
        }

        #[cfg(target_os = "linux")]
        JniCallback::FilePicked(req_id, uri) => {
            let uri = env.new_string(uri).unwrap();
            let req_id = *req_id as jint;

            env.call_static_method(
                "com/arn/scrobble/PanoNativeComponents",
                "onFilePicked",
                "(ILjava/lang/String;)V",
                &[req_id.into(), (&uri).into()],
            )
        }

        JniCallback::DarkModeChanged(is_dark_mode) => {
            let is_dark_mode = *is_dark_mode as jboolean;
            env.call_static_method(
                "com/arn/scrobble/PanoNativeComponents",
                "onDarkModeChange",
                "(Z)V",
                &[is_dark_mode.into()],
            )
        }

        JniCallback::IsAppIdAllowed(app_id) => {
            let app_id_j = env.new_string(app_id).unwrap();
            env.call_static_method(
                "com/arn/scrobble/PanoNativeComponents",
                "isAppIdAllowed",
                "(Ljava/lang/String;)Z",
                &[(&app_id_j).into()],
            )
        }
    };

    if let Err(e) = result {
        eprintln!("Error calling java method: {e}");
    } else if let Ok(ret_val) = result
        && let JniCallback::IsAppIdAllowed(_) = event
    {
        return Some(ret_val.z().unwrap_or(false));
    }

    None
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_PanoNativeComponents_startListeningMedia(
    env: JNIEnv,
    _class: JClass,
) {
    let jvm = env.get_java_vm().unwrap();
    if let Err(e) = listener(move |event| -> Option<bool> {
        let mut env = jvm.attach_current_thread().unwrap();
        call_java_fn(&mut env, &event)
    }) {
        eprintln!("Error listening for media: {e}");
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_PanoNativeComponents_updateDiscordActivity(
    mut env: JNIEnv,
    _class: JClass,
    client_id: JString,
    state: JString,
    details: JString,
    large_text: JString,
    start_time: jlong,
    end_time: jlong,
    art_url: JString,
    is_playing: jboolean,
    status_line: jint,
    button_texts: JObjectArray,
    button_urls: JObjectArray,
) -> jboolean {
    let client_id: String = env
        .get_string(&client_id)
        .expect("Couldn't get java string!")
        .into();
    let state: String = env
        .get_string(&state)
        .expect("Couldn't get java string!")
        .into();
    let details: String = env
        .get_string(&details)
        .expect("Couldn't get java string!")
        .into();
    let large_text: String = env
        .get_string(&large_text)
        .expect("Couldn't get java string!")
        .into();
    let art_url: String = env
        .get_string(&art_url)
        .expect("Couldn't get java string!")
        .into();

    let art_url = if art_url.is_empty() {
        None
    } else {
        Some(art_url)
    };

    let end_time = if end_time > 0 { Some(end_time) } else { None };
    let is_playing = is_playing != 0;

    let buttons_texts_and_urls = if button_texts.is_null() || button_urls.is_null() {
        Vec::new()
    } else {
        let len_texts = env.get_array_length(&button_texts).unwrap();
        let len_urls = env.get_array_length(&button_urls).unwrap();
        let len = len_texts.min(len_urls);

        let mut buttons = Vec::with_capacity(len.try_into().unwrap());

        for i in 0..len {
            let text_obj = env.get_object_array_element(&button_texts, i).unwrap();
            let url_obj = env.get_object_array_element(&button_urls, i).unwrap();

            let text: String = env.get_string(&JString::from(text_obj)).unwrap().into();
            let url: String = env.get_string(&JString::from(url_obj)).unwrap().into();

            if !text.is_empty() && !url.is_empty() {
                buttons.push((text, url));
            }
        }

        buttons
    };

    let activity = DiscordActivity::Playing {
        client_id,
        state,
        details,
        large_text,
        start_time,
        end_time,
        art_url,
        status_line,
        is_playing,
        buttons_texts_and_urls,
    };

    discord::discord_rpc(activity).is_ok() as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_PanoNativeComponents_clearDiscordActivity(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    discord::discord_rpc(DiscordActivity::Clear).is_ok() as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_PanoNativeComponents_stopDiscordActivity(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    discord::discord_rpc(DiscordActivity::Stop).is_ok() as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_PanoNativeComponents_xdgFileChooser(
    mut env: JNIEnv,
    _class: JClass,
    request_id: jint,
    save: jboolean,
    title: JString,
    file_name: JString,
    filters: JObjectArray,
) {
    #[cfg(target_os = "linux")]
    {
        let save = save != 0;
        let title: String = env
            .get_string(&title)
            .expect("Couldn't get java string!")
            .into();
        let file_name: String = env
            .get_string(&file_name)
            .expect("Couldn't get java string!")
            .into();
        let mut filters_vec = Vec::new();

        for i in 0..env.get_array_length(&filters).unwrap() {
            let value = env.get_object_array_element(&filters, i);
            let filter = value.unwrap();
            let filter: String = env.get_string(&JString::from(filter)).unwrap().into();
            filters_vec.push(filter);
        }

        let event =
            IncomingEvent::LaunchFilePicker(request_id, save, title, file_name, filters_vec);
        send_incoming_event(event);
    }
}
