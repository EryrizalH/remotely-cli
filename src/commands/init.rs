use std::io::Write;
use std::path::Path;

use crate::credentials::{self, CredentialStore};
use crate::error::TelepromptError;

pub fn run(db_path: Option<&Path>) -> Result<(), TelepromptError> {
    let resolved_path = match db_path {
        Some(p) => p.to_path_buf(),
        None => credentials::get_default_db_path()?,
    };

    if resolved_path.exists() {
        print!("Credential store already exists at {}. Overwrite? (y/N): ", resolved_path.display());
        std::io::stdout().flush().map_err(TelepromptError::Io)?;
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer).map_err(TelepromptError::Io)?;
        if !answer.trim().eq_ignore_ascii_case("y") {
            println!("Abort initialization.");
            return Ok(());
        }
    }

    // Prompt for password
    print!("Set Master Password (used to encrypt credentials): ");
    std::io::stdout().flush().map_err(TelepromptError::Io)?;
    let password = rpassword::read_password().map_err(TelepromptError::Io)?;
    
    if password.trim().is_empty() {
        return Err(TelepromptError::Cli("Master password cannot be empty".to_string()));
    }

    print!("Confirm Master Password: ");
    std::io::stdout().flush().map_err(TelepromptError::Io)?;
    let confirm = rpassword::read_password().map_err(TelepromptError::Io)?;

    if password != confirm {
        return Err(TelepromptError::Cli("Passwords do not match".to_string()));
    }

    let store = CredentialStore::default();
    credentials::save_store(&store, &resolved_path, &password)?;

    // Cache the master password securely in ~/.teleprompt/master.key
    super::save_master_password(&password)?;

    println!("\nSuccessfully initialized empty credential store at: {}", resolved_path.display());
    println!("✔ Master password cached locally in ~/.teleprompt/master.key (owner-only permissions).");
    println!("✔ Future teleprompt commands will run automatically without asking for a password.");

    Ok(())
}
