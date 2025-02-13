use crate::cell::{ArcCell, CellBuilder, CellParser, TonCellError};
use crate::tlb_types::primitives::option::OptionRef;
use crate::tlb_types::traits::TLBObject;

// https://github.com/ton-blockchain/ton/blob/59a8cf0ae5c3062d14ec4c89a04fee80b5fd05c1/crypto/block/block.tlb#L281
#[derive(Debug, Clone, PartialEq)]
pub struct StateInit {
    pub split_depth: Option<u8>,
    pub tick_tock: Option<TickTock>,
    pub code: OptionRef<ArcCell>,
    pub data: OptionRef<ArcCell>,
    // there is likely library:(HashmapE 256 SimpleLib)
    pub library: OptionRef<ArcCell>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TickTock {
    pub tick: bool,
    pub tock: bool,
}

impl StateInit {
    pub const fn new(code: ArcCell, data: ArcCell) -> Self {
        StateInit {
            split_depth: None,
            tick_tock: None,
            code: OptionRef::new(code),
            data: OptionRef::new(data),
            library: OptionRef::NONE,
        }
    }
}

impl TLBObject for StateInit {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        Ok(StateInit {
            split_depth: parser.load_number_optional(5)?,
            tick_tock: TLBObject::read(parser)?,
            code: TLBObject::read(parser)?,
            data: TLBObject::read(parser)?,
            library: TLBObject::read(parser)?,
        })
    }

    fn write_to(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        dst.store_number_optional(5, self.split_depth)?;
        self.tick_tock.write_to(dst)?;
        self.code.write_to(dst)?;
        self.data.write_to(dst)?;
        self.library.write_to(dst)?;
        Ok(())
    }
}

impl TLBObject for TickTock {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        let tick = parser.load_bit()?;
        let tock = parser.load_bit()?;
        Ok(TickTock { tick, tock })
    }

    fn write_to(&self, builder: &mut CellBuilder) -> Result<(), TonCellError> {
        builder.store_bit(self.tick)?;
        builder.store_bit(self.tock)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Deref;

    use super::*;
    use crate::cell::{BagOfCells, TonCellError};

    #[test]
    fn test_state_init_regular_contract() -> Result<(), TonCellError> {
        // state_init of UQCJ7Quj9gM_SE3uwOk3gEJC2JFQcgg0s7CSpLr7B_2yiHPG contract
        let state_init_hex = "b5ee9c720102160100030400020134020100510000082f29a9a31738dd3a33f904d35e2f4f6f9af2d2f9c563c05faa6bb0b12648d5632083ea3f89400114ff00f4a413f4bcf2c80b03020120090404f8f28308d71820d31fd31fd31f02f823bbf264ed44d0d31fd31fd3fff404d15143baf2a15151baf2a205f901541064f910f2a3f80024a4c8cb1f5240cb1f5230cbff5210f400c9ed54f80f01d30721c0009f6c519320d74a96d307d402fb00e830e021c001e30021c002e30001c0039130e30d03a4c8cb1f12cb1fcbff08070605000af400c9ed54006c810108d718fa00d33f305224810108f459f2a782106473747270748018c8cb05cb025005cf165003fa0213cb6acb1f12cb3fc973fb000070810108d718fa00d33fc8542047810108f451f2a782106e6f746570748018c8cb05cb025006cf165004fa0214cb6a12cb1fcb3fc973fb0002006ed207fa00d4d422f90005c8ca0715cbffc9d077748018c8cb05cb0222cf165005fa0214cb6b12ccccc973fb00c84014810108f451f2a702020148130a0201200c0b0059bd242b6f6a2684080a06b90fa0218470d4080847a4937d29910ce6903e9ff9837812801b7810148987159f31840201200e0d0011b8c97ed44d0d70b1f8020158120f02012011100019af1df6a26840106b90eb858fc00019adce76a26840206b90eb85ffc0003db29dfb513420405035c87d010c00b23281f2fff274006040423d029be84c6002e6d001d0d3032171b0925f04e022d749c120925f04e002d31f218210706c7567bd22821064737472bdb0925f05e003fa403020fa4401c8ca07cbffc9d0ed44d0810140d721f404305c810108f40a6fa131b3925f07e005d33fc8258210706c7567ba923830e30d03821064737472ba925f06e30d1514008a5004810108f45930ed44d0810140d720c801cf16f400c9ed540172b08e23821064737472831eb17080185005cb055003cf1623fa0213cb6acb1fcb3fc98040fb00925f03e2007801fa00f40430f8276f2230500aa121bef2e0508210706c7567831eb17080185004cb0526cf1658fa0219f400cb6917cb1f5260cb3f20c98040fb0006";
        let source_cell = BagOfCells::parse_hex(state_init_hex)?.into_single_root()?;
        let state_init = source_cell.parser().load_tlb::<StateInit>()?;
        assert_eq!(state_init.split_depth, None);
        assert_eq!(state_init.tick_tock, None);
        assert!(state_init.code.is_some());
        assert!(state_init.data.is_some());
        assert_eq!(state_init.library, OptionRef::NONE);
        let serial_cell = CellBuilder::new().store_tlb(&state_init)?.build()?;
        assert_eq!(source_cell.deref(), &serial_cell);
        Ok(())
    }
}
