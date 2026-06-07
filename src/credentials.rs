use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::crypto;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ConnectionType {
    Ssh,
    Telnet,
}

#[derive(Serialize, Deserialize, Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct Device {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub key_path: Option<String>,
    #[zeroize(skip)]
    pub connection_type: ConnectionType,
    #[zeroize(skip)]
    pub sudo_capable: bool,
    #[zeroize(skip)]
    pub sudo_password_required: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CredentialStore {
    pub devices: HashMap<String, Device>,
}

use crate::error::TelepromptError;

/// Returns the default path to the credentials file (~/.teleprompt/credentials.enc)
pub fn get_default_db_path() -> Result<PathBuf, TelepromptError> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| TelepromptError::Other("Could not find home directory".to_string()))?;
    Ok(home_dir.join(".teleprompt").join("credentials.enc"))
}

/// Loads and decrypts the credential store from the specified path.
/// If the file does not exist, returns an empty CredentialStore.
pub fn load_store<P: AsRef<Path>>(path: P, password: &str) -> Result<CredentialStore, TelepromptError> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(CredentialStore::default());
    }

    let encrypted_data = fs::read(path)?;
    let decrypted_json = crypto::decrypt(&encrypted_data, password)
        .map_err(|_e| TelepromptError::InvalidPassword)?;
    
    let store: CredentialStore = serde_json::from_slice(&decrypted_json)
        .map_err(|e| TelepromptError::CryptoOrSerial(e.to_string()))?;
    Ok(store)
}

/// Encrypts and saves the credential store to the specified path.
pub fn save_store<P: AsRef<Path>>(
    store: &CredentialStore,
    path: P,
    password: &str,
) -> Result<(), TelepromptError> {
    let path = path.as_ref();
    
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let json_bytes = serde_json::to_vec(store)
        .map_err(|e| TelepromptError::CryptoOrSerial(e.to_string()))?;
    let encrypted_data = crypto::encrypt(&json_bytes, password)
        .map_err(|e| TelepromptError::CryptoOrSerial(e.to_string()))?;
    
    // Write atomically
    let temp_path = path.with_extension("tmp");
    let mut file = File::create(&temp_path)?;
    file.write_all(&encrypted_data)?;
    file.sync_all()?;
    
    fs::rename(temp_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_crud_operations() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join("teleprompt_test_db.enc");
        if db_path.exists() {
            let _ = fs::remove_file(&db_path);
        }

        let password = "master_pwd_123";
        
        // 1. Load non-existent store (should be empty)
        let mut store = load_store(&db_path, password).expect("Failed to load empty store");
        assert!(store.devices.is_empty());

        // 2. Add device
        let dev1 = Device {
            name: "test_srv".to_string(),
            host: "192.168.1.50".to_string(),
            port: 22,
            username: "admin".to_string(),
            password: Some("admin123".to_string()),
            key_path: None,
            connection_type: ConnectionType::Ssh,
            sudo_capable: true,
            sudo_password_required: true,
        };
        
        store.devices.insert(dev1.name.clone(), dev1.clone());
        save_store(&store, &db_path, password).expect("Failed to save store");

        // 3. Load again and verify
        let loaded_store = load_store(&db_path, password).expect("Failed to load saved store");
        assert_eq!(loaded_store.devices.len(), 1);
        let loaded_dev = loaded_store.devices.get("test_srv").unwrap();
        assert_eq!(loaded_dev.host, "192.168.1.50");
        assert_eq!(loaded_dev.password.as_deref(), Some("admin123"));

        // 4. Delete file
        let _ = fs::remove_file(&db_path);
    }
}
