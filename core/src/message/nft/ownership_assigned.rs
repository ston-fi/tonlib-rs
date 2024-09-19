use num_bigint::BigUint;

use super::NFT_OWNERSHIP_ASSIGNED;
use crate::cell::{ArcCell, Cell, CellBuilder, EitherCellLayout, EMPTY_ARC_CELL};
use crate::message::{HasOpcode, TonMessage, TonMessageError, WithForwardPayload};
use crate::TonAddress;

/// Creates a body for nft ownership assigned  according to TL-B schema:
///
/// ```raw
/// ownership_assigned#0x05138d91
///   query_id:uint64
///   prev_owner:MsgAddress
///   forward_payload:(Either Cell ^Cell)
/// = InternalMsgBody;
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct NftOwnershipAssignedMessage {
    /// arbitrary request number.
    pub query_id: u64,
    /// address of the previous owner of the NFT item.
    pub prev_owner: TonAddress,
    ///  optional custom data that should be sent to the destination address.
    pub forward_payload: ArcCell,

    pub forward_payload_layout: EitherCellLayout,
}

impl NftOwnershipAssignedMessage {
    pub fn new(prev_owner: &TonAddress) -> Self {
        NftOwnershipAssignedMessage {
            query_id: 0,
            prev_owner: prev_owner.clone(),
            forward_payload: EMPTY_ARC_CELL.clone(),
            forward_payload_layout: EitherCellLayout::Native,
        }
    }
}

impl TonMessage for NftOwnershipAssignedMessage {
    fn build(&self) -> Result<Cell, TonMessageError> {
        let mut builder = CellBuilder::new();
        builder.store_u32(32, Self::opcode())?;
        builder.store_u64(64, self.query_id)?;
        builder.store_address(&self.prev_owner)?;
        builder
            .store_either_cell_or_cell_ref(&self.forward_payload, self.forward_payload_layout)?;
        Ok(builder.build()?)
    }

    fn parse(cell: &Cell) -> Result<Self, TonMessageError> {
        let mut parser = cell.parser();

        let opcode: u32 = parser.load_u32(32)?;
        let query_id = parser.load_u64(64)?;
        let prev_owner = parser.load_address()?;
        let forward_payload = parser.load_either_cell_or_cell_ref()?;
        parser.ensure_empty()?;

        let result = NftOwnershipAssignedMessage {
            query_id,
            prev_owner,
            forward_payload,
            forward_payload_layout: EitherCellLayout::Native,
        };
        result.verify_opcode(opcode)?;
        Ok(result)
    }
}

impl WithForwardPayload for NftOwnershipAssignedMessage {
    fn set_forward_payload(&mut self, forward_payload: ArcCell, _forward_ton_amount: BigUint) {
        self.forward_payload = forward_payload;
    }
}

impl HasOpcode for NftOwnershipAssignedMessage {
    fn set_query_id(&mut self, query_id: u64) {
        self.query_id = query_id;
    }

    fn query_id(&self) -> u64 {
        self.query_id
    }

    fn opcode() -> u32 {
        NFT_OWNERSHIP_ASSIGNED
    }
}
