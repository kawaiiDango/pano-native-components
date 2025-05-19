#[cfg(target_os = "macos")]
mod macos_simple_loop;

#[cfg(target_os = "linux")]
mod linux_tokio_loop;

mod winit_loop;

#[cfg(target_os = "macos")]
pub use macos_simple_loop::{event_loop, send_user_event};

#[cfg(target_os = "linux")]
pub use linux_tokio_loop::{
    send_tokio_user_event as send_user_event, tokio_event_loop as event_loop,
};

#[cfg(target_os = "windows")]
pub use winit_loop::{event_loop, send_user_event};

pub fn dummy_icon(size: u32) -> Vec<u8> {
    vec![200; (size * size * 4) as usize]
}
