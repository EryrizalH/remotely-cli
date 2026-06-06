use std::io::Write;
use std::path::Path;

use crate::commands::get_master_password;
use crate::credentials::{self, ConnectionType};
use crate::error::RemotelyError;
use crate::{ssh, telnet};

pub fn run(
    db_path: Option<&Path>,
    name: &str,
    command_args: &[String],
    timeout_secs: u64,
) -> Result<i32, RemotelyError> {
    let resolved_path = match db_path {
        Some(p) => p.to_path_buf(),
        None => credentials::get_default_db_path()?,
    };

    if !resolved_path.exists() {
        return Err(RemotelyError::NotInitialized);
    }

    let master_pwd = get_master_password()?;
    let store = credentials::load_store(&resolved_path, &master_pwd)?;

    let device = store.devices.get(name)
        .ok_or_else(|| RemotelyError::DeviceNotFound(name.to_string()))?;

    // Combine args into single command
    // If command_args is empty, we default to doing nothing or throwing error
    if command_args.is_empty() {
        return Err(RemotelyError::Cli("No remote command provided".to_string()));
    }
    let command = command_args.join(" ");

    // Execute based on connection type
    let (exit_code, stdout, stderr) = match device.connection_type {
        ConnectionType::Ssh => ssh::execute_command(device, &command, timeout_secs)?,
        ConnectionType::Telnet => telnet::execute_command(device, &command, timeout_secs)?,
    };

    // Print stdout and stderr to match remote output exactly
    if !stdout.is_empty() {
        std::io::stdout().write_all(&stdout).map_err(RemotelyError::Io)?;
        std::io::stdout().flush().map_err(RemotelyError::Io)?;
    }
    if !stderr.is_empty() {
        std::io::stderr().write_all(&stderr).map_err(RemotelyError::Io)?;
        std::io::stderr().flush().map_err(RemotelyError::Io)?;
    }

    Ok(exit_code)
}
