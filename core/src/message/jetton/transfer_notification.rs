use num_bigint::BigUint;

use super::JETTON_TRANSFER_NOTIFICATION;
use crate::cell::{ArcCell, Cell, CellBuilder};
use crate::message::{InvalidMessage, TonMessageError};
use crate::TonAddress;

/// Creates a body for jetton transfer notification according to TL-B schema:
///
/// ```raw
///transfer_notification#7362d09c query_id:uint64 amount:(VarUInteger 16)
///                               sender:MsgAddress forward_payload:(Either Cell ^Cell)
///                               = InternalMsgBody;
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct JettonTransferNotificationMessage {
    /// should be equal with request's query_id.
    pub query_id: u64,
    /// amount of transferred jettons.
    pub amount: BigUint,
    /// is address of the previous owner of transferred jettons.
    pub sender: TonAddress,
    ///  optional custom data that should be sent to the destination address.
    pub forward_payload: Option<ArcCell>,
}

impl JettonTransferNotificationMessage {
    pub fn new(sender: &TonAddress, amount: &BigUint) -> Self {
        JettonTransferNotificationMessage {
            query_id: 0,
            amount: amount.clone(),
            sender: sender.clone(),
            forward_payload: None,
        }
    }

    pub fn with_query_id(&mut self, query_id: u64) -> &mut Self {
        self.query_id = query_id;
        self
    }

    pub fn with_forward_payload(&mut self, forward_payload: &ArcCell) -> &mut Self {
        self.forward_payload = Some(forward_payload.clone());
        self
    }

    pub fn build(&self) -> Result<Cell, TonMessageError> {
        let mut message = CellBuilder::new();
        message.store_u32(32, JETTON_TRANSFER_NOTIFICATION)?;
        message.store_u64(64, self.query_id)?;
        message.store_coins(&self.amount)?;
        message.store_address(&self.sender)?;
        if let Some(fp) = self.forward_payload.as_ref() {
            message.store_bit(true)?;
            message.store_reference(fp)?;
        } else {
            message.store_bit(false)?;
        }
        Ok(message.build()?)
    }

    pub fn parse(cell: &Cell) -> Result<Self, TonMessageError> {
        let mut parser = cell.parser();

        let opcode: u32 = parser.load_u32(32)?;
        let query_id = parser.load_u64(64)?;
        if opcode != JETTON_TRANSFER_NOTIFICATION {
            let invalid = InvalidMessage {
                opcode: Some(opcode),
                query_id: Some(query_id),
                message: format!(
                    "Unexpected opcode.  {0:08x} expected",
                    JETTON_TRANSFER_NOTIFICATION
                ),
            };
            return Err(TonMessageError::InvalidMessage(invalid));
        }
        let amount = parser.load_coins()?;
        let sender = parser.load_address()?;
        let has_forward_payload = parser.load_bit()?;
        parser.ensure_empty()?;

        let forward_payload = if has_forward_payload {
            cell.expect_reference_count(1)?;
            Some(cell.reference(0)?.clone())
        } else {
            cell.expect_reference_count(0)?;
            None
        };

        let result = JettonTransferNotificationMessage {
            query_id,
            amount,
            sender,
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

    use crate::cell::{BagOfCells, Cell};
    use crate::message::{JettonTransferNotificationMessage, TonMessageError};
    use crate::TonAddress;

    const JETTON_TRANSFER_NOTIFICATION_MSG: &str = "b5ee9c720101020100a60001647362d09c000000d2c7ceef23401312d008003be20895401cd8539741eb7815d5e63b3429014018d7e5f7800de16a984f27730100dd25938561800f2465b65c76b1b562f32423676970b431319419d5f45ffd2eeb2155ce6ab7eacc78ee0250ef0300077c4112a8039b0a72e83d6f02babcc766852028031afcbef001bc2d5309e4ee700257a672371a90e149b7d25864dbfd44827cc1e8a30df1b1e0c4338502ade2ad96";
    const TRANSFER_NOTIFICATION_PAYLOAD: &str = "25938561800f2465b65c76b1b562f32423676970b431319419d5f45ffd2eeb2155ce6ab7eacc78ee0250ef0300077c4112a8039b0a72e83d6f02babcc766852028031afcbef001bc2d5309e4ee700257a672371a90e149b7d25864dbfd44827cc1e8a30df1b1e0c4338502ade2ad94";

    #[test]
    fn test_jetton_transfer_notification_parser() -> Result<(), TonMessageError> {
        let boc = BagOfCells::parse_hex(JETTON_TRANSFER_NOTIFICATION_MSG).unwrap();
        let cell = boc.single_root().unwrap();

        let expected_jetton_transfer_notification_msg = JettonTransferNotificationMessage {
            query_id: 905295359779,
            amount: BigUint::from(20000000u64),
            sender: TonAddress::from_str("EQAd8QRKoA5sKcug9bwK6vMdmhSAoAxr8vvABvC1TCeTude5")
                .unwrap(),
            forward_payload: Some(Arc::new(
                Cell::new(
                    hex::decode(TRANSFER_NOTIFICATION_PAYLOAD).unwrap(),
                    886,
                    vec![],
                    false,
                )
                .unwrap(),
            )),
        };
        let result_jetton_transfer_msg = JettonTransferNotificationMessage::parse(cell)?;

        assert_eq!(
            expected_jetton_transfer_notification_msg,
            result_jetton_transfer_msg
        );
        Ok(())
    }

    #[test]
    fn test_jetton_transfer_notification_builder() -> Result<(), TonMessageError> {
        let jetton_transfer_notification_msg = JettonTransferNotificationMessage {
            query_id: 905295359779,
            amount: BigUint::from(20000000u64),
            sender: TonAddress::from_str("EQAd8QRKoA5sKcug9bwK6vMdmhSAoAxr8vvABvC1TCeTude5")
                .unwrap(),
            forward_payload: Some(Arc::new(
                Cell::new(
                    hex::decode(TRANSFER_NOTIFICATION_PAYLOAD).unwrap(),
                    886,
                    vec![],
                    false,
                )
                .unwrap(),
            )),
        };

        let result_cell = jetton_transfer_notification_msg.build()?;

        let expected_boc_serialized = hex::decode(JETTON_TRANSFER_NOTIFICATION_MSG).unwrap();
        let result_boc_serialized = BagOfCells::from_root(result_cell).serialize(false).unwrap();

        assert_eq!(expected_boc_serialized, result_boc_serialized);
        Ok(())
    }
}
