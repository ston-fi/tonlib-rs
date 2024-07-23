use thiserror::Error;

#[derive(Debug, Error)]
pub enum MnemonicError {
    #[error("Invalid mnemonic word count (count: {0})")]
    UnexpectedWordCount(usize),

    #[error("Invalid mnemonic word (word: {0})")]
    InvalidWord(String),

    #[error("Invalid mnemonic with password (first byte: {0:#X})")]
    InvalidFirstByte(u8),

    #[error("Invalid passwordless mnemonic (first byte: {0:#X})")]
    InvalidPasswordlessMenmonicFirstByte(u8),

    #[error("Invalid password (hash: {0})")]
    PasswordHashError(pbkdf2::password_hash::Error),

    #[error("Invalid length of sha digest (length: {0})")]
    ShaDigestLengthInvalid(#[from] sha2::digest::InvalidLength),
}
