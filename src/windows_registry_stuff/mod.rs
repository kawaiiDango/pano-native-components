#[cfg(windows)]
mod registry_stuff;

#[cfg(windows)]
pub use registry_stuff::add_remove_startup;

#[cfg(windows)]
pub use registry_stuff::is_added_to_startup;

#[cfg(windows)]
pub use registry_stuff::register_aumid_if_needed;

#[cfg(windows)]
pub use registry_stuff::AUMID;