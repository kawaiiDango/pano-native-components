#[derive(Debug, Clone)]
pub enum WebViewIncomingEvent {
    LaunchWebView(String, String, String, String, String, i32),
    SendCallback(String, String),
    DeleteAndQuit,
}
#[derive(Debug)]
pub enum WebViewOutgoingEvent {
    WebViewCallback(String, Vec<String>),
}
