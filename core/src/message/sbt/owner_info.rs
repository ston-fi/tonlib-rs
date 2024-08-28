use num_bigint::BigUint;

use super::SBT_OWNER_INFO;
use crate::cell::{ArcCell, Cell, CellBuilder};
use crate::message::{HasOpcode, TonMessage, TonMessageError};
use crate::TonAddress;

/// Creates a body for sbt ownership proof according to TL-B schema:
///
/// ```raw
/// owner_info#0dd607e3
///   query_id:uint64
///   item_id:uint256
///   initiator:MsgAddress
///   owner:MsgAddress
///   data:^Cell
///   revoked_at:uint64
///   content:(Maybe ^Cell)
/// = InternalMsgBody;
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct OwnerInfoMessage {
    /// arbitrary request number.
    pub query_id: u64,
    /// id of NFT.
    pub item_id: BigUint,
    /// address of request initiator.
    pub initiator: TonAddress,
    /// current owner's address.
    pub owner: TonAddress,
    /// data cell passed in prove_ownership.
    pub data: ArcCell,
    /// unix time when SBT was revoked, 0 if it was not.
    pub revoked_at: u64,
    /// NFT's content, it is passed if with_content was true in prove_ownership.
    pub content: Option<ArcCell>,
}

impl OwnerInfoMessage {
    pub fn new(
        item_id: BigUint,
        owner: &TonAddress,
        initiator: &TonAddress,
        data: ArcCell,
        revoked_at: u64,
        content: Option<ArcCell>,
    ) -> Self {
        OwnerInfoMessage {
            query_id: 0,
            item_id,
            initiator: initiator.clone(),
            owner: owner.clone(),
            data,
            revoked_at,
            content,
        }
    }
}

impl TonMessage for OwnerInfoMessage {
    fn build(&self) -> Result<Cell, TonMessageError> {
        let mut builder = CellBuilder::new();
        builder.store_u32(32, Self::opcode())?;
        builder.store_u64(64, self.query_id)?;
        builder.store_uint(256, &self.item_id)?;
        builder.store_address(&self.initiator)?;
        builder.store_address(&self.owner)?;
        builder.store_reference(&self.data)?;
        builder.store_u64(64, self.revoked_at)?;
        builder.store_maybe_cell_ref(&self.content)?;
        Ok(builder.build()?)
    }

    fn parse(cell: &Cell) -> Result<Self, TonMessageError> {
        let mut parser = cell.parser();

        let opcode: u32 = parser.load_u32(32)?;
        let query_id = parser.load_u64(64)?;
        let item_id = parser.load_uint(256)?;
        let initiator = parser.load_address()?;
        let owner = parser.load_address()?;
        let data = parser.next_reference()?;
        let revoked_at = parser.load_u64(64)?;
        let content = parser.load_maybe_cell_ref()?;
        parser.ensure_empty()?;

        let result = OwnerInfoMessage {
            query_id,
            item_id,
            initiator,
            owner,
            data,
            revoked_at,
            content,
        };
        result.verify_opcode(opcode)?;
        Ok(result)
    }
}

impl HasOpcode for OwnerInfoMessage {
    fn set_query_id(&mut self, query_id: u64) {
        self.query_id = query_id;
    }

    fn query_id(&self) -> u64 {
        self.query_id
    }

    fn opcode() -> u32 {
        SBT_OWNER_INFO
    }
}

#[cfg(test)]
mod tests {

    use std::sync::Arc;

    use num_bigint::BigUint;

    use super::OwnerInfoMessage;
    use crate::cell::{ArcCell, CellBuilder};
    use crate::message::{HasOpcode, TonMessage};
    use crate::TonAddress;
    #[test]
    fn sbt_owner_info_msg_test() {
        let query_id = 1234567890;
        let item_id = BigUint::from(123u64);

        let initiator =
            &TonAddress::from_base64_url("EQAW42HutyDem98Be1f27PoXobghh81umTQ-cGgaKVmRLS7-")
                .unwrap();
        let owner =
            &TonAddress::from_base64_url("EQAd8QRKoA5sKcug9bwK6vMdmhSAoAxr8vvABvC1TCeTude5")
                .unwrap();

        let data: ArcCell = Arc::new(
            CellBuilder::new()
                .store_u32(12, 123)
                .unwrap()
                .build()
                .unwrap(),
        );
        let revoked_at = 123456;
        let content = Some(Arc::new(
            CellBuilder::new()
                .store_u32(12, 456)
                .unwrap()
                .build()
                .unwrap(),
        ));

        let expected = OwnerInfoMessage {
            query_id,
            item_id: item_id.clone(),
            initiator: initiator.clone(),
            owner: owner.clone(),
            data: data.clone(),
            revoked_at,
            content: content.clone(),
        };

        let build_result =
            OwnerInfoMessage::new(item_id, owner, initiator, data, revoked_at, content)
                .with_query_id(query_id)
                .build();
        assert!(build_result.is_ok());

        let cell = build_result.unwrap();
        let parse_result = OwnerInfoMessage::parse(&cell);
        assert!(parse_result.is_ok());

        let parsed_msg = parse_result.unwrap();
        assert_eq!(expected, parsed_msg);
    }
}
