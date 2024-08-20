use num_bigint::BigUint;

use super::SBT_PROVE_OWNERSHIP;
use crate::cell::{ArcCell, Cell, CellBuilder, EMPTY_ARC_CELL};
use crate::message::{HasOpcode, TonMessage, TonMessageError, WithForwardPayload};
use crate::TonAddress;

/// Creates a body for sbt prove according to TL-B schema:
///
/// ```raw
/// prove_ownership#04ded148
///   query_id:uint64
///   dest:MsgAddress
///   forward_payload:^Cell
///   with_content:Bool
/// = InternalMsgBody;
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct ProveOwnershipMessage {
    /// arbitrary request number.
    pub query_id: u64,
    ///  address of the contract to which the ownership of SBT should be proven.
    pub dest: TonAddress,
    /// arbitrary data required by target contract.
    pub forward_payload: ArcCell,
    /// if true, SBT's content cell will be included in message to contract.
    pub with_content: bool,
}

impl ProveOwnershipMessage {
    pub fn new(dest: &TonAddress, with_content: bool) -> Self {
        ProveOwnershipMessage {
            query_id: 0,
            dest: dest.clone(),
            forward_payload: EMPTY_ARC_CELL.clone(),
            with_content,
        }
    }
}

impl TonMessage for ProveOwnershipMessage {
    fn build(&self) -> Result<Cell, TonMessageError> {
        let mut builder = CellBuilder::new();
        builder.store_u32(32, Self::opcode())?;
        builder.store_u64(64, self.query_id)?;
        builder.store_address(&self.dest)?;
        builder.store_reference(&self.forward_payload)?;
        builder.store_bit(self.with_content)?;
        Ok(builder.build()?)
    }

    fn parse(cell: &Cell) -> Result<Self, TonMessageError> {
        let mut parser = cell.parser();

        let opcode: u32 = parser.load_u32(32)?;
        let query_id = parser.load_u64(64)?;
        let dest = parser.load_address()?;
        let forward_payload = parser.next_reference()?;
        let with_content = parser.load_bit()?;
        parser.ensure_empty()?;

        let result = ProveOwnershipMessage {
            query_id,
            dest,
            forward_payload,
            with_content,
        };
        result.verify_opcode(opcode)?;
        Ok(result)
    }
}

impl WithForwardPayload for ProveOwnershipMessage {
    fn set_forward_payload(&mut self, forward_payload: ArcCell, _forward_ton_amount: BigUint) {
        self.forward_payload = forward_payload;
    }
}

impl HasOpcode for ProveOwnershipMessage {
    fn set_query_id(&mut self, query_id: u64) {
        self.query_id = query_id;
    }

    fn query_id(&self) -> u64 {
        self.query_id
    }

    fn opcode() -> u32 {
        SBT_PROVE_OWNERSHIP
    }
}

#[cfg(test)]
mod tests {

    use std::sync::Arc;

    use num_bigint::BigUint;
    use num_traits::Zero;

    use crate::cell::{ArcCell, CellBuilder};
    use crate::message::{HasOpcode, ProveOwnershipMessage, TonMessage, WithForwardPayload};
    use crate::TonAddress;

    #[test]
    fn sbt_prove_ownership_msg_test() {
        let query_id = 1234567890;

        let dest = &TonAddress::from_base64_url("EQAW42HutyDem98Be1f27PoXobghh81umTQ-cGgaKVmRLS7-")
            .unwrap();

        let forward_payload: ArcCell = Arc::new(
            CellBuilder::new()
                .store_u32(12, 123)
                .unwrap()
                .build()
                .unwrap()
                .into(),
        );

        let expected = ProveOwnershipMessage {
            query_id,
            dest: dest.clone(),
            forward_payload: forward_payload.clone(),
            with_content: true,
        };

        let build_result = ProveOwnershipMessage::new(dest, true)
            .with_forward_payload(BigUint::zero(), forward_payload)
            .with_query_id(query_id)
            .build();
        assert!(build_result.is_ok());

        let cell = build_result.unwrap();
        let parse_result = ProveOwnershipMessage::parse(&cell);
        assert!(parse_result.is_ok());

        let parsed_msg = parse_result.unwrap();
        assert_eq!(expected, parsed_msg);
    }
}
