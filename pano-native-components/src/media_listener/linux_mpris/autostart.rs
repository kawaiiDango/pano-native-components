use std::env;
use std::fs;
use std::path::PathBuf;

use ashpd::desktop::background::Background;

pub fn autostart(add: bool) {
    let desktop_file = env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".config"))
        })
        .map(|x| x.join("autostart/pano-scrobbler.desktop"));

    let desktop_file = match desktop_file {
        Some(path) => path,
        None => {
            log::error!("Could not determine autostart file path (HOME not set)");
            return;
        }
    };

    if add {
        let exec_path = if let Ok(appimage) = env::var("APPIMAGE") {
            appimage
        } else {
            match env::current_exe() {
                Ok(path) => path.display().to_string(),
                Err(e) => {
                    log::error!("Failed to get current executable path: {e}");
                    return;
                }
            }
        };

        // Escape embedded double-quotes before wrapping in quotes per XDG spec
        let escaped = exec_path.replace('"', "\\\"");
        let exec_command = format!("\"{escaped}\" --minimized");

        let contents = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=Pano Scrobbler\n\
             Comment=Feature packed music tracker\n\
             Terminal=false\n\
             Exec={exec_command}\n\
             Icon=pano-scrobbler\n\
             X-GNOME-Autostart-enabled=true\n\
             StartupWMClass=pano-scrobbler\n\
             Categories=AudioVideo;Audio;\n"
        );

        if let Some(parent) = desktop_file.parent()
            && let Err(e) = fs::create_dir_all(parent)
        {
            log::error!("Failed to create autostart directory: {e}");
            return;
        }

        if let Err(e) = fs::write(&desktop_file, contents) {
            log::error!("Failed to write autostart file: {e}");
        }
    } else {
        if let Err(e) = fs::remove_file(&desktop_file) {
            log::error!("Failed to remove autostart file: {e}");
        }
    }
}

pub async fn autostart_sandboxed(add: bool) {
    match Background::request()
        .reason("Start Pano Scrobbler on login")
        .auto_start(add)
        .command(["pano-scrobbler", "--minimized"])
        .dbus_activatable(false)
        .send()
        .await
    {
        Err(e) => log::error!("Failed to send background autostart request: {e}"),
        Ok(response) => {
            if let Err(e) = response.response() {
                log::error!("Background autostart request denied: {e}");
            }
        }
    }
}
