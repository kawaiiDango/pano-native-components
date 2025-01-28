use std::sync::OnceLock;

use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};
use winit::{
    application::ApplicationHandler,
    event_loop::{EventLoop, EventLoopProxy},
    platform::windows::EventLoopBuilderExtWindows,
};

use crate::{pano_tray::PanoTray, user_event::UserEvent};

use super::dummy_icon;

pub(crate) static USER_EVENT_SENDER: OnceLock<EventLoopProxy<UserEvent>> = OnceLock::new();

pub fn send_user_event(user_event: UserEvent) {
    if let Some(proxy) = USER_EVENT_SENDER.get() {
        match proxy.send_event(user_event) {
            Ok(_) => {}
            Err(e) => eprintln!("Error sending message to event loop: {}", e),
        }
    } else {
        eprintln!("Event loop not running");
    }
}

struct PanoTrayApplication {
    tray_icon: Option<TrayIcon>,
    jni_callback: Box<dyn FnMut(String, String)>,
}

impl PanoTrayApplication {
    fn new(jni_callback: Box<dyn FnMut(String, String)>) -> PanoTrayApplication {
        PanoTrayApplication {
            tray_icon: None,
            jni_callback,
        }
    }

    fn new_tray() -> TrayIcon {
        let icon = Icon::from_rgba(dummy_icon(64), 64, 64).unwrap();
        let menu = Menu::new();
        let item1 = MenuItem::new("item1", true, None);
        if let Err(err) = menu.append(&item1) {
            println!("{err:?}");
        }

        TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Pano Scrobbler")
            .with_icon(icon)
            .build()
            .unwrap()
    }

    fn update_tray(&mut self, pano_tray: &PanoTray) {
        if let Some(tray_icon) = &mut self.tray_icon {
            let icon = Icon::from_rgba(
                pano_tray.icon_argb.clone(),
                pano_tray.icon_dim,
                pano_tray.icon_dim,
            )
            .ok();
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
}

impl ApplicationHandler<UserEvent> for PanoTrayApplication {
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {}

    fn window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        _event: winit::event::WindowEvent,
    ) {
    }

    fn new_events(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        cause: winit::event::StartCause,
    ) {
        // We create the icon once the event loop is actually running
        // to prevent issues like https://github.com/tauri-apps/tray-icon/issues/90
        if winit::event::StartCause::Init == cause {
            self.tray_icon = Some(Self::new_tray());
        }
    }

    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::JniCallback(fn_name, str_arg) => {
                (self.jni_callback)(fn_name, str_arg);
            }

            UserEvent::UpdateTray(pano_tray) => {
                self.update_tray(&pano_tray);
            }

            UserEvent::ShutdownEventLoop => {
                event_loop.exit();
            }
        }
    }
}

pub fn event_loop(jni_callback: impl FnMut(String, String) + 'static) {
    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .with_any_thread(true)
        .build()
        .unwrap();

    let mut app = PanoTrayApplication::new(Box::new(jni_callback));

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
        eprintln!("Error running event loop: {}", e);
    };
}
