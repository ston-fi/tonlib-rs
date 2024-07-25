mod address;
mod error;
mod tx_id;

pub use address::*;
pub use error::*;
pub use tx_id::*;

pub const TON_HASH_BYTES: usize = 32;
pub const ZERO_HASH: TonHash = [0; 32];
pub type TonHash = [u8; TON_HASH_BYTES];
