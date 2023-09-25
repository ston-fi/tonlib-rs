use std::sync::Arc;

use num_bigint::BigUint;

use crate::address::TonAddress;
use crate::cell::{Cell, CellBuilder};
use crate::message::ZERO_COINS;

use super::TonMessageError;

pub struct TransferBuilder {
    dest: TonAddress,
    value: BigUint,
    state_init: Option<Arc<Cell>>,
    data: Option<Arc<Cell>>,
}

impl TransferBuilder {
    pub fn new(dest: &TonAddress, value: &BigUint) -> Self {
        TransferBuilder {
            dest: dest.clone(),
            value: value.clone(),
            state_init: None,
            data: None,
        }
    }

    pub fn with_state_init(&mut self, state_init: Cell) -> &mut Self {
        self.with_state_init_ref(&Arc::new(state_init))
    }

    pub fn with_state_init_ref(&mut self, state_init: &Arc<Cell>) -> &mut Self {
        self.state_init = Some(state_init.clone());
        self
    }

    pub fn with_data(&mut self, data: Cell) -> &mut Self {
        self.with_data_ref(&Arc::new(data))
    }

    pub fn with_data_ref(&mut self, data: &Arc<Cell>) -> &mut Self {
        self.data = Some(data.clone());
        self
    }

    pub fn build(&self) -> Result<Cell, TonMessageError> {
        let mut builder = CellBuilder::new();
        builder.store_bit(false)?; // bit0
        builder.store_bit(true)?; // ihr_disabled
        builder.store_bit(true)?; // bounce
        builder.store_bit(false)?; // bounced
        builder.store_address(&TonAddress::NULL)?; // src_addr
        builder.store_address(&self.dest)?; // dest_addr
        builder.store_coins(&self.value)?; // value
        builder.store_bit(false)?; // currency_coll
        builder.store_coins(&ZERO_COINS)?; // ihr_fees
        builder.store_coins(&ZERO_COINS)?; // fwd_fees
        builder.store_u64(64, 0)?; // created_lt
        builder.store_u32(32, 0)?; // created_at
        builder.store_bit(self.state_init.is_some())?; // state_init?
        if let Some(state_init) = self.state_init.as_ref() {
            builder.store_reference(state_init)?;
        }
        builder.store_bit(self.data.is_some())?; // data?
        if let Some(data) = self.data.as_ref() {
            builder.store_reference(data)?;
        }
        Ok(builder.build()?)
    }
}
