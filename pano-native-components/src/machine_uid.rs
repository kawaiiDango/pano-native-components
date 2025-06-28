// adapted from the machine-uid crate

#[cfg(target_os = "linux")]
pub mod machine_id {
    use std::error::Error;
    use std::fs::File;
    use std::io::Read;

    // dbusPath is the default path for dbus machine id.
    const DBUS_PATH: &str = "/var/lib/dbus/machine-id";
    // or when not found (e.g. Fedora 20)
    const DBUS_PATH_ETC: &str = "/etc/machine-id";

    #[allow(dead_code)]
    fn read_file(file_path: &str) -> Result<String, Box<dyn Error>> {
        let mut fd = File::open(file_path)?;
        let mut content = String::new();
        fd.read_to_string(&mut content)?;
        Ok(content.trim().to_string())
    }

    /// Return machine id
    pub fn get_machine_id() -> Result<String, Box<dyn Error>> {
        match read_file(DBUS_PATH) {
            Ok(machine_id) => Ok(machine_id),
            Err(_) => Ok(read_file(DBUS_PATH_ETC)?),
        }
    }
}

#[cfg(target_os = "macos")]
mod machine_id {
    // machineID returns the uuid returned by `ioreg -rd1 -c IOPlatformExpertDevice`.
    use std::error::Error;
    use std::process::Command;

    /// Return machine id
    pub fn get_machine_id() -> Result<String, Box<dyn Error>> {
        let output = Command::new("ioreg")
            .args(&["-rd1", "-c", "IOPlatformExpertDevice"])
            .output()?;
        let content = String::from_utf8_lossy(&output.stdout);
        extract_id(&content)
    }

    fn extract_id(content: &str) -> Result<String, Box<dyn Error>> {
        let lines = content.split('\n');
        for line in lines {
            if line.contains("IOPlatformUUID") {
                let k: Vec<&str> = line.rsplitn(2, '=').collect();
                let id = k[0].trim_matches(|c: char| c == '"' || c.is_whitespace());
                return Ok(id.to_string());
            }
        }
        Err(From::from(
            "No matching IOPlatformUUID in `ioreg -rd1 -c IOPlatformExpertDevice` command.",
        ))
    }
}

#[cfg(target_os = "windows")]
pub mod machine_id {
    use std::error::Error;
    use windows_registry::LOCAL_MACHINE;

    /// Return machine id
    pub fn get_machine_id() -> Result<String, Box<dyn Error>> {
        let crypto = LOCAL_MACHINE.open("SOFTWARE\\Microsoft\\Cryptography")?;
        let id: String = crypto.get_string("MachineGuid")?;

        Ok(id.trim().to_string())
    }
}

pub use machine_id::get_machine_id as get;
