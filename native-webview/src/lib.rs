mod tao_loop;
mod webview_event;

use jni::EnvUnowned;
use jni::jni_sig;
use jni::jni_str;
use jni::objects::JObjectArray;
use jni::objects::{JClass, JString};

use crate::webview_event::{WebViewIncomingEvent, WebViewOutgoingEvent};

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_DesktopWebView_startEventLoop(
    mut unowned_env: EnvUnowned,
    _class: JClass,
) {
    // on proprietary nvidia drivers, set WEBKIT_DISABLE_DMABUF_RENDERER=1
    // by checking if /proc/driver/nvidia/version exists

    #[cfg(target_os = "linux")]
    if std::path::Path::new("/proc/driver/nvidia/version").exists() {
        unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
        eprintln!("Using proprietary nvidia driver workaround");
    }

    unowned_env
        .with_env(|env| -> jni::errors::Result<()> {
            let jvm = env.get_java_vm()?;
            tao_loop::event_loop(move |event| {
                jvm.attach_current_thread(|env| -> jni::errors::Result<()> {
                    let class = jni_str!("com/arn/scrobble/DesktopWebView");

                    match event {
                        WebViewOutgoingEvent::WebViewCallback(url, cookies_vec) => {
                            let url = JString::from_str(env, url)?;

                            let cookies = JObjectArray::<JString>::new(
                                env,
                                cookies_vec.len(),
                                JString::null(),
                            )?;

                            for (i, cookie) in cookies_vec.into_iter().enumerate() {
                                let cookie_str = JString::from_str(env, cookie)?;
                                cookies.set_element(env, i, cookie_str)?;
                            }

                            env.call_static_method(
                                class,
                                jni_str!("onCallback"),
                                jni_sig!("(Ljava/lang/String;[Ljava/lang/String;)V"),
                                &[(&url).into(), (&cookies).into()],
                            )?;
                        }
                    }
                    Ok(())
                })
                .unwrap();
            });
            Ok(())
        })
        .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_DesktopWebView_launchWebView(
    mut unowned_env: EnvUnowned,
    _class: JClass,
    url: JString,
    callback_prefix: JString,
    cookies_url: JString,
    data_dir: JString,
) {
    unowned_env
        .with_env(|env| -> jni::errors::Result<()> {
            let url: String = url.mutf8_chars(env)?.into();
            let callback_prefix: String = callback_prefix.mutf8_chars(env)?.into();
            let cookies_url: String = cookies_url.mutf8_chars(env)?.into();
            let data_dir: String = data_dir.mutf8_chars(env)?.into();

            tao_loop::send_incoming_webview_event(WebViewIncomingEvent::LaunchWebView(
                url,
                callback_prefix,
                cookies_url,
                data_dir,
            ));
            Ok(())
        })
        .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_arn_scrobble_DesktopWebView_deleteAndQuit(
    _env: EnvUnowned,
    _class: JClass,
) {
    tao_loop::send_incoming_webview_event(WebViewIncomingEvent::DeleteAndQuit);
}
