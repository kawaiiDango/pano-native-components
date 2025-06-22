use windows::Win32::Foundation::{HMODULE, HWND};
use windows::Win32::Globalization::{
    GetLocaleInfoEx, GetUserDefaultLocaleName, LOCALE_SISO639LANGNAME, LOCALE_SISO3166CTRYNAME,
};
use windows::Win32::Graphics::Dwm::{DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};
use windows::core::{PCSTR, PCWSTR, s};
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

pub fn get_language_country_codes() -> Result<(String, String), Box<dyn std::error::Error>> {
    unsafe {
        // Get the current user's locale name
        let mut locale_name = [0u16; 85]; // LOCALE_NAME_MAX_LENGTH
        let result = GetUserDefaultLocaleName(&mut locale_name);
        if result == 0 {
            return Err("Failed to get user default locale name".into());
        }

        let locale_name_ptr = PCWSTR::from_raw(locale_name.as_ptr());

        // Get language code (ISO 639)
        let mut lang_buffer = [0u16; 10];
        let lang_result = GetLocaleInfoEx(
            locale_name_ptr,
            LOCALE_SISO639LANGNAME,
            Some(&mut lang_buffer),
        );

        if lang_result == 0 {
            return Err("Failed to get language code".into());
        }

        // Get country code (ISO 3166)
        let mut country_buffer = [0u16; 10];
        let country_result = GetLocaleInfoEx(
            locale_name_ptr,
            LOCALE_SISO3166CTRYNAME,
            Some(&mut country_buffer),
        );

        if country_result == 0 {
            return Err("Failed to get country code".into());
        }

        // Convert UTF-16 to String
        let language = String::from_utf16_lossy(&lang_buffer[..lang_result as usize - 1]);
        let country = String::from_utf16_lossy(&country_buffer[..country_result as usize - 1]);

        Ok((language, country))
    }
}
