use std::sync::OnceLock;

use crate::{pano_tray::PanoTray, user_event::UserEvent};
use gtk::glib::once_cell::sync::OnceCell;
use ksni::{Icon, MenuItem, TrayMethods, menu::StandardItem};
use tokio::sync::mpsc;

use super::{
    dummy_icon,
    winit_loop::{self, send_user_event},
};

static TOKIO_USER_EVENT_SENDER: OnceLock<mpsc::Sender<UserEvent>> = OnceLock::new();

pub fn send_tokio_user_event(user_event: UserEvent) {
    if let Some(sender) = TOKIO_USER_EVENT_SENDER.get() {
        // todo make it reliable by using async
        sender.try_send(user_event).unwrap_or_else(|_| {
            eprintln!("Failed to send user event");
        });
    } else {
        eprintln!("Event loop not running");
    }
}

impl ksni::Tray for PanoTray {
    fn id(&self) -> String {
        "com.arn.scrobble.tray".into()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        vec![Icon {
            width: self.icon_dim as i32,
            height: self.icon_dim as i32,
            data: self.icon_argb.clone(),
        }]
    }

    const MENU_ON_ACTIVATE: bool = true;

    fn title(&self) -> String {
        "Pano Scrobbler".to_string()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: "Pano Scrobbler".to_string(),
            description: self.tooltip.clone(),
            ..Default::default()
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        self.menu_items
            .iter()
            .map(|(id, text)| {
                let id_owned = id.clone();
                let text_owned = text.clone();
                match id.as_str() {
                    "Separator" => MenuItem::Separator,
                    _ => MenuItem::Standard(StandardItem {
                        label: text_owned,
                        activate: Box::new(move |_| {
                            send_tokio_user_event(UserEvent::JniCallback(
                                "onTrayMenuItemClicked".to_string(),
                                id_owned.clone(),
                            ));
                        }),
                        enabled: id != "Dummy",
                        ..Default::default()
                    }),
                }
            })
            .collect()
    }
}

#[tokio::main(flavor = "current_thread")]
pub async fn tokio_event_loop(mut jni_callback: impl FnMut(String, String) + 'static) {
    let (sender, mut receiver) = mpsc::channel::<UserEvent>(100);
    TOKIO_USER_EVENT_SENDER.set(sender).unwrap();

    let tray = PanoTray {
        tooltip: "Pano Scrobbler".to_string(),
        icon_argb: dummy_icon(64),
        icon_dim: 64,
        menu_items: vec![(
            "Dummy".to_string(),
            "Initializing Pano Scrobbler".to_string(),
        )],
    };

    let handle_res = tray.spawn().await;

    let winit_thread_handle: OnceCell<std::thread::JoinHandle<()>> = OnceCell::new();

    if let Err(e) = &handle_res {
        eprintln!("Error creating tray: {e}");
    }

    loop {
        match receiver.recv().await {
            Some(UserEvent::JniCallback(fn_name, str_arg)) => {
                jni_callback(fn_name, str_arg);
            }

            Some(UserEvent::UpdateTray(new_tray)) => {
                if let Ok(handle) = &handle_res {
                    handle
                        .update(|existing_tray| {
                            *existing_tray = new_tray;
                        })
                        .await;
                }
            }

            None | Some(UserEvent::ShutdownEventLoop) => {
                if let Ok(handle) = &handle_res {
                    handle.shutdown().await;
                }
                break;
            }

            Some(UserEvent::LaunchWebview(url, callback_prefix, data_dir)) => {
                // if winit_thread_handle is Some, send a message to the existing winit loop
                if winit_thread_handle.get().is_some() {
                    send_user_event(UserEvent::LaunchWebview(url, callback_prefix, data_dir))
                } else {
                    // start winit loop in a new thread
                    let _ = winit_thread_handle.set(std::thread::spawn(move || {
                        winit_loop::event_loop(UserEvent::LaunchWebview(
                            url,
                            callback_prefix,
                            data_dir,
                        ));
                    }));
                }
            }
            Some(UserEvent::QuitWebview) => {
                if winit_thread_handle.get().is_some() {
                    send_user_event(UserEvent::QuitWebview);
                }
            }
            Some(UserEvent::WebViewCookiesFor(url)) => {
                if winit_thread_handle.get().is_some() {
                    send_user_event(UserEvent::WebViewCookiesFor(url));
                }
            }
        }
    }
}
