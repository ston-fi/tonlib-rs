mod address;
mod error;
mod tx_id;

pub use address::*;
pub use error::*;
pub use tx_id::*;

pub const TON_HASH_BYTES: usize = 32;
pub const ZERO_HASH: TonHash = [0; 32];
pub type TonHash = [u8; TON_HASH_BYTES];

pub const DEFAULT_CELL_HASH: TonHash = [
    150, 162, 150, 210, 36, 242, 133, 198, 123, 238, 147, 195, 15, 138, 48, 145, 87, 240, 218, 163,
    93, 197, 184, 126, 65, 11, 120, 99, 10, 9, 207, 199,
];
