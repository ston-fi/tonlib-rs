use std::sync::Arc;

use super::{
    CommonMsgInfo, ExternalIncomingMessage, ExternalOutgoingMessage, InternalMessage, TonMessage,
    TonMessageError,
};
use crate::cell::{ArcCell, Cell, CellBuilder};

#[derive(Clone, Debug, PartialEq)]
pub struct TransferMessage {
    pub common_msg_info: CommonMsgInfo,
    pub state_init: Option<ArcCell>,
    pub data: Option<ArcCell>,
}

impl TransferMessage {
    pub fn new(common_msg_info: CommonMsgInfo) -> Self {
        TransferMessage {
            common_msg_info,
            state_init: None,
            data: None,
        }
    }

    pub fn with_state_init(&mut self, state_init: Cell) -> &mut Self {
        self.with_state_init_ref(&Arc::new(state_init))
    }

    pub fn with_state_init_ref(&mut self, state_init: &ArcCell) -> &mut Self {
        self.state_init = Some(state_init.clone());
        self
    }

    pub fn with_data(&mut self, data: ArcCell) -> &mut Self {
        self.data = Some(data);
        self
    }
}

impl TonMessage for TransferMessage {
    fn build(&self) -> Result<Cell, TonMessageError> {
        let mut builder = CellBuilder::new();

        match &self.common_msg_info {
            CommonMsgInfo::InternalMessage(m) => {
                builder.store_bit(false)?; // bit0 (is_external)
                builder.store_bit(m.ihr_disabled)?; // ihr_disabled
                builder.store_bit(m.bounce)?; // bounce
                builder.store_bit(m.bounced)?; // bounced
                builder.store_address(&m.src)?; // src_addr
                builder.store_address(&m.dest)?; // dest_addr
                builder.store_coins(&m.value)?; // value
                builder.store_coins(&m.ihr_fee)?; // ihr_fees
                builder.store_coins(&m.fwd_fee)?; // fwd_fees
                builder.store_u64(64, m.created_lt)?; // created_lt
                builder.store_u32(32, m.created_at)?; // created_at
            }
            CommonMsgInfo::ExternalIncomingMessage(m) => {
                builder.store_bit(true)?; // bit0 (is_external)
                builder.store_bit(false)?; // bit0 (is_outgoing)
                builder.store_address(&m.src)?;
                builder.store_address(&m.dest)?;
                builder.store_coins(&m.import_fee)?;
            }
            CommonMsgInfo::ExternalOutgoingMessage(m) => {
                builder.store_bit(true)?; // bit0 (is_external)
                builder.store_bit(true)?; // bit0 (is_outgoing)
                builder.store_address(&m.src)?;
                builder.store_address(&m.dest)?;
                builder.store_u64(64, m.created_lt)?; // created_lt
                builder.store_u32(32, m.created_at)?; // created_at
            }
        }
        builder.store_maybe_cell_ref(&self.state_init)?;

        builder.store_maybe_cell_ref(&self.data)?;

        Ok(builder.build()?)
    }

    fn parse(cell: &Cell) -> Result<Self, TonMessageError> {
        let mut parser = cell.parser();
        //   TODO: Review structure of transfer message
        let bit0 = parser.load_bit()?;

        // internal message
        let common_msg_info = if !bit0 {
            let ihr_disabled = parser.load_bit()?;
            let bounce = parser.load_bit()?;
            let bounced = parser.load_bit()?;
            let src = parser.load_address()?;
            let dest = parser.load_address()?;
            let value = parser.load_coins()?;
            let ihr_fee = parser.load_coins()?;
            let fwd_fee = parser.load_coins()?;
            let created_lt = parser.load_u64(64)?;
            let created_at = parser.load_u32(32)?;

            CommonMsgInfo::InternalMessage(InternalMessage {
                ihr_disabled,
                bounce,
                bounced,
                src,
                dest,
                value,
                ihr_fee,
                fwd_fee,
                created_lt,
                created_at,
            })
        } else {
            let bit1 = parser.load_bit()?;

            if !bit1 {
                let src = parser.load_address()?;
                let dest = parser.load_address()?;
                let import_fee = parser.load_coins()?;

                CommonMsgInfo::ExternalIncomingMessage(ExternalIncomingMessage {
                    src,
                    dest,
                    import_fee,
                })
            } else {
                let src = parser.load_address()?;
                let dest = parser.load_address()?;

                let created_lt = parser.load_u64(64)?;
                let created_at = parser.load_u32(32)?;

                CommonMsgInfo::ExternalOutgoingMessage(ExternalOutgoingMessage {
                    src,
                    dest,
                    created_lt,
                    created_at,
                })
            }
        };
        let state_init = parser.load_maybe_cell_ref()?;
        let data = parser.load_maybe_cell_ref()?;

        parser.ensure_empty()?;

        Ok(TransferMessage {
            common_msg_info,
            state_init,
            data,
        })
    }
}
