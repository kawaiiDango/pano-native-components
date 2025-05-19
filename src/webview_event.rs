use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct WebViewEvent {
    pub url: String,
    pub cookies: Vec<String>,
}
