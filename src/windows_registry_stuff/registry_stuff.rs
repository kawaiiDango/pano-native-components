use windows_registry::CURRENT_USER;

const REG_PATH: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const REG_NAME: &str = "Pano Scrobbler";
pub const AUMID: &str = "com.arn.scrobble.desktop.notifications";

pub fn add_remove_startup(exe_path: &str, add: bool) -> Result<(), Box<dyn std::error::Error>> {
    // .open will throw an AccessDenied error on .set_string
    let key = CURRENT_USER.create(REG_PATH)?;

    if add {
        key.set_string(REG_NAME, format!("\"{}\" -m", exe_path))?;
    } else {
        key.remove_value(REG_NAME)?;
    }

    Ok(())
}

pub fn is_added_to_startup(exe_path: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let key = CURRENT_USER.open(REG_PATH)?;

    let result = key.get_string(REG_NAME);

    let is_added = match result {
        Ok(value) => value == format!("\"{}\" -m", exe_path),
        Err(_) => false,
    };

    Ok(is_added)
}

pub fn register_aumid_if_needed(icon_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let key_path = format!("Software\\Classes\\AppUserModelId\\{}", AUMID);

    let exists = CURRENT_USER.open(&key_path).is_ok();

    if !exists {
        let key = CURRENT_USER.create(&key_path)?;
        key.set_expand_string("DisplayName", REG_NAME)?;
        key.set_expand_string("IconUri", icon_path)?;
        key.set_string("IconBackgroundColor", "0")?;
    }
    Ok(())
}
