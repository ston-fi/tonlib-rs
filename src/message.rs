use lazy_static::lazy_static;
use num_bigint::BigUint;
use num_traits::Zero;

pub use error::*;
pub use jetton_transfer::*;
pub use transfer::*;

mod error;
mod jetton_transfer;
mod transfer;

lazy_static! {
    pub(crate) static ref ZERO_COINS: BigUint = BigUint::zero();
}
