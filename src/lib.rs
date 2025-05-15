mod event_loop;
mod machine_uid;
mod media_info_structs;
mod media_listener;

#[cfg(not(target_os = "macos"))]
mod notifications;

mod pano_tray;
mod single_instance;
mod user_event;
mod windows_utils;

use event_loop::send_user_event;
use jni::sys::{jboolean, jint, jlong};

use jni::JNIEnv;
use jni::objects::{JClass, JIntArray, JObject, JObjectArray, JString, ReleaseMode};

use media_info_structs::IncomingPlayerEvent;
use media_listener::listener;
use pano_tray::PanoTray;
use single_instance::SingleInstance;
use tokio::sync::mpsc;
use user_event::UserEvent;

use std::collections::HashSet;
use std::env;
use std::sync::{LazyLock, Mutex};

static INCOMING_PLAYER_EVENT_TX: LazyLock<Mutex<Option<mpsc::Sender<IncomingPlayerEvent>>>> =
    LazyLock::new(|| Mutex::new(None));
static APP_IDS_ALLOW_LIST: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

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
pub extern "system" fn Java_com_arn_scrobble_PanoNativeComponents_setAllowedAppIds(
    mut env: JNIEnv,
    _class: JClass,
    app_ids: JObjectArray, // this is a java string array
) {
    // replace the current allow list with the new one
    let mut new_allow_list = HashSet::<String>::new();
    for i in 0..env.get_array_length(&app_ids).unwrap() {
        let value = env.get_object_array_element(&app_ids, i);
        let app_id = value.unwrap();
        let app_id = env.get_string(&JString::from(app_id)).unwrap().into();
        new_allow_list.insert(app_id);
    }

    *APP_IDS_ALLOW_LIST.lock().unwrap() = new_allow_list;
    send_incoming_player_event(IncomingPlayerEvent::RefreshSessions);
}

pub fn is_app_allowed(app_id: &str) -> bool {
    // true
    APP_IDS_ALLOW_LIST.lock().unwrap().contains(app_id)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_PanoNativeComponents_stopListeningMedia(
    _env: JNIEnv,
    _class: JClass,
) {
    send_incoming_player_event(IncomingPlayerEvent::Shutdown);
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_PanoNativeComponents_stopEventLoop(
    _env: JNIEnv,
    _class: JClass,
) {
    send_user_event(UserEvent::ShutdownEventLoop);
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

    send_incoming_player_event(IncomingPlayerEvent::Skip(app_id));
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

    send_incoming_player_event(IncomingPlayerEvent::Mute(app_id));
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
    send_incoming_player_event(IncomingPlayerEvent::Unmute(app_id));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_PanoNativeComponents_notify(
    mut env: JNIEnv,
    _class: JClass,
    title: JString,
    body: JString,
    icon_path: JString,
) {
    #[cfg(not(target_os = "macos"))]
    {
        let title: String = env.get_string(&title).unwrap().into();
        let body: String = env.get_string(&body).unwrap().into();
        let icon_path: String = env.get_string(&icon_path).unwrap().into();

        notifications::notify(&title, &body, &icon_path);
    }
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
    let tooltip: String = env
        .get_string(&tooltip)
        .expect("Couldn't get java string!")
        .into();

    let len: usize = env.get_array_length(&argb).unwrap().try_into().unwrap();

    let argb_rust = unsafe {
        env.get_array_elements(&argb, ReleaseMode::NoCopyBack)
            .unwrap()
    };

    // convert the argb packed ints to argb bytes

    let mut icon_rgba = Vec::<u8>::with_capacity(len * 4);

    for &argb in argb_rust.iter() {
        let a = (argb >> 24) as u8;
        let r = (argb >> 16) as u8;
        let g = (argb >> 8) as u8;
        let b = argb as u8;

        icon_rgba.push(r);
        icon_rgba.push(g);
        icon_rgba.push(b);
        icon_rgba.push(a);
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

    send_user_event(UserEvent::UpdateTray(PanoTray {
        tooltip,
        icon_rgba,
        icon_dim,
        menu_items,
    }));
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
pub extern "system" fn Java_com_arn_scrobble_PanoNativeComponents_isSingleInstance(
    mut env: JNIEnv,
    _class: JClass,
    name: JString,
) -> jboolean {
    let name: String = env
        .get_string(&name)
        .expect("Couldn't get java string!")
        .into();

    let instance = SingleInstance::new(&name);

    match instance {
        Ok(instance) => {
            if instance.is_single() {
                Box::leak(Box::new(instance));
                1
            } else {
                drop(instance);
                0
            }
        }
        Err(e) => {
            eprintln!("Error creating single instance: {e}");
            0
        }
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_PanoNativeComponents_addRemoveStartupWin(
    mut env: JNIEnv,
    _class: JClass,
    exe_path: JString,
    add: jboolean,
) -> jboolean {
    #[cfg(target_os = "windows")]
    {
        let exe_path: String = env.get_string(&exe_path).unwrap().into();
        let add = add != 0;

        if let Err(e) = windows_utils::add_remove_startup(&exe_path, add) {
            eprintln!("Error adding/removing from startup: {e}");
            0
        } else {
            1
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        0
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_PanoNativeComponents_isAddedToStartupWin(
    mut env: JNIEnv,
    _class: JClass,
    exe_path: JString,
) -> jboolean {
    #[cfg(target_os = "windows")]
    {
        let exe_path: String = env.get_string(&exe_path).unwrap().into();

        match windows_utils::is_added_to_startup(&exe_path) {
            Ok(is_added) => is_added as jboolean,
            Err(e) => {
                eprintln!("Error checking if added to startup: {e}");
                0
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        0
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_PanoNativeComponents_applyDarkModeToWindow(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    #[cfg(target_os = "windows")]
    {
        windows_utils::apply_dark_mode_to_window(handle);
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

pub fn send_incoming_player_event(incoming_event: IncomingPlayerEvent) {
    let tx = INCOMING_PLAYER_EVENT_TX.lock().unwrap();

    if let Some(ref sender) = *tx {
        match sender.blocking_send(incoming_event) {
            Ok(_) => {}
            Err(e) => eprintln!("Error sending message to channel: {e}"),
        }
    } else {
        eprintln!("Sender not initialized, did not send {incoming_event:?}");
    }
}

pub fn log_info(msg: &str) {
    send_user_event(UserEvent::JniCallback(
        "onLogInfo".to_string(),
        msg.to_string(),
    ));
}

pub fn log_warn(msg: &str) {
    send_user_event(UserEvent::JniCallback(
        "onLogWarn".to_string(),
        msg.to_string(),
    ));
}

pub fn on_active_sessions_changed(json_data: String) {
    send_user_event(UserEvent::JniCallback(
        "onActiveSessionsChanged".to_string(),
        json_data,
    ));
}

pub fn on_metadata_changed(json_data: String) {
    send_user_event(UserEvent::JniCallback(
        "onMetadataChanged".to_string(),
        json_data,
    ));
}

pub fn on_playback_state_changed(json_data: String) {
    send_user_event(UserEvent::JniCallback(
        "onPlaybackStateChanged".to_string(),
        json_data,
    ));
}

// pub fn on_timeline_properties_changed(json_data: String) {
//     STRING_CHANNEL_TX
//         .get()
//         .unwrap()
//         .send(("onTimelinePropertiesChanged", json_data))
//         .unwrap();
// }

fn string_tx_call_me_back(env: &mut JNIEnv, callback: &JObject, java_method_name: &str, msg: &str) {
    let java_msg = env
        .new_string(msg)
        .unwrap_or_else(|_| panic!("Couldn't create java string for {java_method_name}"));
    let result = env.call_method(
        callback,
        java_method_name,
        "(Ljava/lang/String;)V",
        &[(&java_msg).into()],
    );

    if let Err(e) = result {
        eprintln!("Error calling java method {java_method_name}: {e}");
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_PanoNativeComponents_startEventLoop(
    env: JNIEnv,
    _class: JClass,
    callback: JObject,
) {
    let global_ref_callback = env.new_global_ref(callback).unwrap();
    let jvm = env.get_java_vm().unwrap();
    event_loop::event_loop(move |method_name, msg| {
        let mut env = jvm.attach_current_thread().unwrap();
        string_tx_call_me_back(&mut env, &global_ref_callback, &method_name, &msg);
    });
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_PanoNativeComponents_startListeningMedia(
    _env: JNIEnv,
    _class: JClass,
) {
    if let Err(e) = listener() {
        // zbus::Error::Unsupported is a dummy error that I create on linux
        log_warn(&format!("Error listening for media: {e}"));
    }
}
