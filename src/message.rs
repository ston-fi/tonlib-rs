pub use error::*;
pub use jetton::*;
pub use transfer::*;

mod error;
mod jetton;
mod transfer;

use lazy_static::lazy_static;
use num_bigint::BigUint;
use num_traits::Zero;

lazy_static! {
    pub(crate) static ref ZERO_COINS: BigUint = BigUint::zero();
}
