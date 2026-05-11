#[derive(Debug, Clone)]
pub enum WebViewIncomingEvent {
    LaunchWebView(String, String, String, String, String, i32),
    #[cfg(target_os = "windows")]
    SendCallback(String, String),
    Close,
    Quit,
}
#[derive(Debug)]
pub enum WebViewOutgoingEvent {
    WebViewCallback(String, Vec<String>),
}
