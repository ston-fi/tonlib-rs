use thiserror::Error;

#[derive(Debug, Error)]
pub enum MnemonicError {
    #[error("Invalid mnemonic word count, got: {count}")]
    UnexpectedWordCount { count: usize },

    #[error("Invalid mnemonic word: '{word}'")]
    InvalidWord { word: String },

    #[error("Invalid mnemonic with password, expected first byte equal to {byte}")]
    InvalidFirstByte { byte: u8 },

    #[error("Invalid passwordless mnemonic, expected first byte equal to {byte}")]
    InvalidPasswordlessMenmonicFirstByte { byte: u8 },

    #[error("Invalid password {e}")]
    PasswordHashError { e: pbkdf2::password_hash::Error },

    #[error("Invalid length of sha digest: {0}")]
    ShaDigestLengthInvalid(#[from] sha2::digest::InvalidLength),
}
