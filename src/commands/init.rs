use std::io::Write;
use std::path::Path;

use crate::credentials::{self, CredentialStore};
use crate::error::RemotelyError;

pub fn run(db_path: Option<&Path>) -> Result<(), RemotelyError> {
    let resolved_path = match db_path {
        Some(p) => p.to_path_buf(),
        None => credentials::get_default_db_path()
            .map_err(|e| RemotelyError::Other(e.to_string()))?,
    };

    if resolved_path.exists() {
        print!("Credential store already exists at {}. Overwrite? (y/N): ", resolved_path.display());
        std::io::stdout().flush().map_err(RemotelyError::Io)?;
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer).map_err(RemotelyError::Io)?;
        if !answer.trim().eq_ignore_ascii_case("y") {
            println!("Abort initialization.");
            return Ok(());
        }
    }

    // Prompt for password
    print!("Set Master Password (used to encrypt credentials): ");
    std::io::stdout().flush().map_err(RemotelyError::Io)?;
    let password = rpassword::read_password().map_err(RemotelyError::Io)?;
    
    if password.trim().is_empty() {
        return Err(RemotelyError::Cli("Master password cannot be empty".to_string()));
    }

    print!("Confirm Master Password: ");
    std::io::stdout().flush().map_err(RemotelyError::Io)?;
    let confirm = rpassword::read_password().map_err(RemotelyError::Io)?;

    if password != confirm {
        return Err(RemotelyError::Cli("Passwords do not match".to_string()));
    }

    let store = CredentialStore::default();
    credentials::save_store(&store, &resolved_path, &password)
        .map_err(|e| RemotelyError::CryptoOrSerial(e.to_string()))?;

    println!("\nSuccessfully initialized empty credential store at: {}", resolved_path.display());
    println!("IMPORTANT: Set the REMOTELY_KEY environment variable to bypass future password prompts.");
    println!("Example: export REMOTELY_KEY=\"your-password\"");

    Ok(())
}
