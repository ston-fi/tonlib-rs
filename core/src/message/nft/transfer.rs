use num_bigint::BigUint;
use num_traits::Zero;

use super::NFT_TRANSFER;
use crate::cell::{ArcCell, Cell, CellBuilder, EitherCellLayout, EMPTY_ARC_CELL};
use crate::message::{HasOpcode, TonMessage, TonMessageError, WithForwardPayload, ZERO_COINS};
use crate::TonAddress;

/// Creates a body for jetton transfer according to TL-B schema:
///
/// ```raw
/// transfer#5fcc3d14
///   query_id:uint64
///   new_owner:MsgAddress
///   response_destination:MsgAddress
///   custom_payload:(Maybe ^Cell)
///   forward_amount:(VarUInteger 16)
///   forward_payload:(Either Cell ^Cell)
/// = InternalMsgBody;
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct NftTransferMessage {
    /// arbitrary request number.
    pub query_id: u64,
    /// address of the new owner of the NFT item.
    pub new_owner: TonAddress,
    ///  address where to send a response with confirmation of a successful transfer and the rest of the incoming message coins.
    pub response_destination: TonAddress,
    /// optional custom data.
    pub custom_payload: Option<ArcCell>,
    ///  the amount of nanotons to be sent to the destination address.
    pub forward_ton_amount: BigUint,
    ///  optional custom data that should be sent to the destination address.
    pub forward_payload: ArcCell,

    pub forward_payload_layout: EitherCellLayout,
}

impl NftTransferMessage {
    pub fn new(new_owner: &TonAddress) -> Self {
        NftTransferMessage {
            query_id: 0,
            new_owner: new_owner.clone(),
            response_destination: TonAddress::null(),
            custom_payload: None,
            forward_ton_amount: ZERO_COINS.clone(),
            forward_payload: EMPTY_ARC_CELL.clone(),
            forward_payload_layout: EitherCellLayout::Native,
        }
    }

    pub fn with_response_destination(&mut self, response_destination: &TonAddress) -> &mut Self {
        self.response_destination = response_destination.clone();
        self
    }

    pub fn with_custom_payload(&mut self, custom_payload: ArcCell) -> &mut Self {
        self.custom_payload = Some(custom_payload);
        self
    }
}

impl TonMessage for NftTransferMessage {
    fn build(&self) -> Result<Cell, TonMessageError> {
        if self.forward_ton_amount.is_zero() && self.forward_payload == EMPTY_ARC_CELL.clone() {
            return Err(TonMessageError::ForwardTonAmountIsNegative);
        }

        let mut builder = CellBuilder::new();
        builder.store_u32(32, Self::opcode())?;
        builder.store_u64(64, self.query_id)?;

        builder.store_address(&self.new_owner)?;
        builder.store_address(&self.response_destination)?;
        builder.store_maybe_cell_ref(&self.custom_payload)?;
        builder.store_coins(&self.forward_ton_amount)?;
        builder
            .store_either_cell_or_cell_ref(&self.forward_payload, self.forward_payload_layout)?;
        Ok(builder.build()?)
    }

    fn parse(cell: &Cell) -> Result<Self, TonMessageError> {
        let mut parser = cell.parser();

        let opcode: u32 = parser.load_u32(32)?;
        let query_id = parser.load_u64(64)?;

        let new_owner = parser.load_address()?;
        let response_destination = parser.load_address()?;
        let custom_payload = parser.load_maybe_cell_ref()?;
        let forward_ton_amount = parser.load_coins()?;
        let forward_payload = parser.load_either_cell_or_cell_ref()?;
        parser.ensure_empty()?;

        let result = NftTransferMessage {
            query_id,
            new_owner,
            response_destination,
            custom_payload,
            forward_ton_amount,
            forward_payload,
            forward_payload_layout: EitherCellLayout::Native,
        };
        result.verify_opcode(opcode)?;

        Ok(result)
    }
}

impl WithForwardPayload for NftTransferMessage {
    fn set_forward_payload(&mut self, forward_payload: ArcCell, forward_ton_amount: BigUint) {
        self.forward_payload = forward_payload;
        self.forward_ton_amount = forward_ton_amount;
    }
}

impl HasOpcode for NftTransferMessage {
    fn set_query_id(&mut self, query_id: u64) {
        self.query_id = query_id;
    }

    fn query_id(&self) -> u64 {
        self.query_id
    }

    fn opcode() -> u32 {
        NFT_TRANSFER
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use lazy_static::lazy_static;
    use num_bigint::BigUint;

    use crate::cell::{ArcCell, BagOfCells, Cell, EitherCellLayout};
    use crate::message::{
        HasOpcode, NftTransferMessage, TonMessage, TonMessageError, WithForwardPayload,
    };
    use crate::TonAddress;

    const NFT_TRANSFER_MSG: &str="b5ee9c7201010101006f0000d95fcc3d140000000000000000800e20aaf07ad251d1800fe45e3af334769b7b2069d3ab2ea6c9ee0f73dfd072a21000a1b4b24b6a66313f3e0b49d095f3e8f4294af504b3a0f7b99290129f3aaafcc47312d0040544f4e506c616e65747320676966742077697468206c6f76658";
    const NFT_TRANSFER_PAYLOAD_DATA: &str = "40544F4E506C616E65747320676966742077697468206C6F7665";
    const NFT_TRANSFER_PAYLOAD_BIT_LEN: usize = 208;

    lazy_static! {
        static ref NFT_TRANSFER_PAYLOAD: ArcCell = Arc::new(
            Cell::new(
                hex::decode(NFT_TRANSFER_PAYLOAD_DATA).unwrap(),
                NFT_TRANSFER_PAYLOAD_BIT_LEN,
                vec![],
                false,
            )
            .unwrap()
        );
    }
    #[test]
    fn test_ft_transfer_parser() -> Result<(), TonMessageError> {
        let boc = BagOfCells::parse_hex(NFT_TRANSFER_MSG).unwrap();
        let cell = boc.single_root().unwrap();

        let result_nft_transfer_msg = NftTransferMessage::parse(cell)?;

        let forward_ton_amount = BigUint::from(10000000u64);
        let expected_nft_transfer_msg = NftTransferMessage {
            query_id: 0,
            new_owner: TonAddress::from_hex_str(
                "0:71055783d6928e8c007f22f1d799a3b4dbd9034e9d5975364f707b9efe839510",
            )
            .unwrap(),
            response_destination: TonAddress::from_hex_str(
                "0:286d2c92da998c4fcf82d274257cfa3d0a52bd412ce83dee64a404a7ceaabf31",
            )
            .unwrap(),
            custom_payload: None,
            forward_ton_amount,
            forward_payload: NFT_TRANSFER_PAYLOAD.clone(),
            forward_payload_layout: EitherCellLayout::Native,
        };

        assert_eq!(expected_nft_transfer_msg, result_nft_transfer_msg);
        Ok(())
    }

    #[test]
    fn test_nft_transfer_builder() -> Result<(), TonMessageError> {
        let jetton_transfer_msg = NftTransferMessage::new(
            &TonAddress::from_hex_str(
                "0:71055783d6928e8c007f22f1d799a3b4dbd9034e9d5975364f707b9efe839510",
            )
            .unwrap(),
        )
        .with_query_id(0)
        .with_response_destination(
            &TonAddress::from_hex_str(
                "0:286d2c92da998c4fcf82d274257cfa3d0a52bd412ce83dee64a404a7ceaabf31",
            )
            .unwrap(),
        )
        .with_forward_payload(BigUint::from(10000000u64), NFT_TRANSFER_PAYLOAD.clone())
        .build();

        let result_boc_serialized = BagOfCells::from_root(jetton_transfer_msg.unwrap())
            .serialize(false)
            .unwrap();
        let expected_boc_serialized = hex::decode(NFT_TRANSFER_MSG).unwrap();

        assert_eq!(expected_boc_serialized, result_boc_serialized);
        Ok(())
    }
}
