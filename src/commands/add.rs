use std::io::Write;
use std::path::Path;

use crate::commands::get_master_password;
use crate::credentials::{self, ConnectionType, Device, OsType};
use crate::error::TelepromptError;
use crate::{ssh, telnet};

pub fn run(db_path: Option<&Path>, timeout_secs: u64) -> Result<(), TelepromptError> {
    let resolved_path = match db_path {
        Some(p) => p.to_path_buf(),
        None => credentials::get_default_db_path()?,
    };

    if !resolved_path.exists() {
        return Err(TelepromptError::NotInitialized);
    }

    let master_pwd = get_master_password()?;
    let mut store = credentials::load_store(&resolved_path, &master_pwd)?;

    println!("--- Add New Device ---");

    // 1. Device name
    let name = loop {
        let n = prompt_input("Device Name", None)?;
        if n.trim().is_empty() {
            println!("Device name cannot be empty.");
            continue;
        }
        if store.devices.contains_key(&n) {
            println!("Device '{}' already exists.", n);
            continue;
        }
        break n;
    };

    // 2. Host
    let host = loop {
        let h = prompt_input("Host/IP Address", None)?;
        if h.trim().is_empty() {
            println!("Host cannot be empty.");
            continue;
        }
        break h;
    };

    // 3. Connection type
    let conn_type_str = prompt_input("Connection Type (ssh/telnet) [ssh]", Some("ssh"))?;
    let connection_type = match conn_type_str.trim().to_lowercase().as_str() {
        "telnet" => ConnectionType::Telnet,
        _ => ConnectionType::Ssh,
    };

    // 4. Port
    let default_port = match connection_type {
        ConnectionType::Ssh => "22",
        ConnectionType::Telnet => "23",
    };
    let port_str = prompt_input(&format!("Port [{}]", default_port), Some(default_port))?;
    let port: u16 = port_str.trim().parse().unwrap_or(match connection_type {
        ConnectionType::Ssh => 22,
        ConnectionType::Telnet => 23,
    });

    // 5. Username
    let username = loop {
        let u = prompt_input("Username", None)?;
        if u.trim().is_empty() {
            println!("Username cannot be empty.");
            continue;
        }
        break u;
    };

    // 6. Authentication details
    let mut password = None;
    let mut key_path = None;

    match connection_type {
        ConnectionType::Ssh => {
            let auth_method = prompt_input("Auth Method (password/key) [password]", Some("password"))?;
            if auth_method.trim().to_lowercase() == "key" {
                let kp = prompt_input("SSH Private Key Path", None)?;
                key_path = Some(kp);
                // Prompt for password anyway in case key is encrypted or we need password for sudo
                print!("Sudo/Password (optional, press Enter to skip): ");
                std::io::stdout().flush().map_err(TelepromptError::Io)?;
                let pwd = rpassword::read_password().map_err(TelepromptError::Io)?;
                if !pwd.is_empty() {
                    password = Some(pwd);
                }
            } else {
                print!("Password: ");
                std::io::stdout().flush().map_err(TelepromptError::Io)?;
                let pwd = rpassword::read_password().map_err(TelepromptError::Io)?;
                password = Some(pwd);
            }
        }
        ConnectionType::Telnet => {
            print!("Password: ");
            std::io::stdout().flush().map_err(TelepromptError::Io)?;
            let pwd = rpassword::read_password().map_err(TelepromptError::Io)?;
            password = Some(pwd);
        }
    }

    // 6. OS Type Selection
    let os_type = OsType::prompt_selection(None)?;

    let mut device = Device {
        name: name.clone(),
        host,
        port,
        username,
        password,
        key_path,
        connection_type: connection_type.clone(),
        sudo_capable: false,
        sudo_password_required: false,
        os_type,
    };

    // 7. Test connection and detect sudo
    println!("\nTesting connection to {}...", device.name);
    let test_res = match connection_type {
        ConnectionType::Ssh => ssh::test_connection(&device, timeout_secs),
        ConnectionType::Telnet => telnet::test_connection(&device, timeout_secs),
    };

    match test_res {
        Ok(_) => {
            println!("✔ Connection successful!");
            if connection_type == ConnectionType::Ssh {
                println!("Checking sudo capability...");
                let mut mock_device = device.clone();
                if let Ok(_) = ssh::detect_sudo_capability(&mut mock_device, timeout_secs) {
                    device.sudo_capable = mock_device.sudo_capable;
                    device.sudo_password_required = mock_device.sudo_password_required;
                    if device.sudo_capable {
                        println!("✔ Sudo capable (Password required: {})", device.sudo_password_required);
                    } else {
                        println!("ℹ Sudo not available or access denied.");
                    }
                }
            }
        }
        Err(e) => {
            println!("✖ Connection failed: {}", e);
            print!("Do you still want to save this device? (y/N): ");
            std::io::stdout().flush().map_err(TelepromptError::Io)?;
            let mut answer = String::new();
            std::io::stdin().read_line(&mut answer).map_err(TelepromptError::Io)?;
            if !answer.trim().eq_ignore_ascii_case("y") {
                println!("Device not saved.");
                return Ok(());
            }
        }
    }

    // 8. Save
    store.devices.insert(name.clone(), device);
    credentials::save_store(&store, &resolved_path, &master_pwd)?;

    println!("✔ Device '{}' successfully saved!", name);
    Ok(())
}

fn prompt_input(label: &str, default: Option<&str>) -> Result<String, TelepromptError> {
    print!("{}: ", label);
    std::io::stdout().flush().map_err(TelepromptError::Io)?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).map_err(TelepromptError::Io)?;
    let input = input.trim();
    if input.is_empty() {
        if let Some(def) = default {
            return Ok(def.to_string());
        }
    }
    Ok(input.to_string())
}
