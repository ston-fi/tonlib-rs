use crate::cell::{ArcCell, CellBuilder, CellParser, TonCellError};
use crate::tlb_types::block::coins::CurrencyCollection;
use crate::tlb_types::primitives::either::Either;
use crate::tlb_types::primitives::reference::Ref;
use crate::tlb_types::tlb::{TLBPrefix, TLB};
use crate::TonHash;

// https://github.com/ton-blockchain/ton/blob/2a68c8610bf28b43b2019a479a70d0606c2a0aa1/crypto/block/block.tlb#L399
#[derive(Debug, PartialEq, Clone)]
pub enum OutList {
    Empty,
    Some(OutListSome),
}

#[derive(Debug, PartialEq, Clone)]
pub struct OutListSome {
    pub prev: Ref<ArcCell>, // it's recursive structure, prev == OutList
    pub action: OutAction,
}

// https://github.com/ton-blockchain/ton/blob/2a68c8610bf28b43b2019a479a70d0606c2a0aa1/crypto/block/block.tlb#L408
#[derive(Debug, PartialEq, Clone)]
pub enum OutAction {
    SendMsg(OutActionSendMsg),
    SetCode(OutActionSetCode),
    ReserveCurrency(OutActionReserveCurrency),
    ChangeLibrary(OutActionChangeLibrary),
}

#[derive(Debug, PartialEq, Clone)]
pub struct OutActionSendMsg {
    pub mode: u8,
    pub out_msg: ArcCell,
}

#[derive(Debug, PartialEq, Clone)]
pub struct OutActionSetCode {
    pub new_code: ArcCell,
}

#[derive(Debug, PartialEq, Clone)]
pub struct OutActionReserveCurrency {
    pub mode: u8,
    pub currency_collection: CurrencyCollection,
}

#[derive(Debug, PartialEq, Clone)]
pub struct OutActionChangeLibrary {
    pub mode: u8,
    pub library: Either<TonHash, Ref<ArcCell>>,
}

impl OutList {
    pub fn new(actions: &[OutAction]) -> Result<Self, TonCellError> {
        let val = if actions.is_empty() {
            Self::Empty
        } else {
            let action = &actions[0];
            let prev = OutList::new(&actions[1..])?;
            Self::Some(OutListSome {
                prev: Ref::new(prev.to_cell()?.to_arc()),
                action: action.clone(),
            })
        };
        Ok(val)
    }
}

impl TLB for OutList {
    fn read_definition(parser: &mut CellParser) -> Result<Self, TonCellError> {
        if parser.remaining_bits() == 0 {
            return Ok(Self::Empty);
        }
        Ok(Self::Some(OutListSome::read(parser)?))
    }

    fn write_definition(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        match self {
            Self::Empty => {}
            Self::Some(val) => val.write(dst)?,
        }
        Ok(())
    }
}

impl TLB for OutListSome {
    fn read_definition(parser: &mut CellParser) -> Result<Self, TonCellError> {
        Ok(Self {
            prev: TLB::read(parser)?,
            action: TLB::read(parser)?,
        })
    }

    fn write_definition(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        self.prev.write(dst)?;
        self.action.write(dst)?;
        Ok(())
    }
}

impl TLB for OutAction {
    fn read_definition(parser: &mut CellParser) -> Result<Self, TonCellError> {
        let prefix = TLBPrefix::new(32, parser.load_u32(32)? as u64);
        parser.seek(-32)?;
        let result = if prefix == OutActionSendMsg::PREFIX {
            Self::SendMsg(TLB::read(parser)?)
        } else if prefix == OutActionSetCode::PREFIX {
            Self::SetCode(TLB::read(parser)?)
        } else if prefix == OutActionReserveCurrency::PREFIX {
            Self::ReserveCurrency(TLB::read(parser)?)
        } else if prefix == OutActionChangeLibrary::PREFIX {
            Self::ChangeLibrary(TLB::read(parser)?)
        } else {
            let err_str = format!("Got unexpected OutAction prefix: {prefix:?}");
            return Err(TonCellError::InvalidCellData(err_str));
        };
        Ok(result)
    }

    fn write_definition(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        match self {
            Self::SendMsg(action) => action.write(dst)?,
            Self::SetCode(action) => action.write(dst)?,
            Self::ReserveCurrency(action) => action.write(dst)?,
            Self::ChangeLibrary(action) => action.write(dst)?,
        }
        Ok(())
    }
}

impl TLB for OutActionSendMsg {
    const PREFIX: TLBPrefix = TLBPrefix::new(32, 0x0ec3c86d);
    fn read_definition(parser: &mut CellParser) -> Result<Self, TonCellError> {
        Ok(Self {
            mode: parser.load_u8(8)?,
            out_msg: parser.next_reference()?,
        })
    }

    fn write_definition(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        dst.store_u8(8, self.mode)?;
        dst.store_reference(&self.out_msg)?;
        Ok(())
    }
}

impl TLB for OutActionSetCode {
    const PREFIX: TLBPrefix = TLBPrefix::new(32, 0xad4de08e);

    fn read_definition(parser: &mut CellParser) -> Result<Self, TonCellError> {
        Ok(Self {
            new_code: parser.next_reference()?,
        })
    }

    fn write_definition(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        dst.store_reference(&self.new_code)?;
        Ok(())
    }
}

impl TLB for OutActionReserveCurrency {
    const PREFIX: TLBPrefix = TLBPrefix::new(32, 0x36e6b809);

    fn read_definition(parser: &mut CellParser) -> Result<Self, TonCellError> {
        Ok(Self {
            mode: parser.load_u8(8)?,
            currency_collection: TLB::read(parser)?,
        })
    }

    fn write_definition(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        dst.store_u8(8, self.mode)?;
        self.currency_collection.write(dst)?;
        Ok(())
    }
}

impl TLB for OutActionChangeLibrary {
    const PREFIX: TLBPrefix = TLBPrefix::new(32, 0x26fa1dd4);

    fn read_definition(parser: &mut CellParser) -> Result<Self, TonCellError> {
        Ok(Self {
            mode: parser.load_u8(7)?,
            library: TLB::read(parser)?,
        })
    }

    fn write_definition(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        dst.store_u8(7, self.mode)?;
        self.library.write(dst)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::cell::EMPTY_CELL;
    use crate::tlb_types::block::out_action::{OutAction, OutActionSendMsg, OutList};
    use crate::tlb_types::primitives::reference::Ref;
    use crate::tlb_types::tlb::TLB;

    #[test]
    fn test_out_list_send_msg_action_manual_build() -> anyhow::Result<()> {
        let actions_cnt = 10;
        let mut actions = vec![];
        for i in 0..actions_cnt {
            let act = OutAction::SendMsg(OutActionSendMsg {
                mode: i as u8,
                out_msg: EMPTY_CELL.clone().to_arc(),
            });
            actions.push(act);
        }

        let out_list = OutList::new(&actions)?;
        let serial_cell = out_list.to_cell()?;
        let parsed_back = OutList::from_cell(&serial_cell)?;
        assert_eq!(out_list, parsed_back);
        Ok(())
    }

    #[test]
    fn test_out_list_send_msg_action_bc_data() -> anyhow::Result<()> {
        let opt_ref_out_list: Option<Ref<OutList>> = TLB::from_boc_hex("b5ee9c72010104010084000181bc04889cb28b36a3a00810e363a413763ec34860bf0fce552c5d36e37289fafd442f1983d740f92378919d969dd530aec92d258a0779fb371d4659f10ca1b3826001020a0ec3c86d0302030000006642007847b4630eb08d9f486fe846d5496878556dfd5a084f82a9a3fb01224e67c84c187a120000000000000000000000000000")?;
        let out_list = opt_ref_out_list.unwrap().0;

        // validate parsed data
        let OutList::Some(action) = &out_list else {
            panic!("OutListSome expected")
        };
        let child = OutList::from_cell(&action.prev.0)?;
        assert_eq!(child, OutList::Empty);

        // validate serialization
        let serial = out_list.to_cell()?;
        let parsed_back = OutList::from_cell(&serial)?;
        assert_eq!(out_list, parsed_back);

        Ok(())
    }
}
