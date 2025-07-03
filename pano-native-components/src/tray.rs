use std::sync::{Arc, OnceLock};

use tokio::sync::mpsc;

pub struct PanoTrayData {
    pub tooltip: String,
    pub icon_argb: Vec<u8>,
    pub icon_dim: u32,
    pub menu_items: Vec<(String, String)>,
}

struct PanoTray {
    pub data: PanoTrayData,
    pub callback: Arc<dyn Fn(String) + Send + Sync + 'static>,
}

static TOKIO_USER_EVENT_SENDER: OnceLock<mpsc::Sender<PanoTrayData>> = OnceLock::new();

pub fn update_tray(pano_tray_data: PanoTrayData) {
    if let Some(sender) = TOKIO_USER_EVENT_SENDER.get() {
        // todo make it reliable by using async
        sender.try_send(pano_tray_data).unwrap_or_else(|_| {
            eprintln!("Failed to send tray event");
        });
    } else {
        eprintln!("Event loop not running");
    }
}

use ksni::{Icon, MenuItem, TrayMethods, menu::StandardItem};
impl ksni::Tray for PanoTray {
    fn id(&self) -> String {
        "com.arn.scrobble.tray".into()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        vec![Icon {
            width: self.data.icon_dim as i32,
            height: self.data.icon_dim as i32,
            data: self.data.icon_argb.clone(),
        }]
    }

    const MENU_ON_ACTIVATE: bool = true;

    fn title(&self) -> String {
        "Pano Scrobbler".to_string()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: "Pano Scrobbler".to_string(),
            description: self.data.tooltip.clone(),
            ..Default::default()
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        self.data
            .menu_items
            .iter()
            .map(|(id, text)| {
                let id_owned = id.clone();
                let text_owned = text.clone();
                match id.as_str() {
                    "Separator" => MenuItem::Separator,
                    _ => MenuItem::Standard(StandardItem {
                        label: text_owned,
                        activate: Box::new(move |tray: &mut PanoTray| {
                            (tray.callback)(id_owned.clone());
                        }),
                        enabled: id != "Dummy",
                        ..Default::default()
                    }),
                }
            })
            .collect()
    }
}

pub async fn tray_listener(
    tray_callback: impl Fn(String) + Send + Sync + 'static,
) -> Result<(), Box<dyn std::error::Error>> {
    let (sender, mut receiver) = mpsc::channel::<PanoTrayData>(1);
    TOKIO_USER_EVENT_SENDER.set(sender).unwrap();

    let tray_handle: OnceLock<ksni::Handle<PanoTray>> = OnceLock::new();
    let mut tray_init_attempted = false;
    let tray_callback = Arc::new(tray_callback);

    while let Some(tray_data) = receiver.recv().await {
        if !tray_init_attempted {
            tray_init_attempted = true;

            let tray = PanoTray {
                data: tray_data,
                callback: tray_callback.clone(),
            };
            let handle = tray.spawn().await?;
            let _ = tray_handle.set(handle);
        } else if let Some(handle) = tray_handle.get() {
            handle
                .update(|existing_tray| {
                    existing_tray.data = tray_data;
                })
                .await;
        }
    }

    Ok(())
}
