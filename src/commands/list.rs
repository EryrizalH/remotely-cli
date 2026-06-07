use comfy_table::Table;
use std::path::Path;

use crate::commands::get_master_password;
use crate::credentials::{self, ConnectionType};
use crate::error::TelepromptError;

pub fn run(db_path: Option<&Path>) -> Result<(), TelepromptError> {
    let resolved_path = match db_path {
        Some(p) => p.to_path_buf(),
        None => credentials::get_default_db_path()?,
    };

    if !resolved_path.exists() {
        return Err(TelepromptError::NotInitialized);
    }

    let master_pwd = get_master_password()?;
    let store = credentials::load_store(&resolved_path, &master_pwd)?;

    if store.devices.is_empty() {
        println!("No devices registered. Run 'teleprompt add' to register a new device.");
        return Ok(());
    }

    let mut table = Table::new();
    table.set_header(vec!["Device Name", "Host / IP", "Port", "User", "Type", "Sudo Access"]);

    // Sort devices by name for consistent output
    let mut sorted_keys: Vec<&String> = store.devices.keys().collect();
    sorted_keys.sort();

    for name in sorted_keys {
        let dev = store.devices.get(name).unwrap();
        let conn_type_str = match dev.connection_type {
            ConnectionType::Ssh => "SSH",
            ConnectionType::Telnet => "Telnet",
        };
        
        let sudo_str = if dev.sudo_capable {
            if dev.sudo_password_required {
                "Yes (pwd injected)"
            } else {
                "Yes (no pwd)"
            }
        } else {
            "No"
        };

        table.add_row(vec![
            &dev.name,
            &dev.host,
            &dev.port.to_string(),
            &dev.username,
            conn_type_str,
            sudo_str,
        ]);
    }

    println!("{}", table);
    Ok(())
}
