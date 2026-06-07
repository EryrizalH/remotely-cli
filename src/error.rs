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
