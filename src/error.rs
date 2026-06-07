use thiserror::Error;

#[derive(Error, Debug)]
pub enum TelepromptError {
    #[error("Device '{0}' not found in credential store")]
    DeviceNotFound(String),

    #[error("Connection failed to '{0}': {1}")]
    ConnectionFailed(String, String),

    #[error("Authentication failed for user '{0}' on '{1}'")]
    AuthFailed(String, String),

    #[error("Credential store not initialized. Run 'teleprompt init' first.")]
    NotInitialized,

    #[error("Invalid master password or corrupted credential store")]
    InvalidPassword,

    #[error("Command timed out after {0} seconds")]
    Timeout(u64),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Cryptographic/Serialization error: {0}")]
    CryptoOrSerial(String),

    #[error("Sudo authentication failed: {0}")]
    SudoFailed(String),

    #[error("Telnet error: {0}")]
    Telnet(String),

    #[error("CLI Error: {0}")]
    Cli(String),

    #[error("{0}")]
    Other(String),
}

impl TelepromptError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::DeviceNotFound(_) => 1,
            Self::ConnectionFailed(_, _) => 2,
            Self::AuthFailed(_, _) => 2,
            Self::NotInitialized => 1,
            Self::InvalidPassword => 1,
            Self::Timeout(_) => 4,
            Self::Io(_) => 1,
            Self::CryptoOrSerial(_) => 1,
            Self::SudoFailed(_) => 3,
            Self::Telnet(_) => 2,
            Self::Cli(_) => 1,
            Self::Other(_) => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Error as IoError;

    #[test]
    fn test_error_exit_codes() {
        assert_eq!(TelepromptError::DeviceNotFound("test".to_string()).exit_code(), 1);
        assert_eq!(TelepromptError::ConnectionFailed("host".to_string(), "err".to_string()).exit_code(), 2);
        assert_eq!(TelepromptError::AuthFailed("user".to_string(), "host".to_string()).exit_code(), 2);
        assert_eq!(TelepromptError::NotInitialized.exit_code(), 1);
        assert_eq!(TelepromptError::InvalidPassword.exit_code(), 1);
        assert_eq!(TelepromptError::Timeout(30).exit_code(), 4);
        assert_eq!(TelepromptError::Io(IoError::other("oh no")).exit_code(), 1);
        assert_eq!(TelepromptError::CryptoOrSerial("err".to_string()).exit_code(), 1);
        assert_eq!(TelepromptError::SudoFailed("err".to_string()).exit_code(), 3);
        assert_eq!(TelepromptError::Telnet("err".to_string()).exit_code(), 2);
        assert_eq!(TelepromptError::Cli("err".to_string()).exit_code(), 1);
        assert_eq!(TelepromptError::Other("err".to_string()).exit_code(), 1);
    }

    #[test]
    fn test_error_display() {
        assert_eq!(
            TelepromptError::DeviceNotFound("test_dev".to_string()).to_string(),
            "Device 'test_dev' not found in credential store"
        );
        assert_eq!(
            TelepromptError::ConnectionFailed("192.168.1.1:22".to_string(), "Connection refused".to_string()).to_string(),
            "Connection failed to '192.168.1.1:22': Connection refused"
        );
        assert_eq!(
            TelepromptError::AuthFailed("admin".to_string(), "192.168.1.1:22".to_string()).to_string(),
            "Authentication failed for user 'admin' on '192.168.1.1:22'"
        );
        assert_eq!(
            TelepromptError::NotInitialized.to_string(),
            "Credential store not initialized. Run 'teleprompt init' first."
        );
        assert_eq!(
            TelepromptError::InvalidPassword.to_string(),
            "Invalid master password or corrupted credential store"
        );
        assert_eq!(
            TelepromptError::Timeout(10).to_string(),
            "Command timed out after 10 seconds"
        );
        assert_eq!(
            TelepromptError::SudoFailed("wrong password".to_string()).to_string(),
            "Sudo authentication failed: wrong password"
        );
        assert_eq!(
            TelepromptError::Telnet("negotiation failed".to_string()).to_string(),
            "Telnet error: negotiation failed"
        );
        assert_eq!(
            TelepromptError::Cli("missing arg".to_string()).to_string(),
            "CLI Error: missing arg"
        );
        assert_eq!(
            TelepromptError::Other("random".to_string()).to_string(),
            "random"
        );
    }
}

