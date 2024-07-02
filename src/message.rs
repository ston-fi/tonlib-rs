pub use error::*;
pub use jetton::*;
pub use transfer::*;
pub use util::*;

mod error;
mod jetton;
mod transfer;
mod util;

use lazy_static::lazy_static;
use num_bigint::BigUint;
use num_traits::Zero;

lazy_static! {
    pub(crate) static ref ZERO_COINS: BigUint = BigUint::zero();
}
