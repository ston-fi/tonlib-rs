use crate::cell::Cell;
use crate::{address::TonAddress, cell::CellBuilder};
use crc::{Crc, CRC_32_ISO_HDLC};
use num_bigint::BigUint;
use num_traits::Zero;
use std::sync::Arc;

use crate::message::{TonMessageError, ZERO_COINS};

// Constants from jetton standart
// https://github.com/ton-blockchain/TEPs/blob/master/text/0074-jettons-standard.md

// crc32('transfer query_id:uint64 amount:VarUInteger 16 destination:MsgAddress response_destination:MsgAddress custom_payload:Maybe ^Cell forward_ton_amount:VarUInteger 16 forward_payload:Either Cell ^Cell = InternalMsgBody') = 0x8f8a7ea5 & 0x7fffffff = 0xf8a7ea5
// crc32('transfer_notification query_id:uint64 amount:VarUInteger 16 sender:MsgAddress forward_payload:Either Cell ^Cell = InternalMsgBody') = 0xf362d09c & 0x7fffffff = 0x7362d09c
// crc32('excesses query_id:uint64 = InternalMsgBody') = 0x553276db | 0x80000000 = 0xd53276db
// crc32('burn query_id:uint64 amount:VarUInteger 16 response_destination:MsgAddress custom_payload:Maybe ^Cell = InternalMsgBody') = 0x595f07bc & 0x7fffffff = 0x595f07bc
// crc32('internal_transfer query_id:uint64 amount:VarUInteger 16 from:MsgAddress response_address:MsgAddress forward_ton_amount:VarUInteger 16 forward_payload:Either Cell ^Cell = InternalMsgBody') = 0x978d4519 & 0x7fffffff = 0x178d4519
// crc32('burn_notification query_id:uint64 amount:VarUInteger 16 sender:MsgAddress response_destination:MsgAddress = InternalMsgBody') = 0x7bdd97de & 0x7fffffff = 0x7bdd97de

pub const JETTON_TRANSFER: u32 = 0xf8a7ea5;
pub const JETTON_TRANSFER_NOTIFICATION: u32 = 0x7362d09c;
pub const JETTON_INTERNAL_TRANSFER: u32 = 0x178d4519;
pub const JETTON_EXCESSES: u32 = 0xd53276db;
pub const JETTON_BURN: u32 = 0x595f07bc;
pub const JETTON_BURN_NOTIFICATION: u32 = 0x7bdd97de;

/// Creates a body for jetton transfer according to TL-B schema:
///
/// ```raw
/// transfer#0f8a7ea5 query_id:uint64 amount:(VarUInteger 16) destination:MsgAddress
///                  response_destination:MsgAddress custom_payload:(Maybe ^Cell)
///                  forward_ton_amount:(VarUInteger 16) forward_payload:(Either Cell ^Cell)
///                  = InternalMsgBody;
/// ```
pub struct JettonTransferMessage {
    pub query_id: Option<u64>,
    pub amount: BigUint,
    pub destination: TonAddress,
    pub response_destination: Option<TonAddress>,
    pub custom_payload: Option<Arc<Cell>>,
    pub forward_ton_amount: BigUint,
    pub forward_payload: Option<Arc<Cell>>,
}

impl JettonTransferMessage {
    pub fn new(destination: &TonAddress, amount: &BigUint) -> JettonTransferMessage {
        JettonTransferMessage {
            query_id: None,
            amount: amount.clone(),
            destination: destination.clone(),
            response_destination: None,
            custom_payload: None,
            forward_ton_amount: ZERO_COINS.clone(),
            forward_payload: None,
        }
    }

    pub fn with_query_id(&mut self, query_id: u64) -> &mut Self {
        self.query_id = Some(query_id);
        self
    }

    pub fn with_response_destination(&mut self, response_destination: &TonAddress) -> &mut Self {
        self.response_destination = Some(response_destination.clone());
        self
    }

    pub fn with_custom_payload(&mut self, custom_payload: Cell) -> &mut Self {
        self.with_custom_payload_ref(&Arc::new(custom_payload))
    }

    pub fn with_custom_payload_ref(&mut self, custom_payload_ref: &Arc<Cell>) -> &mut Self {
        self.custom_payload = Some(custom_payload_ref.clone());
        self
    }

    pub fn with_forward(
        &mut self,
        forward_ton_amount: &BigUint,
        forward_payload: Cell,
    ) -> &mut Self {
        self.with_forward_ref(forward_ton_amount, &Arc::new(forward_payload))
    }

    pub fn with_forward_ref(
        &mut self,
        forward_ton_amount: &BigUint,
        forward_payload: &Arc<Cell>,
    ) -> &mut Self {
        self.forward_ton_amount = forward_ton_amount.clone();
        self.forward_payload = Some(forward_payload.clone());
        self
    }

    pub fn build(&self) -> Result<Cell, TonMessageError> {
        if self.forward_ton_amount.is_zero() && self.forward_payload.is_some() {
            return Err(TonMessageError::ForwardTonAmountIsNegative);
        }

        let mut message = CellBuilder::new();
        message.store_u32(32, JETTON_TRANSFER)?;
        message.store_u64(64, self.query_id.unwrap_or_default())?;
        message.store_coins(&self.amount)?;
        message.store_address(&self.destination)?;
        message.store_address(
            self.response_destination
                .as_ref()
                .unwrap_or_else(|| &TonAddress::NULL),
        )?;
        if let Some(cp) = self.custom_payload.as_ref() {
            message.store_bit(true)?;
            message.store_reference(&cp)?;
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
}

#[allow(dead_code)]
fn calc_checksum(command: &str) -> u32 {
    let crc = Crc::<u32>::new(&CRC_32_ISO_HDLC);
    crc.checksum(command.as_bytes())
}

#[allow(dead_code)]
fn calc_opcode(command: &str) -> u32 {
    calc_checksum(command) & 0x7fffffff
}
