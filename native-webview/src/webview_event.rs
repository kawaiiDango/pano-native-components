#[derive(Debug, Clone)]
pub enum WebViewIncomingEvent {
    LaunchWebView(String, String, String),
    WebViewCookiesFor(String),
    WebViewUrlLoaded(String),
    QuitWebView,
}
#[derive(Debug)]
pub enum WebViewOutgoingEvent {
    WebViewCookies(String, Vec<String>),
    WebViewUrlLoaded(String),
}
