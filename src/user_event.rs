use crate::pano_tray::PanoTray;

#[derive(Debug)]
pub enum UserEvent {
    UpdateTray(PanoTray),
    JniCallback(String, String),
    LaunchWebview(String, String),
    WebViewCookiesFor(String),
    QuitWebview,
    ShutdownEventLoop,
}
