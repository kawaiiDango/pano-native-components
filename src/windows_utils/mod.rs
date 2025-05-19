#[cfg(target_os = "windows")]
mod utils;

#[cfg(target_os = "windows")]
pub use utils::{
    AUMID, add_remove_startup, allow_dark_mode_for_app, apply_dark_mode_to_window,
    is_added_to_startup, register_aumid_if_needed,
};
