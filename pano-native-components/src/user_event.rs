use crate::pano_tray::PanoTray;

#[derive(Debug)]
pub enum UserEvent {
    #[cfg(target_os = "linux")]
    UpdateTray(PanoTray),
    JniCallback(String, String),
}
