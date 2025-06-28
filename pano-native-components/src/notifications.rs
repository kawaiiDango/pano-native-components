use notify_rust::Notification;

const AUMID: &str = "com.arn.scrobble";

pub fn notify(title: &str, body: &str, icon_path: &str) {
    // #[cfg(target_os = "windows")]
    // {
    //     use crate::windows_utils;
    //     use std::sync::Once;

    //     static ONCE: Once = Once::new();

    //     ONCE.call_once(|| {
    //         let result = windows_utils::register_aumid_if_needed(icon_path);

    //         if let Err(e) = result {
    //             eprintln!("Error registering AUMID: {e}");
    //         }
    //     });
    // }

    let mut notification = Notification::new();

    notification.summary(title).body(body).timeout(10000);

    #[cfg(target_os = "windows")]
    notification.app_id(AUMID);

    #[cfg(target_os = "linux")]
    notification.appname("pano-scrobbler").icon(icon_path);

    if let Err(e) = notification.show() {
        eprintln!("Error showing notification: {e:?}");
    }
}
