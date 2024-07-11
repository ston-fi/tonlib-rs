use num_bigint::BigUint;

use super::JETTON_BURN;
use crate::address::TonAddress;
use crate::cell::{ArcCell, Cell, CellBuilder};
use crate::message::{InvalidMessage, RawMessageUtils, TonMessageError};
use crate::tl::RawMessage;

/// Creates a body for jetton burn according to TL-B schema:
///
/// ```raw
/// burn#595f07bc query_id:uint64 amount:(VarUInteger 16)
///               response_destination:MsgAddress custom_payload:(Maybe ^Cell)
///               = InternalMsgBody;
/// ```
pub struct JettonBurnMessage {
    /// arbitrary request number.
    pub query_id: u64,
    /// amount of burned jettons
    pub amount: BigUint,
    /// address where to send a response with confirmation of a successful burn and the rest of the incoming message coins.
    pub response_destination: TonAddress,
    /// optional custom data (which is used by either sender or receiver jetton wallet for inner logic).
    pub custom_payload: Option<ArcCell>,
}

impl JettonBurnMessage {
    pub fn new(amount: &BigUint) -> Self {
        JettonBurnMessage {
            query_id: 0,
            amount: amount.clone(),
            response_destination: TonAddress::null(),
            custom_payload: None,
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

    pub fn with_custom_payload(&mut self, custom_payload: &ArcCell) -> &mut Self {
        self.custom_payload = Some(custom_payload.clone());
        self
    }

    pub fn build(&self) -> Result<Cell, TonMessageError> {
        let mut message = CellBuilder::new();
        message.store_u32(32, JETTON_BURN)?;
        message.store_u64(64, self.query_id)?;
        message.store_coins(&self.amount)?;
        message.store_address(&self.response_destination)?;
        if let Some(cp) = self.custom_payload.as_ref() {
            message.store_bit(true)?;
            message.store_reference(cp)?;
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
        if opcode != JETTON_BURN {
            let invalid = InvalidMessage {
                opcode: Some(opcode),
                query_id: Some(query_id),
                message: format!("Unexpected opcode.  {0:08x} expected", JETTON_BURN),
            };
            return Err(TonMessageError::InvalidMessage(invalid));
        }
        let amount = parser.load_coins()?;
        let response_destination = parser.load_address()?;
        let has_custom_payload = parser.load_bit()?;
        parser.ensure_empty()?;

        let custom_payload = if has_custom_payload {
            cell.expect_reference_count(1)?;
            Some(cell.reference(0)?.clone())
        } else {
            cell.expect_reference_count(0)?;
            None
        };

        let result = JettonBurnMessage {
            query_id,
            amount,
            response_destination,
            custom_payload,
        };
        Ok(result)
    }
}
