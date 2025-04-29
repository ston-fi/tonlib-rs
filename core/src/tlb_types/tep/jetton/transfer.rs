use num_bigint::BigUint;

use crate::cell::ArcCell;
use crate::message::JETTON_TRANSFER;
use crate::tlb_types::block::msg_address::MsgAddress;
use crate::tlb_types::primitives::either::{EitherRef, EitherRefLayout};
use crate::tlb_types::primitives::reference::Ref;
use crate::tlb_types::tlb::{TLBPrefix, TLB};

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
    pub destination: MsgAddress,
    /// address where to send a response with confirmation of a successful transfer and the rest of the incoming message Toncoins.
    pub response_destination: MsgAddress,
    /// optional custom data (which is used by either sender or receiver jetton wallet for inner logic).
    pub custom_payload: Option<Ref<ArcCell>>,
    /// the amount of nanotons to be sent to the destination address.
    pub forward_ton_amount: BigUint,
    /// optional custom data that should be sent to the destination address.
    pub forward_payload: EitherRef<ArcCell>,
}

impl JettonTransferMessage {
    pub fn new(
        query_id: u64,
        amount: &BigUint,
        destination: MsgAddress,
        response_destination: MsgAddress,
        custom_payload: Option<ArcCell>,
        forward_ton_amount: &BigUint,
        forward_payload: &ArcCell,
    ) -> Self {
        let custom_payload = custom_payload.map(Ref);

        JettonTransferMessage {
            query_id,
            amount: amount.clone(),
            destination,
            response_destination,
            custom_payload,
            forward_ton_amount: forward_ton_amount.clone(),
            forward_payload: EitherRef {
                value: forward_payload.clone(),
                layout: EitherRefLayout::ToRef,
            },
        }
    }
}

impl TLB for JettonTransferMessage {
    const PREFIX: TLBPrefix = TLBPrefix::new(32, JETTON_TRANSFER as u64);
    fn read_definition(
        parser: &mut crate::cell::CellParser,
    ) -> Result<Self, crate::cell::TonCellError> {
        let query_id = parser.load_u64(64)?;
        let amount = parser.load_coins()?;
        let destination = TLB::read(parser)?;
        let response_destination = TLB::read(parser)?;
        let custom_payload = TLB::read(parser)?;
        let forward_ton_amount = parser.load_coins()?;
        let forward_payload = TLB::read(parser)?;

        parser.ensure_empty()?;

        let result = JettonTransferMessage {
            query_id,
            amount,
            destination,
            response_destination,
            custom_payload,
            forward_ton_amount,
            forward_payload,
        };

        Ok(result)
    }

    fn write_definition(
        &self,
        dst: &mut crate::cell::CellBuilder,
    ) -> Result<(), crate::cell::TonCellError> {
        dst.store_u64(64, self.query_id)?;
        dst.store_coins(&self.amount)?;
        self.destination.write(dst)?;
        self.response_destination.write(dst)?;
        self.custom_payload.write(dst)?;
        dst.store_coins(&self.forward_ton_amount)?;
        self.forward_payload.write(dst)?;

        dst.build()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::sync::Arc;

    use lazy_static::lazy_static;
    use num_bigint::BigUint;

    use super::JettonTransferMessage;
    use crate::cell::Cell;
    use crate::message::TonMessageError;
    use crate::tlb_types::primitives::either::{EitherRef, EitherRefLayout};
    use crate::tlb_types::tlb::TLB;
    use crate::TonAddress;

    const JETTON_TRANSFER_MSG : &str="b5ee9c720101020100a800016d0f8a7ea5001f5512dab844d643b9aca00800ef3b9902a271b2a01c8938a523cfe24e71847aaeb6a620001ed44a77ac0e709c1033428f030100d7259385618009dd924373a9aad41b28cec02da9384d67363af2034fc2a7ccc067e28d4110de86e66deb002365dfa32dfd419308ebdf35e0f6ba7c42534bbb5dab5e89e28ea3e0455cc2d2f00257a672371a90e149b7d25864dbfd44827cc1e8a30df1b1e0c4338502ade2ad96";
    const TRANSFER_PAYLOAD: &str = "259385618009DD924373A9AAD41B28CEC02DA9384D67363AF2034FC2A7CCC067E28D4110DE86E66DEB002365DFA32DFD419308EBDF35E0F6BA7C42534BBB5DAB5E89E28EA3E0455CC2D2F00257A672371A90E149B7D25864DBFD44827CC1E8A30DF1B1E0C4338502ADE2AD94";

    lazy_static! {
        static ref TRANSFER_MESSAGE_CELL: Arc<Cell> = Arc::new(
            Cell::new(hex::decode(TRANSFER_PAYLOAD).unwrap(), 862, vec![], false,).unwrap()
        );
        static ref EXPECTED_JETTON_TRANSFER_MSG: JettonTransferMessage = JettonTransferMessage {
            query_id: 8819263745311958,
            amount: BigUint::from(1000000000u64),
            destination: TonAddress::from_str("EQB3ncyBUTjZUA5EnFKR5_EnOMI9V1tTEAAPaiU71gc4TiUt")
                .unwrap()
                .to_msg_address(),
            response_destination: TonAddress::from_str(
                "EQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAM9c",
            )
            .unwrap()
            .to_msg_address(),
            custom_payload: None,
            forward_ton_amount: BigUint::from(215000000u64),
            forward_payload: EitherRef {
                value: TRANSFER_MESSAGE_CELL.clone(),
                layout: EitherRefLayout::ToRef,
            },
        };
    }

    #[test]
    fn test_jetton_transfer_parser() -> Result<(), TonMessageError> {
        let result_jetton_transfer_msg = JettonTransferMessage::from_boc_hex(JETTON_TRANSFER_MSG)?;

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
                .unwrap()
                .to_msg_address(),
            response_destination: TonAddress::from_str(
                "EQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAM9c",
            )
            .unwrap()
            .to_msg_address(),
            custom_payload: None,
            forward_ton_amount: BigUint::from(215000000u64),
            forward_payload: EitherRef {
                value: transfer_message_cell,
                layout: EitherRefLayout::ToRef,
            },
        };

        assert_eq!(expected_jetton_transfer_msg, result_jetton_transfer_msg);
        Ok(())
    }

    #[test]
    fn test_jetton_transfer_builder() -> anyhow::Result<()> {
        let jetton_transfer_msg = EXPECTED_JETTON_TRANSFER_MSG.clone();
        let result_cell = jetton_transfer_msg.to_cell()?;
        let result_boc_serialized = result_cell.to_boc(false)?;

        let expected_boc_serialized = hex::decode(JETTON_TRANSFER_MSG)?;

        assert_eq!(expected_boc_serialized, result_boc_serialized);
        Ok(())
    }
}
