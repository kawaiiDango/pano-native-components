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

    unsafe impl Send for SingleInstance {}
    unsafe impl Sync for SingleInstance {}

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
    use nix::sys::socket::{self, UnixAddr};
    use std::os::fd::{AsRawFd, OwnedFd};

    use super::SingleInstanceError;

    /// A struct representing one running instance.
    pub struct SingleInstance {
        maybe_sock: Option<OwnedFd>,
    }

    impl SingleInstance {
        /// Returns a new SingleInstance object.
        pub fn new(name: &str) -> Result<Self, SingleInstanceError> {
            let addr =
                UnixAddr::new_abstract(name.as_bytes()).map_err(|_| SingleInstanceError::Nul)?;
            let sock = socket::socket(
                socket::AddressFamily::Unix,
                socket::SockType::Stream,
                // If we fork and exec, then make sure the child process doesn't
                // hang on to this file descriptor.
                socket::SockFlag::SOCK_CLOEXEC,
                None,
            )
            .map_err(|_| SingleInstanceError::Nul)?;

            let maybe_sock = match socket::bind(sock.as_raw_fd(), &addr) {
                Ok(()) => Some(sock),
                Err(e) => {
                    eprintln!("Error binding socket: {e}");
                    return Err(SingleInstanceError::MutexError);
                }
            };

            Ok(Self { maybe_sock })
        }

        /// Returns whether this instance is single.
        pub fn is_single(&self) -> bool {
            self.maybe_sock.is_some()
        }
    }

    impl Drop for SingleInstance {
        fn drop(&mut self) {
            self.maybe_sock.take();
        }
    }
}
