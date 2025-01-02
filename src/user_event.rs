pub enum UserEvent {
    MenuEvent(tray_icon::menu::MenuEvent),
    JniCallback(String, String),
    UpdateTrayTooltip(String),
    UpdateTrayMenu(Vec<(String,String)>),
    UpdateTrayIcon(Vec<u8>, u32, u32),
}