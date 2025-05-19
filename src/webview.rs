use std::rc::Rc;

use winit::window::Window;
use wry::{PageLoadEvent, WebView, WebViewBuilder};

use crate::webview_event::WebViewEvent;

pub fn create_webview(
    window: &Window,
    url: String,
    callback_prefix: String,
    on_page_load: Box<dyn Fn(String)>,
) -> WebView {
    let on_page_load1 = Rc::new(on_page_load);
    let on_page_load2 = on_page_load1.clone();

    WebViewBuilder::new()
        .with_url(&url)
        .with_navigation_handler(move |url| {
            let is_callback = url.starts_with(&callback_prefix);

            if is_callback {
                on_page_load1(url.clone());
                false
            } else {
                true
            }
        })
        .with_on_page_load_handler(move |page_load_event, url| match page_load_event {
            PageLoadEvent::Finished => {}

            PageLoadEvent::Started => {
                on_page_load2(url.clone());
            }
        })
        .build_as_child(&window)
        .unwrap()
}

pub fn get_cookies_for_url(webview: &WebView, url: String) -> WebViewEvent {
    let cookies = webview
        .cookies_for_url(&url)
        .unwrap_or_default()
        .iter()
        .map(|cookie| format!("{}={}", cookie.name(), cookie.value(),))
        .collect::<Vec<_>>();

    WebViewEvent { url, cookies }
}
