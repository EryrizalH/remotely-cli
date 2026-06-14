use std::path::Path;

use crate::commands::get_master_password;
use crate::credentials::{self, ConnectionType};
use crate::error::TelepromptError;
use crate::{ssh, telnet};

pub fn run(db_path: Option<&Path>, name: &str, timeout_secs: u64, verbose: bool) -> Result<(), TelepromptError> {
    let resolved_path = match db_path {
        Some(p) => p.to_path_buf(),
        None => credentials::get_default_db_path()?,
    };

    if !resolved_path.exists() {
        return Err(TelepromptError::NotInitialized);
    }

    let master_pwd = get_master_password()?;
    let store = credentials::load_store(&resolved_path, &master_pwd)?;

    let device = store.devices.get(name)
        .ok_or_else(|| TelepromptError::DeviceNotFound(name.to_string()))?;

    println!("Testing connection to '{}' ({}://{}:{})...", device.name, match device.connection_type {
        ConnectionType::Ssh => "ssh",
        ConnectionType::Telnet => "telnet",
    }, device.host, device.port);

    let test_res = match device.connection_type {
        ConnectionType::Ssh => ssh::test_connection(device, timeout_secs, verbose),
        ConnectionType::Telnet => telnet::test_connection(device, timeout_secs, verbose),
    };

    match test_res {
        Ok(_) => {
            println!("✔ Connection test successful!");
            Ok(())
        }
        Err(e) => {
            println!("✖ Connection test failed!");
            Err(e)
        }
    }
}
