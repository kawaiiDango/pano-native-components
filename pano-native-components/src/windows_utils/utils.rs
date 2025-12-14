use std::fs::OpenOptions;
use std::os::windows::fs::OpenOptionsExt;

use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::{DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute};
// use windows_registry::CURRENT_USER;

// const REG_NAME: &str = "Pano Scrobbler";

// pub fn register_aumid_if_needed(aumid: &str, icon_path: &str) -> Result<(), Box<dyn std::error::Error>> {
//     let key_path = format!("Software\\Classes\\AppUserModelId\\{aumid}");

//     let exists = CURRENT_USER.open(&key_path).is_ok();

//     if !exists {
//         let key = CURRENT_USER.create(&key_path)?;
//         key.set_string("DisplayName", REG_NAME)?;
//         key.set_string("IconUri", icon_path)?;
//         key.set_string("IconBackgroundColor", "0")?;
//     }
//     Ok(())
// }

// taken from tao

/*
pub fn allow_dark_mode_for_app(is_dark_mode: bool) {
    let huxtheme: isize = unsafe { LoadLibraryA(s!("uxtheme.dll")).unwrap_or_default().0 as _ };

    #[repr(C)]
    enum PreferredAppMode {
        Default,
        AllowDark,
        // ForceDark,
        // ForceLight,
        // Max,
    }
    const UXTHEME_SETPREFERREDAPPMODE_ORDINAL: u16 = 135;
    type SetPreferredAppMode = unsafe extern "system" fn(PreferredAppMode) -> PreferredAppMode;
    let set_preferred_app_mode: Option<SetPreferredAppMode> = unsafe {
        if HMODULE(huxtheme as _).is_invalid() {
            return;
        }

        GetProcAddress(
            HMODULE(huxtheme as _),
            PCSTR::from_raw(UXTHEME_SETPREFERREDAPPMODE_ORDINAL as usize as *mut _),
        )
        .map(|handle| std::mem::transmute(handle))
    };

    if let Some(_set_preferred_app_mode) = set_preferred_app_mode {
        let mode = if is_dark_mode {
            PreferredAppMode::AllowDark
        } else {
            PreferredAppMode::Default
        };
        unsafe { _set_preferred_app_mode(mode) };
    }
}
*/

pub fn apply_dark_mode_to_window(handle: i64) {
    let use_dark: i32 = 1;
    let hwnd = HWND(handle as _);

    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &use_dark as *const _ as _,
            std::mem::size_of_val(&use_dark) as u32,
        )
        .unwrap_or_else(|e| {
            eprintln!("Failed to set DWMWA_USE_IMMERSIVE_DARK_MODE for window {handle}: {e}");
        });
    }
}

pub fn is_file_locked(path: &str) -> bool {
    let file = OpenOptions::new()
        .create_new(false)
        .write(false)
        .read(true)
        .share_mode(0)
        .open(path);

    match file {
        Ok(_) => false,
        Err(err) => {
            // eprintln!("Error opening file '{path}': {err}");
            err.raw_os_error() == Some(32) // ERROR_SHARING_VIOLATION
        }
    }
}
