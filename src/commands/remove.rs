use std::io::Write;
use std::path::Path;

use crate::commands::get_master_password;
use crate::credentials;
use crate::error::TelepromptError;

pub fn run(db_path: Option<&Path>, name: &str) -> Result<(), TelepromptError> {
    let resolved_path = match db_path {
        Some(p) => p.to_path_buf(),
        None => credentials::get_default_db_path()?,
    };

    if !resolved_path.exists() {
        return Err(TelepromptError::NotInitialized);
    }

    let master_pwd = get_master_password()?;
    let mut store = credentials::load_store(&resolved_path, &master_pwd)?;

    if !store.devices.contains_key(name) {
        return Err(TelepromptError::DeviceNotFound(name.to_string()));
    }

    print!("Are you sure you want to remove device '{}'? (y/N): ", name);
    std::io::stdout().flush().map_err(TelepromptError::Io)?;
    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer).map_err(TelepromptError::Io)?;
    
    if answer.trim().eq_ignore_ascii_case("y") {
        store.devices.remove(name);
        credentials::save_store(&store, &resolved_path, &master_pwd)?;
        println!("✔ Device '{}' removed successfully.", name);
    } else {
        println!("Removal cancelled.");
    }

    Ok(())
}
