use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::Argon2;
use rand::{RngCore, thread_rng};
use zeroize::Zeroize;

const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;

/// Derives a 256-bit key from a password and a salt using Argon2id.
pub fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; 32], anyhow::Error> {
    let mut key = [0u8; 32];
    let mut pwd_bytes = password.as_bytes().to_vec();
    
    let argon2 = Argon2::default();
    argon2.hash_password_into(&pwd_bytes, salt, &mut key)
        .map_err(|e| anyhow::anyhow!("Argon2 derivation failed: {}", e))?;
        
    pwd_bytes.zeroize();
    Ok(key)
}

/// Encrypts plaintext using a master password.
/// Returns a byte vector: SALT (16B) || NONCE (12B) || CIPHERTEXT
pub fn encrypt(plaintext: &[u8], password: &str) -> Result<Vec<u8>, anyhow::Error> {
    let mut salt = [0u8; SALT_LEN];
    let mut nonce_bytes = [0u8; NONCE_LEN];
    
    // Generate secure random salt and nonce
    let mut rng = thread_rng();
    rng.fill_bytes(&mut salt);
    rng.fill_bytes(&mut nonce_bytes);
    
    let mut key = derive_key(password, &salt)?;
    
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| anyhow::anyhow!("Cipher creation failed: {}", e))?;
    key.zeroize();
    
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher.encrypt(nonce, plaintext)
        .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;
        
    let mut result = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
    result.extend_from_slice(&salt);
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);
    
    Ok(result)
}

/// Decrypts a payload (SALT || NONCE || CIPHERTEXT) using the master password.
pub fn decrypt(payload: &[u8], password: &str) -> Result<Vec<u8>, anyhow::Error> {
    if payload.len() < SALT_LEN + NONCE_LEN {
        return Err(anyhow::anyhow!("Invalid payload length: too short"));
    }
    
    let (salt, rest) = payload.split_at(SALT_LEN);
    let (nonce_bytes, ciphertext) = rest.split_at(NONCE_LEN);
    
    let mut key = derive_key(password, salt)?;
    
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| anyhow::anyhow!("Cipher creation failed: {}", e))?;
    key.zeroize();
    
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher.decrypt(nonce, ciphertext)
        .map_err(|_e| anyhow::anyhow!("Decryption failed: check your master password"))?;
        
    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let password = "my_super_secret_master_password";
        let plaintext = b"Hello, secure remote world!";
        
        let encrypted = encrypt(plaintext, password).expect("Encryption failed");
        assert_ne!(encrypted, plaintext); // Ciphertext shouldn't match plaintext
        
        let decrypted = decrypt(&encrypted, password).expect("Decryption failed");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_decrypt_invalid_password() {
        let password = "correct_password";
        let wrong_password = "wrong_password";
        let plaintext = b"sensitive data";
        
        let encrypted = encrypt(plaintext, password).expect("Encryption failed");
        let result = decrypt(&encrypted, wrong_password);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_invalid_payload_length() {
        let result = decrypt(&[0u8; 10], "password");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Invalid payload length: too short");
    }

    #[test]
    fn test_salt_nonce_uniqueness() {
        let password = "some_password";
        let plaintext = b"identical data";
        
        let enc1 = encrypt(plaintext, password).expect("Encryption 1 failed");
        let enc2 = encrypt(plaintext, password).expect("Encryption 2 failed");
        
        // They must not be identical since salt and nonce are random
        assert_ne!(enc1, enc2);
        
        // Both must decrypt correctly to the same plaintext
        assert_eq!(decrypt(&enc1, password).unwrap(), plaintext);
        assert_eq!(decrypt(&enc2, password).unwrap(), plaintext);
    }

    #[test]
    fn test_encrypt_decrypt_empty_password() {
        let password = "";
        let plaintext = b"some data";
        
        let encrypted = encrypt(plaintext, password).expect("Encryption failed");
        let decrypted = decrypt(&encrypted, password).expect("Decryption failed");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_decrypt_special_chars_password() {
        let password = "🔒🔑 master 🌐 password 123!@#";
        let plaintext = b"some sensitive information";
        
        let encrypted = encrypt(plaintext, password).expect("Encryption failed");
        let decrypted = decrypt(&encrypted, password).expect("Decryption failed");
        assert_eq!(decrypted, plaintext);
    }
}
