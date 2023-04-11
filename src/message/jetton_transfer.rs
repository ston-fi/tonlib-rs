use crate::address::TonAddress;
use crate::cell::{Cell, CellBuilder};
use crate::message::ZERO_COINS;
use anyhow::bail;
use num_bigint::BigUint;
use num_traits::Zero;
use std::sync::Arc;

const OP_REQUEST_TRANSFER: u32 = 0xf8a7ea5;

/// Creates a body for jetton transfer according to TL-B schema:
///
/// ```raw
/// transfer#0f8a7ea5 query_id:uint64 amount:(VarUInteger 16) destination:MsgAddress
///                  response_destination:MsgAddress custom_payload:(Maybe ^Cell)
///                  forward_ton_amount:(VarUInteger 16) forward_payload:(Either Cell ^Cell)
///                  = InternalMsgBody;
/// ```
pub struct JettonTransferBuilder {
    query_id: Option<u64>,
    amount: BigUint,
    destination: TonAddress,
    response_destination: Option<TonAddress>,
    custom_payload: Option<Arc<Cell>>,
    forward_ton_amount: BigUint,
    forward_payload: Option<Arc<Cell>>,
}

impl JettonTransferBuilder {
    pub fn new(destination: &TonAddress, amount: &BigUint) -> JettonTransferBuilder {
        JettonTransferBuilder {
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

    pub fn build(&self) -> anyhow::Result<Cell> {
        if self.forward_ton_amount.is_zero() && self.forward_payload.is_some() {
            bail!("forward_ton_amount must be positive when specifying forward_payload");
        }
        let mut message = CellBuilder::new();
        message.store_u32(32, OP_REQUEST_TRANSFER)?;
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
        message.build()
    }
}
