#[cfg(target_os = "windows")]
mod utils;

#[cfg(target_os = "windows")]
pub use utils::{apply_dark_mode_to_window, get_language_country_codes, is_file_locked};
