#[cfg(target_os = "windows")]
mod utils;

#[cfg(target_os = "windows")]
pub use utils::{apply_dark_mode_to_window, is_file_locked};
