use num_bigint::BigUint;
use num_traits::Zero;

use super::{JETTON_TRANSFER, JETTON_TRANSFER_NOTIFICATION};
use crate::address::TonAddress;
use crate::cell::{ArcCell, Cell, CellBuilder};
use crate::message::{InvalidMessage, RawMessageUtils, TonMessageError, ZERO_COINS};
use crate::tl::RawMessage;

/// Creates a body for jetton transfer notification according to TL-B schema:
///
/// ```raw
///transfer_notification#7362d09c query_id:uint64 amount:(VarUInteger 16)
///                               sender:MsgAddress forward_payload:(Either Cell ^Cell)
///                               = InternalMsgBody;
/// ```
#[derive(Debug, PartialEq)]
pub struct JettonTransferNotificationMessage {
    /// should be equal with request's query_id.
    pub query_id: u64,
    /// amount of transferred jettons.
    pub amount: BigUint,
    /// is address of the previous owner of transferred jettons.
    pub sender: TonAddress,
    ///  the amount of nanotons to be sent to the destination address.
    pub forward_ton_amount: BigUint,
    ///  optional custom data that should be sent to the destination address.
    pub forward_payload: Option<ArcCell>,
}

impl JettonTransferNotificationMessage {
    pub fn new(sender: &TonAddress, amount: &BigUint) -> Self {
        JettonTransferNotificationMessage {
            query_id: 0,
            amount: amount.clone(),
            sender: sender.clone(),
            forward_ton_amount: ZERO_COINS.clone(),
            forward_payload: None,
        }
    }

    pub fn with_query_id(&mut self, query_id: u64) -> &mut Self {
        self.query_id = query_id;
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
        message.store_u32(32, JETTON_TRANSFER_NOTIFICATION)?;
        message.store_u64(64, self.query_id)?;
        message.store_coins(&self.amount)?;
        message.store_address(&self.sender)?;
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
                message: format!(
                    "Unexpected opcode.  {0:08x} expected",
                    JETTON_TRANSFER_NOTIFICATION
                ),
            };
            return Err(TonMessageError::InvalidMessage(invalid));
        }
        let amount = parser.load_coins()?;
        let sender = parser.load_address()?;
        let forward_ton_amount = parser.load_coins()?;
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
            forward_ton_amount,
            forward_payload,
        };

        Ok(result)
    }
}
