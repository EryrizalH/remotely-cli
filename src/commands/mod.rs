pub mod init;
pub mod add;
pub mod remove;
pub mod edit;
pub mod list;
pub mod test;
pub mod exec;

use std::io::Write;

// Helper to get master password from env var or prompt the user
pub fn get_master_password() -> Result<String, crate::error::RemotelyError> {
    if let Ok(pwd) = std::env::var("REMOTELY_KEY") {
        if !pwd.is_empty() {
            return Ok(pwd);
        }
    }

    // Prompt user
    print!("Enter Master Password: ");
    std::io::stdout().flush().map_err(|e| crate::error::RemotelyError::Io(e))?;
    
    let pwd = rpassword::read_password()
        .map_err(|e| crate::error::RemotelyError::Io(e))?;
    
    if pwd.is_empty() {
        return Err(crate::error::RemotelyError::InvalidPassword);
    }
    
    Ok(pwd)
}
