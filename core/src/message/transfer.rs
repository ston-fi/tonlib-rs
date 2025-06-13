use std::sync::Arc;

use super::{
    CommonMsgInfo, ExternalIncomingMessage, ExternalOutgoingMessage, InternalMessage, TonMessage,
    TonMessageError,
};
use crate::cell::{ArcCell, Cell, CellBuilder, EitherCellLayout};

#[derive(Clone, Debug, PartialEq)]
pub struct TransferMessage {
    pub common_msg_info: CommonMsgInfo,
    pub state_init: Option<ArcCell>,
    pub body: ArcCell,
}

impl TransferMessage {
    pub fn new(common_msg_info: CommonMsgInfo, body: ArcCell) -> Self {
        TransferMessage {
            common_msg_info,
            state_init: None,
            body,
        }
    }

    pub fn with_state_init(&mut self, state_init: Cell) -> &mut Self {
        self.with_state_init_ref(&Arc::new(state_init))
    }

    pub fn with_state_init_ref(&mut self, state_init: &ArcCell) -> &mut Self {
        self.state_init = Some(state_init.clone());
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
                builder.store_bit(false)?; // currency_coll
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
        if let Some(state_init) = &self.state_init {
            builder.store_bit(true)?;
            builder.store_either_cell_or_cell_ref(state_init, EitherCellLayout::ToRef)?;
        } else {
            builder.store_bit(false)?;
        }

        builder.store_either_cell_or_cell_ref(&self.body, EitherCellLayout::ToRef)?;

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
            let _currency_coll = parser.load_bit()?;
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
        let state_init = if parser.load_bit()? {
            Some(parser.load_either_cell_or_cell_ref()?)
        } else {
            None
        };
        let body = parser.load_either_cell_or_cell_ref()?;

        parser.ensure_empty()?;

        Ok(TransferMessage {
            common_msg_info,
            state_init,
            body,
        })
    }
}

#[cfg(test)]
mod test {
    use num_bigint::BigUint;

    use super::TransferMessage;
    use crate::cell::{Cell, EMPTY_ARC_CELL};
    use crate::message::{CommonMsgInfo, InternalMessage, TonMessage};
    use crate::tlb_types::tlb::TLB;
    use crate::TonAddress;

    fn make_internal() -> CommonMsgInfo {
        CommonMsgInfo::InternalMessage(InternalMessage {
            ihr_disabled: true,
            bounce: false,
            bounced: false,
            src: TonAddress::NULL,
            dest: TonAddress::NULL,
            value: BigUint::from(100000_u32),
            ihr_fee: BigUint::from(0_u32),
            fwd_fee: BigUint::from(1000_u32),
            created_lt: 0,
            created_at: 0,
        })
    }

    #[test]
    fn test_transfer_message_parser() -> anyhow::Result<()> {
        let transfer_message = TransferMessage::new(make_internal(), EMPTY_ARC_CELL.clone());
        let transfer_cell = transfer_message.build()?;
        let transfer_parsed = TransferMessage::parse(&transfer_cell)?;
        assert_eq!(transfer_message, transfer_parsed);
        Ok(())
    }

    #[test]
    fn test_transfer_msg_with_state_init() -> anyhow::Result<()> {
        let mut transfer_message = TransferMessage::new(make_internal(), EMPTY_ARC_CELL.clone());
        let state_init = Cell::from_boc_hex("b5ee9c720102160100030400020134020100510000082f29a9a31738dd3a33f904d35e2f4f6f9af2d2f9c563c05faa6bb0b12648d5632083ea3f89400114ff00f4a413f4bcf2c80b03020120090404f8f28308d71820d31fd31fd31f02f823bbf264ed44d0d31fd31fd3fff404d15143baf2a15151baf2a205f901541064f910f2a3f80024a4c8cb1f5240cb1f5230cbff5210f400c9ed54f80f01d30721c0009f6c519320d74a96d307d402fb00e830e021c001e30021c002e30001c0039130e30d03a4c8cb1f12cb1fcbff08070605000af400c9ed54006c810108d718fa00d33f305224810108f459f2a782106473747270748018c8cb05cb025005cf165003fa0213cb6acb1f12cb3fc973fb000070810108d718fa00d33fc8542047810108f451f2a782106e6f746570748018c8cb05cb025006cf165004fa0214cb6a12cb1fcb3fc973fb0002006ed207fa00d4d422f90005c8ca0715cbffc9d077748018c8cb05cb0222cf165005fa0214cb6b12ccccc973fb00c84014810108f451f2a702020148130a0201200c0b0059bd242b6f6a2684080a06b90fa0218470d4080847a4937d29910ce6903e9ff9837812801b7810148987159f31840201200e0d0011b8c97ed44d0d70b1f8020158120f02012011100019af1df6a26840106b90eb858fc00019adce76a26840206b90eb85ffc0003db29dfb513420405035c87d010c00b23281f2fff274006040423d029be84c6002e6d001d0d3032171b0925f04e022d749c120925f04e002d31f218210706c7567bd22821064737472bdb0925f05e003fa403020fa4401c8ca07cbffc9d0ed44d0810140d721f404305c810108f40a6fa131b3925f07e005d33fc8258210706c7567ba923830e30d03821064737472ba925f06e30d1514008a5004810108f45930ed44d0810140d720c801cf16f400c9ed540172b08e23821064737472831eb17080185005cb055003cf1623fa0213cb6acb1fcb3fc98040fb00925f03e2007801fa00f40430f8276f2230500aa121bef2e0508210706c7567831eb17080185004cb0526cf1658fa0219f400cb6917cb1f5260cb3f20c98040fb0006")?;
        transfer_message.with_state_init(state_init);

        let transfer_cell = transfer_message.build()?;
        let transfer_parsed = TransferMessage::parse(&transfer_cell)?;
        assert_eq!(transfer_message, transfer_parsed);
        assert_eq!(transfer_cell.to_boc_hex(false)?, "b5ee9c720102180100031e0002284030186a00101f400000000000000000000000070102020134030400000114ff00f4a413f4bcf2c80b0500510000082f29a9a31738dd3a33f904d35e2f4f6f9af2d2f9c563c05faa6bb0b12648d5632083ea3f89400201200607020148080904f8f28308d71820d31fd31fd31f02f823bbf264ed44d0d31fd31fd3fff404d15143baf2a15151baf2a205f901541064f910f2a3f80024a4c8cb1f5240cb1f5230cbff5210f400c9ed54f80f01d30721c0009f6c519320d74a96d307d402fb00e830e021c001e30021c002e30001c0039130e30d03a4c8cb1f12cb1fcbff0a0b0c0d02e6d001d0d3032171b0925f04e022d749c120925f04e002d31f218210706c7567bd22821064737472bdb0925f05e003fa403020fa4401c8ca07cbffc9d0ed44d0810140d721f404305c810108f40a6fa131b3925f07e005d33fc8258210706c7567ba923830e30d03821064737472ba925f06e30d0e0f0201201011006ed207fa00d4d422f90005c8ca0715cbffc9d077748018c8cb05cb0222cf165005fa0214cb6b12ccccc973fb00c84014810108f451f2a7020070810108d718fa00d33fc8542047810108f451f2a782106e6f746570748018c8cb05cb025006cf165004fa0214cb6a12cb1fcb3fc973fb0002006c810108d718fa00d33f305224810108f459f2a782106473747270748018c8cb05cb025005cf165003fa0213cb6acb1f12cb3fc973fb00000af400c9ed54007801fa00f40430f8276f2230500aa121bef2e0508210706c7567831eb17080185004cb0526cf1658fa0219f400cb6917cb1f5260cb3f20c98040fb0006008a5004810108f45930ed44d0810140d720c801cf16f400c9ed540172b08e23821064737472831eb17080185005cb055003cf1623fa0213cb6acb1fcb3fc98040fb00925f03e202012012130059bd242b6f6a2684080a06b90fa0218470d4080847a4937d29910ce6903e9ff9837812801b7810148987159f318402015814150011b8c97ed44d0d70b1f8003db29dfb513420405035c87d010c00b23281f2fff274006040423d029be84c6002012016170019adce76a26840206b90eb85ffc00019af1df6a26840106b90eb858fc0");
        Ok(())
    }
}
