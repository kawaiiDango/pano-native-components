mod tao_loop;
mod webview_event;

use jni::JNIEnv;
use jni::objects::{JClass, JString};

use crate::webview_event::{WebViewIncomingEvent, WebViewOutgoingEvent};

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_DesktopWebView_startEventLoop(
    env: JNIEnv,
    _class: JClass,
) {
    // on proprietary nvidia drivers, set WEBKIT_DISABLE_DMABUF_RENDERER=1
    // by checking if /proc/driver/nvidia/version exists

    #[cfg(target_os = "linux")]
    if std::path::Path::new("/proc/driver/nvidia/version").exists() {
        unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
        eprintln!("Using proprietary nvidia driver workaround");
    }

    let jvm = env.get_java_vm().unwrap();
    tao_loop::event_loop(move |event| {
        let mut env = jvm.attach_current_thread().unwrap();

        match event {
            WebViewOutgoingEvent::WebViewCookies(url, cookies_vec) => {
                let desktop_webview_class =
                    env.find_class("com/arn/scrobble/DesktopWebView").unwrap();
                let string_class = env.find_class("java/lang/String").unwrap();
                let empty_string = env.new_string("").unwrap();
                let url = env.new_string(url).unwrap();

                let cookies = env
                    .new_object_array(cookies_vec.len() as i32, string_class, empty_string)
                    .unwrap();

                for (i, cookie) in cookies_vec.into_iter().enumerate() {
                    let cookie_str = env.new_string(cookie).unwrap();
                    env.set_object_array_element(&cookies, i as i32, cookie_str)
                        .unwrap();
                }

                env.call_static_method(
                    desktop_webview_class,
                    "onWebViewCookies",
                    "(Ljava/lang/String;[Ljava/lang/String;)V",
                    &[(&url).into(), (&cookies).into()],
                )
                .unwrap();
            }

            WebViewOutgoingEvent::WebViewUrlLoaded(url) => {
                let class = env.find_class("com/arn/scrobble/DesktopWebView").unwrap();
                let url = env.new_string(url).unwrap();

                env.call_static_method(
                    class,
                    "onWebViewUrlLoaded",
                    "(Ljava/lang/String;)V",
                    &[(&url).into()],
                )
                .unwrap();
            }
        }
    });
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_DesktopWebView_launchWebView(
    mut env: JNIEnv,
    _class: JClass,
    url: JString,
    callback_prefix: JString,
    data_dir: JString,
) {
    let url: String = env
        .get_string(&url)
        .expect("Couldn't get java string!")
        .into();

    let callback_prefix: String = env
        .get_string(&callback_prefix)
        .expect("Couldn't get java string!")
        .into();

    let data_dir: String = env
        .get_string(&data_dir)
        .expect("Couldn't get java string!")
        .into();

    tao_loop::send_incoming_webview_event(WebViewIncomingEvent::LaunchWebView(
        url,
        callback_prefix,
        data_dir,
    ));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_DesktopWebView_getWebViewCookiesFor(
    mut env: JNIEnv,
    _class: JClass,
    url: JString,
) {
    let url: String = env
        .get_string(&url)
        .expect("Couldn't get java string!")
        .into();

    tao_loop::send_incoming_webview_event(WebViewIncomingEvent::WebViewCookiesFor(url));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_DesktopWebView_quitWebView(
    _env: JNIEnv,
    _class: JClass,
) {
    tao_loop::send_incoming_webview_event(WebViewIncomingEvent::QuitWebView);
}
