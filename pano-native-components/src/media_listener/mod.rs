#[cfg(target_os = "linux")]
mod linux_mpris;

#[cfg(target_os = "windows")]
mod windows_smtc;

#[cfg(target_os = "linux")]
pub use linux_mpris::listener;

#[cfg(target_os = "windows")]
pub use windows_smtc::listener;
