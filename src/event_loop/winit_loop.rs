use std::{path::PathBuf, sync::OnceLock};
use winit::{
    application::ApplicationHandler,
    event::{StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    window::{Window, WindowAttributes, WindowId},
};
use wry::{
    Rect, WebContext, WebView,
    dpi::{LogicalPosition, LogicalSize},
};

#[cfg(target_os = "windows")]
use crate::PanoTray;
#[cfg(target_os = "windows")]
use tray_icon::TrayIcon;

use crate::{user_event::UserEvent, webview::get_cookies_for_url};

pub(crate) static USER_EVENT_SENDER: OnceLock<EventLoopProxy<UserEvent>> = OnceLock::new();

pub fn send_user_event(user_event: UserEvent) {
    if let Some(proxy) = USER_EVENT_SENDER.get() {
        match proxy.send_event(user_event) {
            Ok(_) => {}
            Err(e) => eprintln!("Error sending message to event loop: {e}"),
        }
    } else {
        eprintln!("Event loop not running");
    }
}

struct PanoApplication {
    #[cfg(target_os = "windows")]
    tray_icon: Option<TrayIcon>,
    #[cfg(target_os = "windows")]
    jni_callback: Box<dyn Fn(String, String)>,
    initial_event: Option<UserEvent>,
    webview_window: Option<(Window, WebView, WebContext)>,
}

impl PanoApplication {
    #[cfg(target_os = "windows")]
    fn new(jni_callback: Box<dyn Fn(String, String)>) -> PanoApplication {
        PanoApplication {
            tray_icon: None,
            jni_callback,
            webview_window: None,
            initial_event: None,
        }
    }

    #[cfg(target_os = "linux")]
    fn new(initial_event: UserEvent) -> PanoApplication {
        PanoApplication {
            initial_event: Some(initial_event),
            webview_window: None,
        }
    }

    #[cfg(target_os = "windows")]
    fn new_tray() -> TrayIcon {
        use tray_icon::{
            Icon, TrayIconBuilder,
            menu::{Menu, MenuItem},
        };

        use crate::{event_loop::dummy_icon, windows_utils::allow_dark_mode_for_app};

        allow_dark_mode_for_app(true);

        let icon = Icon::from_rgba(dummy_icon(64), 64, 64).unwrap();
        let menu = Menu::new();
        let item1 = MenuItem::with_id("Dummy", "Initializing Pano Scrobbler", false, None);
        if let Err(err) = menu.append(&item1) {
            eprintln!("{err:?}");
        }

        TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Pano Scrobbler")
            .with_icon(icon)
            .build()
            .unwrap()
    }

    #[cfg(target_os = "windows")]
    fn update_tray(&mut self, pano_tray: &PanoTray) {
        use tray_icon::{
            Icon,
            menu::{Menu, MenuItem, PredefinedMenuItem},
        };

        if let Some(tray_icon) = &mut self.tray_icon {
            let mut icon_rgba = Vec::with_capacity(pano_tray.icon_argb.len());

            for pixel in pano_tray.icon_argb.chunks_exact(4) {
                // argb to rgba
                icon_rgba.push(pixel[1]);
                icon_rgba.push(pixel[2]);
                icon_rgba.push(pixel[3]);
                icon_rgba.push(pixel[0]);
            }

            let icon = Icon::from_rgba(icon_rgba, pano_tray.icon_dim, pano_tray.icon_dim).ok();
            let menu = Menu::new();
            for (id, text) in pano_tray.menu_items.iter() {
                match id.as_str() {
                    "Separator" => {
                        let _ = menu.append(&PredefinedMenuItem::separator());
                    }
                    _ => {
                        let _ = menu.append(&MenuItem::with_id(id, text, true, None));
                    }
                }
            }
            tray_icon.set_icon(icon).unwrap();
            tray_icon.set_tooltip(Some(&pano_tray.tooltip)).unwrap();
            tray_icon.set_menu(Some(Box::new(menu)));
        }
    }

    fn quit_webview(&mut self) {
        self.webview_window.take();
    }
}

impl ApplicationHandler<UserEvent> for PanoApplication {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        // We create the icon once the event loop is actually running
        // to prevent issues like https://github.com/tauri-apps/tray-icon/issues/90

        if StartCause::Init == cause {
            #[cfg(target_os = "windows")]
            {
                self.tray_icon = Some(Self::new_tray());
            }

            if let Some(event) = self.initial_event.take() {
                self.user_event(event_loop, event);
            }
        }
    }

    // Advance GTK event loop <!----- IMPORTANT
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        #[cfg(target_os = "linux")]
        while gtk::events_pending() {
            gtk::main_iteration_do(false);
        }
    }
    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::Resized(size) => {
                let (window, webview, _context) = self.webview_window.as_mut().unwrap();
                let size = size.to_logical::<u32>(window.scale_factor());

                webview
                    .set_bounds(Rect {
                        position: LogicalPosition::new(0, 0).into(),
                        size: LogicalSize::new(size.width, size.height).into(),
                    })
                    .unwrap();
            }
            WindowEvent::CloseRequested => {
                self.quit_webview();
            }
            _ => {}
        }
    }

    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::JniCallback(fn_name, str_arg) => {
                #[cfg(target_os = "windows")]
                (self.jni_callback)(fn_name, str_arg);
            }
            UserEvent::UpdateTray(pano_tray) => {
                #[cfg(target_os = "windows")]
                self.update_tray(&pano_tray);
            }

            UserEvent::LaunchWebview(url, callback_prefix, data_dir) => {
                let mut window_attributes = WindowAttributes::default()
                    .with_inner_size(LogicalSize::new(640.0, 480.0))
                    .with_title("WebView")
                    .with_active(true);

                #[cfg(target_os = "linux")]
                {
                    use winit::platform::x11::WindowAttributesExtX11;
                    window_attributes =
                        window_attributes.with_name("pano-scrobbler", "pano-scrobbler");
                }

                #[cfg(target_os = "windows")]
                {
                    use winit::platform::windows::WindowAttributesExtWindows;
                    window_attributes = window_attributes.with_class_name("pano-scrobbler")
                }

                let window = event_loop.create_window(window_attributes);

                match window {
                    Err(e) => {
                        eprintln!("Error creating window: {e}");
                    }

                    Ok(window) => {
                        let mut context = WebContext::new(Some(PathBuf::from(data_dir)));

                        let webview = crate::webview::create_webview(
                            &window,
                            &mut context,
                            url,
                            callback_prefix,
                            Box::new(|url| {
                                let new_event =
                                    UserEvent::JniCallback("onWebViewPageLoad".to_string(), url);

                                #[cfg(target_os = "windows")]
                                send_user_event(new_event);

                                #[cfg(target_os = "linux")]
                                crate::event_loop::linux_tokio_loop::send_tokio_user_event(
                                    new_event,
                                );
                            }),
                        );

                        #[cfg(target_os = "windows")]
                        {
                            // does not work properly on linux
                            window.focus_window();
                        }

                        self.webview_window = Some((window, webview, context));
                    }
                }
            }

            UserEvent::WebViewCookiesFor(url) => {
                if let Some((_, webview, _)) = &self.webview_window {
                    let webview_event = get_cookies_for_url(webview, url);
                    let new_event = UserEvent::JniCallback(
                        "onWebViewCookies".to_string(),
                        serde_json::to_string(&webview_event).unwrap(),
                    );

                    #[cfg(target_os = "windows")]
                    send_user_event(new_event);

                    #[cfg(target_os = "linux")]
                    {
                        // Send the event to the tokio thread
                        crate::event_loop::linux_tokio_loop::send_tokio_user_event(new_event);
                    }
                }
            }

            UserEvent::QuitWebview => {
                self.quit_webview();
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub fn event_loop(jni_callback: impl Fn(String, String) + 'static) {
    use tray_icon::menu::MenuEvent;
    use winit::platform::windows::EventLoopBuilderExtWindows;

    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .with_any_thread(true)
        .build()
        .unwrap();

    let mut app = PanoApplication::new(Box::new(jni_callback));

    let proxy = event_loop.create_proxy();
    USER_EVENT_SENDER.set(proxy).unwrap();

    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let _ = proxy.send_event(UserEvent::JniCallback(
            "onTrayMenuItemClicked".to_string(),
            event.id().0.clone(),
        ));
    }));

    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("Error running event loop: {e}");
    };
}

#[cfg(target_os = "linux")]
pub fn event_loop(launch_webview_event: UserEvent) {
    use gtk;
    use std::sync::Once;
    use winit::platform::x11::EventLoopBuilderExtX11;

    static INIT: Once = Once::new();

    INIT.call_once(|| {
        if let Err(e) = gtk::init() {
            eprintln!("Error initializing GTK: {e}");
        } else {
            gtk::glib::set_application_name("Pano Scrobbler");

            let event_loop_result = EventLoop::<UserEvent>::with_user_event()
                .with_any_thread(true)
                .build();

            match event_loop_result {
                Ok(event_loop) => {
                    let mut app = PanoApplication::new(launch_webview_event);

                    let proxy = event_loop.create_proxy();
                    USER_EVENT_SENDER.set(proxy).unwrap();

                    if let Err(e) = event_loop.run_app(&mut app) {
                        eprintln!("Error running winit event loop: {e}");
                    };
                }
                Err(e) => {
                    eprintln!("Error creating event loop: {e}");
                }
            }
        }
    });
}
