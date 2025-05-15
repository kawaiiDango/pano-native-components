#[cfg(target_os = "windows")]
mod utils;

#[cfg(target_os = "windows")]
pub use utils::add_remove_startup;

#[cfg(target_os = "windows")]
pub use utils::is_added_to_startup;

#[cfg(target_os = "windows")]
pub use utils::register_aumid_if_needed;

#[cfg(target_os = "windows")]
pub use utils::AUMID;

#[cfg(target_os = "windows")]
pub use utils::apply_dark_mode_to_window;

#[cfg(target_os = "windows")]
pub use utils::allow_dark_mode_for_app;
