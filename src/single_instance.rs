pub use self::inner::*;
use std::error::Error;
use std::fmt;

// this code is from https://github.com/WLBF/single-instance/tree/master
// but reimplemented to use the windows crate instead

#[derive(Debug)]
pub enum SingleInstanceError {
    Nul,
    MutexError,
}

impl fmt::Display for SingleInstanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SingleInstanceError::Nul => write!(f, "Wide string null error"),

            SingleInstanceError::MutexError => {
                write!(f, "CreateMutex failed")
            }
        }
    }
}

impl Error for SingleInstanceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[cfg(target_os = "windows")]
mod inner {
    use std::ptr::null_mut;

    use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
    use windows::Win32::System::Threading::CreateMutexW;
    use windows::core::PCWSTR;

    use super::SingleInstanceError;

    /// A struct representing one running instance.
    pub struct SingleInstance {
        handle: Option<HANDLE>,
    }

    impl SingleInstance {
        /// Returns a new SingleInstance object.
        pub fn new(name: &str) -> Result<Self, SingleInstanceError> {
            // Convert the name to a wide string (UTF-16) for Windows APIs.
            let wide_name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
            let name_pcwstr = PCWSTR(wide_name.as_ptr());

            unsafe {
                // Create a named mutex.
                let handle = CreateMutexW(Some(null_mut()), false, name_pcwstr);
                match handle {
                    Ok(handle) => {
                        if handle.is_invalid() || GetLastError() == ERROR_ALREADY_EXISTS {
                            // Handle is invalid, meaning an error occurred.
                            return Err(SingleInstanceError::MutexError);
                        }
                        Ok(Self {
                            handle: Some(handle),
                        })
                    }
                    Err(_e) => {
                        // Check if the error is ERROR_ALREADY_EXISTS.
                        // if e.code() == ERROR_ALREADY_EXISTS {
                        //     // Another instance is already running.
                        // }
                        Err(SingleInstanceError::Nul)
                    }
                }
            }
        }

        /// Returns whether this instance is single.
        pub fn is_single(&self) -> bool {
            self.handle.is_some()
        }
    }

    impl Drop for SingleInstance {
        fn drop(&mut self) {
            if let Some(handle) = self.handle.take() {
                unsafe {
                    let _ = CloseHandle(handle);
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod inner {
    use super::SingleInstanceError;
    use nix::fcntl::{Flock, FlockArg};
    use std::fs::{File, OpenOptions};

    /// A struct representing one running instance.
    pub struct SingleInstance {
        maybe_flock: Option<Flock<File>>,
    }

    impl SingleInstance {
        /// Returns a new SingleInstance object.
        pub fn new(name: &str) -> Result<Self, SingleInstanceError> {
            // Place your lock file in the system temp directory
            let mut lock_path = std::env::temp_dir();
            lock_path.push(name);

            // Open (or create) the lock file
            let file = OpenOptions::new()
                .create(true)
                .truncate(false)
                .write(true)
                .open(&lock_path);

            match file {
                Ok(file) => {
                    // Try to acquire an exclusive non‐blocking lock
                    match Flock::lock(file, FlockArg::LockExclusiveNonblock) {
                        Ok(f) => Ok(Self {
                            maybe_flock: Some(f),
                        }), // Lock acquired => no other instance
                        Err((_f, err_no)) => {
                            eprintln!("Error acquiring lock: {err_no}");
                            Err(SingleInstanceError::MutexError)
                        }
                    }
                }
                Err(_e) => {
                    // Handle the error (e.g., file not found, permission denied, etc.)
                    eprintln!("Error opening lock file");
                    Err(SingleInstanceError::Nul)
                }
            }
        }

        /// Returns whether this instance is single.
        pub fn is_single(&self) -> bool {
            self.maybe_flock.is_some()
        }
    }

    impl Drop for SingleInstance {
        fn drop(&mut self) {
            // File will be closed automatically, releasing the lock
            self.maybe_flock = None;
        }
    }
}
