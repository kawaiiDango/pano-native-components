#[derive(Debug, Clone)]
pub enum WebViewIncomingEvent {
    LaunchWebView(String, String, String, String),
    SendCallback(String, String),
    DeleteAndQuit,
}
#[derive(Debug)]
pub enum WebViewOutgoingEvent {
    WebViewCallback(String, Vec<String>),
}
