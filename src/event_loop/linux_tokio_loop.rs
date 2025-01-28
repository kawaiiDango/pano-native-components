use std::sync::OnceLock;

use crate::{pano_tray::PanoTray, user_event::UserEvent};
use ksni::{Icon, MenuItem, TrayMethods, menu::StandardItem};
use tokio::sync::mpsc;

use super::dummy_icon;

static USER_EVENT_SENDER: OnceLock<mpsc::Sender<UserEvent>> = OnceLock::new();

pub fn send_user_event(user_event: UserEvent) {
    if let Some(sender) = USER_EVENT_SENDER.get() {
        // todo make it reliable by using async
        sender.try_send(user_event).unwrap();
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
        self.tooltip.clone()
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let menu = self
            .menu_items
            .iter()
            .map(|(id, text)| {
                let id_owned = id.clone();
                let text_owned = text.clone();
                match id.as_str() {
                    "Separator" => MenuItem::Separator,
                    _ => MenuItem::Standard(StandardItem {
                        label: text_owned,
                        activate: Box::new(move |_| {
                            send_user_event(UserEvent::JniCallback(
                                "onTrayMenuItemClicked".to_string(),
                                id_owned.clone(),
                            ));
                        }),
                        ..Default::default()
                    }),
                }
            })
            .collect();

        menu
    }
}

#[tokio::main(flavor = "current_thread")]
pub async fn event_loop(mut jni_callback: impl FnMut(String, String) + 'static) {
    let (sender, mut receiver) = mpsc::channel::<UserEvent>(1);
    USER_EVENT_SENDER.set(sender).unwrap();

    let tray = PanoTray {
        tooltip: "Pano Scrobbler".to_string(),
        icon_argb: dummy_icon(64),
        icon_dim: 64,
        menu_items: vec![],
    };

    let handle_res = tray.spawn().await;

    if let Err(e) = &handle_res {
        eprintln!("Error creating tray: {}", e);
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
        }
    }
}
