use num_bigint::BigUint;
use num_traits::Zero;

use super::JETTON_TRANSFER;
use crate::address::TonAddress;
use crate::cell::{ArcCell, Cell, CellBuilder};
use crate::message::{InvalidMessage, RawMessageUtils, TonMessageError, ZERO_COINS};
use crate::tl::RawMessage;

/// Creates a body for jetton transfer according to TL-B schema:
///
/// ```raw
/// transfer#0f8a7ea5 query_id:uint64 amount:(VarUInteger 16) destination:MsgAddress
///                  response_destination:MsgAddress custom_payload:(Maybe ^Cell)
///                  forward_ton_amount:(VarUInteger 16) forward_payload:(Either Cell ^Cell)
///                  = InternalMsgBody;
/// ```
#[derive(Debug, PartialEq)]
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
    pub forward_payload: Option<ArcCell>,
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
            forward_payload: None,
        }
    }

    pub fn with_query_id(&mut self, query_id: u64) -> &mut Self {
        self.query_id = query_id;
        self
    }

    pub fn with_response_destination(&mut self, response_destination: &TonAddress) -> &mut Self {
        self.response_destination = response_destination.clone();
        self
    }

    pub fn with_custom_payload<T>(&mut self, custom_payload: T) -> &mut Self
    where
        T: AsRef<ArcCell>,
    {
        self.custom_payload = Some(custom_payload.as_ref().clone());
        self
    }

    pub fn with_forward_payload<T>(
        &mut self,
        forward_ton_amount: &BigUint,
        forward_payload: T,
    ) -> &mut Self
    where
        T: AsRef<ArcCell>,
    {
        self.forward_ton_amount.clone_from(forward_ton_amount);
        self.forward_payload = Some(forward_payload.as_ref().clone());
        self
    }

    pub fn build(&self) -> Result<Cell, TonMessageError> {
        if self.forward_ton_amount.is_zero() && self.forward_payload.is_some() {
            return Err(TonMessageError::ForwardTonAmountIsNegative);
        }

        let mut message = CellBuilder::new();
        message.store_u32(32, JETTON_TRANSFER)?;
        message.store_u64(64, self.query_id)?;
        message.store_coins(&self.amount)?;
        message.store_address(&self.destination)?;
        message.store_address(&self.response_destination)?;
        if let Some(cp) = self.custom_payload.as_ref() {
            message.store_bit(true)?;
            message.store_reference(cp)?;
        } else {
            message.store_bit(false)?;
        }
        message.store_coins(&self.forward_ton_amount)?;
        if let Some(fp) = self.forward_payload.as_ref() {
            message.store_bit(true)?;
            message.store_reference(fp)?;
        } else {
            message.store_bit(false)?;
        }
        Ok(message.build()?)
    }

    pub fn parse(msg: &RawMessage) -> Result<Self, TonMessageError> {
        let cell = (&msg).get_raw_data_cell()?;
        let mut parser = cell.parser();

        let opcode: u32 = parser.load_u32(32)?;
        let query_id = parser.load_u64(64)?;
        if opcode != JETTON_TRANSFER {
            let invalid = InvalidMessage {
                opcode: Some(opcode),
                query_id: Some(query_id),
                message: format!("Unexpected opcode.  {0:08x} expected", JETTON_TRANSFER),
            };
            return Err(TonMessageError::InvalidMessage(invalid));
        }
        let amount = parser.load_coins()?;
        let destination = parser.load_address()?;
        let response_destination = parser.load_address()?;
        let has_custom_payload = parser.load_bit()?;
        let forward_ton_amount = parser.load_coins()?;
        let has_forward_payload = parser.load_bit()?;
        parser.ensure_empty()?;

        let (custom_payload, forward_payload) = match (has_custom_payload, has_forward_payload) {
            (true, true) => {
                cell.expect_reference_count(2)?;
                (
                    Some(cell.reference(0)?.clone()),
                    Some(cell.reference(1)?.clone()),
                )
            }
            (true, false) => {
                cell.expect_reference_count(1)?;
                (Some(cell.reference(0)?.clone()), None)
            }
            (false, true) => {
                cell.expect_reference_count(1)?;
                (None, Some(cell.reference(0)?.clone()))
            }
            (false, false) => {
                cell.expect_reference_count(0)?;
                (None, None)
            }
        };

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
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::sync::Arc;

    use num_bigint::BigUint;
    use tokio_test::assert_ok;

    use crate::address::TonAddress;
    use crate::cell::{BagOfCells, Cell};
    use crate::message::JettonTransferMessage;
    use crate::tl::{AccountAddress, MsgData, RawMessage};
    // message origin: https://tonviewer.com/transaction/2e250e3c9367d8092f15e09fb3c3d750749187c2a528a616bf0e88e5f36ca3f4
    const JETTON_TRANSFER_MSG : &str="b5ee9c720101020100a800016d0f8a7ea5001f5512dab844d643b9aca00800ef3b9902a271b2a01c8938a523cfe24e71847aaeb6a620001ed44a77ac0e709c1033428f030100d7259385618009dd924373a9aad41b28cec02da9384d67363af2034fc2a7ccc067e28d4110de86e66deb002365dfa32dfd419308ebdf35e0f6ba7c42534bbb5dab5e89e28ea3e0455cc2d2f00257a672371a90e149b7d25864dbfd44827cc1e8a30df1b1e0c4338502ade2ad96";
    const TRANSFER_PAYLOAD: &str = "259385618009DD924373A9AAD41B28CEC02DA9384D67363AF2034FC2A7CCC067E28D4110DE86E66DEB002365DFA32DFD419308EBDF35E0F6BA7C42534BBB5DAB5E89E28EA3E0455CC2D2F00257A672371A90E149B7D25864DBFD44827CC1E8A30DF1B1E0C4338502ADE2AD94";

    #[test]
    fn test_jetton_transfer_parser() {
        let msg_data = hex::decode(JETTON_TRANSFER_MSG).unwrap();

        let raw_msg = RawMessage {
            source: AccountAddress {
                account_address: String::new(),
            },
            destination: AccountAddress {
                account_address: String::new(),
            },
            value: 0,
            fwd_fee: 0,
            ihr_fee: 0,
            created_lt: 0,
            body_hash: vec![],
            msg_data: MsgData::Raw {
                body: msg_data.clone(),
                init_state: vec![],
            },
        };

        let result_jetton_transfer_msg = assert_ok!(JettonTransferMessage::parse(&raw_msg));

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
            forward_payload: Some(Arc::new(
                Cell::new(hex::decode(TRANSFER_PAYLOAD).unwrap(), 862, vec![], false).unwrap(),
            )),
        };

        assert_eq!(expected_jetton_transfer_msg, result_jetton_transfer_msg);
    }
    #[test]
    fn test_jetton_transfer_builder() {
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
            forward_payload: Some(Arc::new(
                Cell::new(hex::decode(TRANSFER_PAYLOAD).unwrap(), 862, vec![], false).unwrap(),
            )),
        };

        let result_cell = assert_ok!(jetton_transfer_msg.build());

        let result_boc_serialized = BagOfCells::from_root(result_cell).serialize(false).unwrap();
        let expected_boc_serialized = hex::decode(JETTON_TRANSFER_MSG).unwrap();

        assert_eq!(expected_boc_serialized, result_boc_serialized)
    }
}
