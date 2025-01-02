use tao::{
    event::Event,
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    Icon, TrayIconBuilder,
};

use crate::{user_event::UserEvent, JAVA_CALLBACK_PROXY};

fn dummy_icon(width: u32, height: u32) -> Icon {
    Icon::from_rgba(vec![200; (width * height * 4) as usize], width, height).unwrap()
}

#[cfg(unix)]
fn build_event_loop() -> EventLoop<UserEvent> {
    use tao::platform::unix::EventLoopBuilderExtUnix;

    EventLoopBuilder::<UserEvent>::with_user_event()
        .with_any_thread(true)
        .build()
}

#[cfg(windows)]
fn build_event_loop() -> EventLoop<UserEvent> {
    use tao::platform::windows::EventLoopBuilderExtWindows;

    EventLoopBuilder::<UserEvent>::with_user_event()
        .with_any_thread(true)
        .build()
}

pub fn tao_event_loop(mut jni_callback: impl FnMut(String, String) + 'static) {
    let event_loop = build_event_loop();
    let mut tray_icon = None;

    // set a tray event handler that forwards the event and wakes up the event loop
    let proxy = event_loop.create_proxy();
    JAVA_CALLBACK_PROXY.get_or_init(|| proxy);

    let proxy = event_loop.create_proxy();

    // set a menu event handler that forwards the event and wakes up the event loop
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::MenuEvent(event));
    }));

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(tao::event::StartCause::Init) => {
                let icon = dummy_icon(64, 64);

                let tray_menu = Menu::new();
                let _ = tray_menu.append(
                        &MenuItem::with_id("dummy", "text", true, None)
                );

                // We create the icon once the event loop is actually running
                // to prevent issues like https://github.com/tauri-apps/tray-icon/issues/90
                tray_icon = Some(
                    TrayIconBuilder::new()
                        .with_menu(Box::new(tray_menu))
                        .with_tooltip("Pano Scrobbler")
                        .with_icon(icon)
                        .build()
                        .unwrap(),
                );
            }

            Event::UserEvent(UserEvent::MenuEvent(event)) => {
                jni_callback("onTrayMenuItemClicked".to_string(), event.id().0.clone());
            }

            Event::UserEvent(UserEvent::UpdateTrayIcon(rgba, width, height)) => {
                if let Some(tray_icon) = &mut tray_icon {

                    let icon = Icon::from_rgba(rgba, width, height).ok();
                    tray_icon.set_icon(icon).unwrap();
                }
            }

            Event::UserEvent(UserEvent::UpdateTrayTooltip(tooltip)) => {
                if let Some(tray_icon) = &mut tray_icon {
                    tray_icon.set_tooltip(Some(tooltip)).unwrap();
                }
            }

            Event::UserEvent(UserEvent::UpdateTrayMenu(menu_items)) => {
                let tray_menu = Menu::new();

                for (id, text) in menu_items.iter() {
                    match id.as_str() {
                        "Separator" => {
                            let _ = tray_menu.append(&PredefinedMenuItem::separator());
                        }
                        _ => {
                            let _ = tray_menu.append(&MenuItem::with_id(id, text, true, None));
                        }
                    }
                }

                if let Some(tray_icon) = &mut tray_icon {
                    tray_icon.set_menu(Some(Box::new(tray_menu)));
                }
            }

            Event::UserEvent(UserEvent::JniCallback(fn_name, str_arg)) => {
                jni_callback(fn_name, str_arg);
            }
            _ => {}
        }
    });
}
