#[cfg(unix)]
mod unix_mpris;

#[cfg(windows)]
mod windows_smtc;

#[cfg(unix)]
pub use unix_mpris::listener;

#[cfg(windows)]
pub use windows_smtc::listener;
