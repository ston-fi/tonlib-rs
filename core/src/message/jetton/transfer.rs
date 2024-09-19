use num_bigint::BigUint;
use num_traits::Zero;

use super::JETTON_TRANSFER;
use crate::cell::{ArcCell, Cell, CellBuilder, EitherCellLayout, EMPTY_ARC_CELL};
use crate::message::{HasOpcode, TonMessage, TonMessageError, WithForwardPayload, ZERO_COINS};
use crate::TonAddress;

/// Creates a body for jetton transfer according to TL-B schema:
///
/// ```raw
/// transfer#0f8a7ea5 query_id:uint64 amount:(VarUInteger 16) destination:MsgAddress
///                  response_destination:MsgAddress custom_payload:(Maybe ^Cell)
///                  forward_ton_amount:(VarUInteger 16) forward_payload:(Either Cell ^Cell)
///                  = InternalMsgBody;
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct JettonTransferMessage {
    /// arbitrary request number.
    pub query_id: u64,
    /// amount of transferred jettons in elementary units.
    pub amount: BigUint,
    /// address of the new owner of the jettons.
    pub destination: TonAddress,
    /// address where to send a response with confirmation of a successful transfer and the rest of the incoming message Toncoins.
    pub response_destination: TonAddress,
    /// optional custom data (which is used by either sender or receiver jetton wallet for inner logic).
    pub custom_payload: Option<ArcCell>,
    ///  the amount of nanotons to be sent to the destination address.
    pub forward_ton_amount: BigUint,
    ///  optional custom data that should be sent to the destination address.
    pub forward_payload: ArcCell,

    pub forward_payload_layout: EitherCellLayout,
}

impl JettonTransferMessage {
    pub fn new(destination: &TonAddress, amount: &BigUint) -> Self {
        JettonTransferMessage {
            query_id: 0,
            amount: amount.clone(),
            destination: destination.clone(),
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

    pub fn set_either_cell_layout(&mut self, layout: EitherCellLayout) -> &mut Self {
        self.forward_payload_layout = layout;
        self
    }
}

impl WithForwardPayload for JettonTransferMessage {
    fn set_forward_payload(&mut self, forward_payload: ArcCell, forward_ton_amount: BigUint) {
        self.forward_payload = forward_payload;
        self.forward_ton_amount = forward_ton_amount;
    }
}

impl TonMessage for JettonTransferMessage {
    fn build(&self) -> Result<Cell, TonMessageError> {
        if self.forward_ton_amount.is_zero() && self.forward_payload != *EMPTY_ARC_CELL {
            return Err(TonMessageError::ForwardTonAmountIsNegative);
        }

        let mut builder = CellBuilder::new();
        builder.store_u32(32, Self::opcode())?;
        builder.store_u64(64, self.query_id)?;
        builder.store_coins(&self.amount)?;
        builder.store_address(&self.destination)?;
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

        let amount = parser.load_coins()?;
        let destination = parser.load_address()?;
        let response_destination = parser.load_address()?;
        let custom_payload = parser.load_maybe_cell_ref()?;
        let forward_ton_amount = parser.load_coins()?;
        let forward_payload = parser.load_either_cell_or_cell_ref()?;
        parser.ensure_empty()?;

        let result = JettonTransferMessage {
            query_id,
            amount,
            destination,
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

impl HasOpcode for JettonTransferMessage {
    fn set_query_id(&mut self, query_id: u64) {
        self.query_id = query_id;
    }

    fn query_id(&self) -> u64 {
        self.query_id
    }

    fn opcode() -> u32 {
        JETTON_TRANSFER
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::sync::Arc;

    use num_bigint::BigUint;
    use num_traits::Zero;

    use crate::cell::{BagOfCells, Cell, CellBuilder, EitherCellLayout, EMPTY_ARC_CELL};
    use crate::message::{JettonTransferMessage, TonMessage, TonMessageError, WithForwardPayload};
    use crate::TonAddress;

    const JETTON_TRANSFER_MSG : &str="b5ee9c720101020100a800016d0f8a7ea5001f5512dab844d643b9aca00800ef3b9902a271b2a01c8938a523cfe24e71847aaeb6a620001ed44a77ac0e709c1033428f030100d7259385618009dd924373a9aad41b28cec02da9384d67363af2034fc2a7ccc067e28d4110de86e66deb002365dfa32dfd419308ebdf35e0f6ba7c42534bbb5dab5e89e28ea3e0455cc2d2f00257a672371a90e149b7d25864dbfd44827cc1e8a30df1b1e0c4338502ade2ad96";
    const TRANSFER_PAYLOAD: &str = "259385618009DD924373A9AAD41B28CEC02DA9384D67363AF2034FC2A7CCC067E28D4110DE86E66DEB002365DFA32DFD419308EBDF35E0F6BA7C42534BBB5DAB5E89E28EA3E0455CC2D2F00257A672371A90E149B7D25864DBFD44827CC1E8A30DF1B1E0C4338502ADE2AD94";

    #[test]
    fn test_jetton_transfer_parser() -> Result<(), TonMessageError> {
        let boc = BagOfCells::parse_hex(JETTON_TRANSFER_MSG).unwrap();
        let cell = boc.single_root().unwrap();

        let result_jetton_transfer_msg = JettonTransferMessage::parse(cell)?;

        let transfer_message_cell = Arc::new(Cell::new(
            hex::decode(TRANSFER_PAYLOAD).unwrap(),
            862,
            vec![],
            false,
        )?);

        let expected_jetton_transfer_msg = JettonTransferMessage {
            query_id: 8819263745311958,
            amount: BigUint::from(1000000000u64),
            destination: TonAddress::from_str("EQB3ncyBUTjZUA5EnFKR5_EnOMI9V1tTEAAPaiU71gc4TiUt")
                .unwrap(),
            response_destination: TonAddress::from_str(
                "EQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAM9c",
            )
            .unwrap(),
            custom_payload: None,
            forward_ton_amount: BigUint::from(215000000u64),
            forward_payload: transfer_message_cell,
            forward_payload_layout: EitherCellLayout::Native,
        };

        assert_eq!(expected_jetton_transfer_msg, result_jetton_transfer_msg);
        Ok(())
    }

    #[test]
    fn test_jetton_transfer_builder() -> Result<(), TonMessageError> {
        let jetton_transfer_msg = JettonTransferMessage {
            query_id: 8819263745311958,
            amount: BigUint::from(1000000000u64),
            destination: TonAddress::from_str("EQB3ncyBUTjZUA5EnFKR5_EnOMI9V1tTEAAPaiU71gc4TiUt")
                .unwrap(),
            response_destination: TonAddress::from_str(
                "EQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAM9c",
            )
            .unwrap(),
            custom_payload: None,
            forward_ton_amount: BigUint::from(215000000u64),
            forward_payload: Arc::new(
                Cell::new(hex::decode(TRANSFER_PAYLOAD).unwrap(), 862, vec![], false).unwrap(),
            ),
            forward_payload_layout: EitherCellLayout::Native,
        };

        let result_cell = jetton_transfer_msg.build()?;

        let result_boc_serialized = BagOfCells::from_root(result_cell).serialize(false).unwrap();
        let expected_boc_serialized = hex::decode(JETTON_TRANSFER_MSG).unwrap();

        assert_eq!(expected_boc_serialized, result_boc_serialized);
        Ok(())
    }

    #[test]
    fn test_jetton_transfer_builder_bad_forward_amount() -> Result<(), TonMessageError> {
        let forward_payload =
            Arc::new(CellBuilder::new().store_byte(123).unwrap().build().unwrap());

        let mut jetton_transfer_msg = JettonTransferMessage::new(
            &TonAddress::from_str("EQB3ncyBUTjZUA5EnFKR5_EnOMI9V1tTEAAPaiU71gc4TiUt").unwrap(),
            &BigUint::from(300u32),
        );

        jetton_transfer_msg.with_forward_payload(BigUint::zero(), forward_payload.clone());
        assert!(jetton_transfer_msg.build().is_err());

        jetton_transfer_msg.with_forward_payload(BigUint::from(300u32), forward_payload.clone());
        assert!(jetton_transfer_msg.build().is_ok());

        jetton_transfer_msg.with_forward_payload(BigUint::zero(), EMPTY_ARC_CELL.clone());
        assert!(jetton_transfer_msg.build().is_ok());

        Ok(())
    }
}
