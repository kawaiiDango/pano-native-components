#[cfg(target_os = "windows")]
mod utils;

#[cfg(target_os = "windows")]
pub use utils::{allow_dark_mode_for_app, apply_dark_mode_to_window};
