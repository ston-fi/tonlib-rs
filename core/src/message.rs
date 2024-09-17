mod error;
pub use error::*;
use lazy_static::lazy_static;
use num_bigint::BigUint;
use num_traits::Zero;

mod common_msg_info;
pub use common_msg_info::*;

use crate::cell::{ArcCell, Cell};

mod common;
mod jetton;
mod nft;
mod sbt;
mod transfer;
pub use common::*;
pub use jetton::*;
pub use nft::*;
pub use sbt::*;
pub use transfer::*;

lazy_static! {
    pub(crate) static ref ZERO_COINS: BigUint = BigUint::zero();
}

pub trait TonMessage: Sized {
    fn build(&self) -> Result<Cell, TonMessageError>;

    fn parse(cell: &Cell) -> Result<Self, TonMessageError>;
}

pub trait HasOpcode: TonMessage {
    fn verify_opcode(&self, opcode: u32) -> Result<(), TonMessageError> {
        let expected_opcode = Self::opcode();
        if opcode != expected_opcode {
            let invalid = InvalidMessage {
                opcode: Some(opcode),
                query_id: Some(self.query_id()),
                message: format!("Unexpected opcode.  {0:08x} expected", expected_opcode),
            };
            Err(TonMessageError::InvalidMessage(invalid))
        } else {
            Ok(())
        }
    }

    fn opcode() -> u32;
    fn with_query_id(&mut self, query_id: u64) -> &mut Self {
        self.set_query_id(query_id);
        self
    }
    fn set_query_id(&mut self, query_id: u64);

    fn query_id(&self) -> u64;
}

impl TonMessage for Cell {
    fn build(&self) -> Result<Cell, TonMessageError> {
        Ok(self.clone())
    }

    fn parse(cell: &Cell) -> Result<Self, TonMessageError> {
        Ok(cell.clone())
    }
}

pub trait WithForwardPayload: TonMessage {
    fn with_forward_payload(
        &mut self,
        forward_ton_amount: BigUint,
        forward_payload: ArcCell,
    ) -> &mut Self {
        self.set_forward_payload(forward_payload, forward_ton_amount);
        self
    }

    fn set_forward_payload(&mut self, forward_payload: ArcCell, forward_ton_amount: BigUint);
}
