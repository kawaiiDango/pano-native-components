use std::sync::OnceLock;
use tokio::sync::mpsc;

use crate::user_event::UserEvent;

static USER_EVENT_SENDER: OnceLock<mpsc::Sender<UserEvent>> = OnceLock::new();

pub fn event_loop(mut jni_callback: impl FnMut(String, String) + 'static) {
    let (sender, mut receiver) = mpsc::channel::<UserEvent>(100);

    USER_EVENT_SENDER.set(sender).unwrap();

    loop {
        match receiver.blocking_recv() {
            Some(UserEvent::JniCallback(fn_name, str_arg)) => {
                jni_callback(fn_name, str_arg);
            }
            None | Some(UserEvent::ShutdownEventLoop) => {
                break;
            }

            _ => {}
        }
    }
}

pub fn send_user_event(user_event: UserEvent) {
    if let Some(sender) = USER_EVENT_SENDER.get() {
        sender.blocking_send(user_event).unwrap();
    } else {
        eprintln!("Event loop not running");
    }
}
