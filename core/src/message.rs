pub use error::*;
pub use jetton::*;
use lazy_static::lazy_static;
use num_bigint::BigUint;
use num_traits::Zero;
pub use transfer::*;

mod common_msg_info;
pub use common_msg_info::*;

use crate::cell::Cell;

mod error;
mod jetton;
mod transfer;

lazy_static! {
    pub(crate) static ref ZERO_COINS: BigUint = BigUint::zero();
}

pub trait TonMessage: Sized {
    fn build(&self) -> Result<Cell, TonMessageError>;

    fn parse(cell: &Cell) -> Result<Self, TonMessageError>;
}

impl TonMessage for Cell {
    fn build(&self) -> Result<Cell, TonMessageError> {
        Ok(self.clone())
    }

    fn parse(cell: &Cell) -> Result<Self, TonMessageError> {
        Ok(cell.clone())
    }
}
