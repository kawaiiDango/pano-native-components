use std::sync::OnceLock;

use crate::{pano_tray::PanoTray, user_event::UserEvent};
use tokio::sync::mpsc;

static TOKIO_USER_EVENT_SENDER: OnceLock<mpsc::Sender<UserEvent>> = OnceLock::new();

pub fn send_tokio_event(user_event: UserEvent) {
    if let Some(sender) = TOKIO_USER_EVENT_SENDER.get() {
        // todo make it reliable by using async
        sender.try_send(user_event).unwrap_or_else(|_| {
            eprintln!("Failed to send user event");
        });
    } else {
        eprintln!("Event loop not running");
    }
}

#[cfg(target_os = "linux")]
use ksni::{Icon, MenuItem, TrayMethods, menu::StandardItem};
#[cfg(target_os = "linux")]
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
                            send_tokio_event(UserEvent::JniCallback(
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
pub async fn tokio_event_loop(jni_callback: impl Fn(String, String) + 'static) {
    let (sender, mut receiver) = mpsc::channel::<UserEvent>(100);
    TOKIO_USER_EVENT_SENDER.set(sender).unwrap();

    #[cfg(target_os = "linux")]
    let tray_handle: OnceLock<ksni::Handle<PanoTray>> = OnceLock::new();
    #[cfg(target_os = "linux")]
    let mut tray_init_attempted = false;

    while let Some(event) = receiver.recv().await {
        match event {
            UserEvent::JniCallback(fn_name, str_arg) => {
                jni_callback(fn_name, str_arg);
            }

            #[cfg(target_os = "linux")]
            UserEvent::UpdateTray(new_tray) => {
                if !tray_init_attempted {
                    tray_init_attempted = true;

                    let tray = PanoTray {
                        icon_dim: new_tray.icon_dim,
                        icon_argb: new_tray.icon_argb.clone(),
                        tooltip: new_tray.tooltip.clone(),
                        menu_items: new_tray.menu_items.clone(),
                    };
                    let handle_res = tray.spawn().await;
                    match handle_res {
                        Ok(handle) => {
                            let _ = tray_handle.set(handle);
                        }
                        Err(e) => {
                            eprintln!("Error creating tray: {e}");
                        }
                    }
                } else if let Some(handle) = tray_handle.get() {
                    handle
                        .update(|existing_tray| {
                            *existing_tray = new_tray;
                        })
                        .await;
                }
            }
        }
    }
}
