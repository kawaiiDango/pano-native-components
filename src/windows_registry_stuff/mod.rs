#[cfg(target_os = "windows")]
mod registry_stuff;

#[cfg(target_os = "windows")]
pub use registry_stuff::add_remove_startup;

#[cfg(target_os = "windows")]
pub use registry_stuff::is_added_to_startup;

#[cfg(target_os = "windows")]
pub use registry_stuff::register_aumid_if_needed;

#[cfg(target_os = "windows")]
pub use registry_stuff::AUMID;
