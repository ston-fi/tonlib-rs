use crate::cell::{ArcCell, CellBuilder, CellParser, TonCellError};
use crate::tlb_types::block::coins::{CurrencyCollection, Grams};
use crate::tlb_types::block::msg_address::MsgAddress;
use crate::tlb_types::block::state_init::StateInit;
use crate::tlb_types::primitives::either::EitherRef;
use crate::tlb_types::traits::{TLBObject, TLBPrefix};

// https://github.com/ton-blockchain/ton/blob/050a984163a53df16fb03f66cc445c34bfed48ed/crypto/block/block.tlb#L157
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub info: CommonMsgInfo,
    pub init: Option<EitherRef<StateInit>>,
    pub body: EitherRef<ArcCell>,
}

// https://github.com/ton-blockchain/ton/blob/050a984163a53df16fb03f66cc445c34bfed48ed/crypto/block/block.tlb#L155
#[derive(Debug, Clone, PartialEq)]
pub enum CommonMsgInfo {
    Int(IntMsgInfo),
    ExtIn(ExtInMsgInfo),
    ExtOut(ExtOutMsgInfo), // is not tested
}

#[derive(Clone, Debug, PartialEq)]
pub struct IntMsgInfo {
    pub ihr_disabled: bool,
    pub bounce: bool,
    pub bounced: bool,
    pub src: MsgAddress,
    pub dest: MsgAddress,
    pub value: CurrencyCollection,
    pub ihr_fee: Grams,
    pub fwd_fee: Grams,
    pub created_lt: u64,
    pub created_at: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExtInMsgInfo {
    pub src: MsgAddress,
    pub dest: MsgAddress,
    pub import_fee: Grams,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExtOutMsgInfo {
    pub src: MsgAddress,
    pub dest: MsgAddress,
    pub created_lt: u64,
    pub created_at: u32,
}

impl Message {
    pub fn new(info: CommonMsgInfo, body: ArcCell) -> Self {
        Self {
            info,
            init: None,
            body: EitherRef::new(body),
        }
    }

    pub fn with_state_init(&mut self, init: StateInit) -> &mut Self {
        self.init = Some(EitherRef::new(init));
        self
    }
}

impl TLBObject for Message {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        let value = Self {
            info: TLBObject::read(parser)?,
            init: TLBObject::read(parser)?,
            body: TLBObject::read(parser)?,
        };
        Ok(value)
    }

    fn write_to(&self, builder: &mut CellBuilder) -> Result<(), TonCellError> {
        self.info.write_to(builder)?;
        self.init.write_to(builder)?;
        self.body.write_to(builder)?;
        Ok(())
    }
}

impl TLBObject for CommonMsgInfo {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        let first_tag_bit = parser.load_bit()?;
        if !first_tag_bit {
            parser.seek(-1)?;
            return Ok(Self::Int(TLBObject::read(parser)?));
        };
        let second_tag_bit = parser.load_bit()?;
        parser.seek(-2)?;
        match second_tag_bit {
            false => Ok(Self::ExtIn(TLBObject::read(parser)?)),
            true => Ok(Self::ExtOut(TLBObject::read(parser)?)),
        }
    }

    fn write_to(&self, builder: &mut CellBuilder) -> Result<(), TonCellError> {
        match self {
            Self::Int(info) => info.write_to(builder)?,
            Self::ExtIn(info) => info.write_to(builder)?,
            Self::ExtOut(info) => info.write_to(builder)?,
        }
        Ok(())
    }
}

impl TLBObject for IntMsgInfo {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        Self::verify_prefix(parser)?;
        let value = Self {
            ihr_disabled: parser.load_bit()?,
            bounce: parser.load_bit()?,
            bounced: parser.load_bit()?,
            src: TLBObject::read(parser)?,
            dest: TLBObject::read(parser)?,
            value: CurrencyCollection::read(parser)?,
            ihr_fee: Grams::read(parser)?,
            fwd_fee: Grams::read(parser)?,
            created_lt: parser.load_u64(64)?,
            created_at: parser.load_u32(32)?,
        };
        Ok(value)
    }

    fn write_to(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        Self::write_prefix(dst)?;
        dst.store_bit(self.ihr_disabled)?;
        dst.store_bit(self.bounce)?;
        dst.store_bit(self.bounced)?;
        self.src.write_to(dst)?;
        self.dest.write_to(dst)?;
        self.value.write_to(dst)?;
        self.ihr_fee.write_to(dst)?;
        self.fwd_fee.write_to(dst)?;
        dst.store_u64(64, self.created_lt)?;
        dst.store_u32(32, self.created_at)?;
        Ok(())
    }

    fn prefix() -> Option<&'static TLBPrefix> {
        const PREFIX: TLBPrefix = TLBPrefix::new(1, 0b0);
        Some(&PREFIX)
    }
}

impl TLBObject for ExtInMsgInfo {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        Self::verify_prefix(parser)?;
        let value = Self {
            src: TLBObject::read(parser)?,
            dest: TLBObject::read(parser)?,
            import_fee: TLBObject::read(parser)?,
        };
        Ok(value)
    }

    fn write_to(&self, builder: &mut CellBuilder) -> Result<(), TonCellError> {
        Self::write_prefix(builder)?;
        self.src.write_to(builder)?;
        self.dest.write_to(builder)?;
        self.import_fee.write_to(builder)?;
        Ok(())
    }

    fn prefix() -> Option<&'static TLBPrefix> {
        const PREFIX: TLBPrefix = TLBPrefix::new(2, 0b10);
        Some(&PREFIX)
    }
}

impl TLBObject for ExtOutMsgInfo {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        Self::verify_prefix(parser)?;
        let value = Self {
            src: TLBObject::read(parser)?,
            dest: TLBObject::read(parser)?,
            created_lt: parser.load_u64(64)?,
            created_at: parser.load_u32(32)?,
        };
        Ok(value)
    }

    fn write_to(&self, builder: &mut CellBuilder) -> Result<(), TonCellError> {
        Self::write_prefix(builder)?;
        self.src.write_to(builder)?;
        self.dest.write_to(builder)?;
        builder.store_u64(64, self.created_lt)?;
        builder.store_u32(32, self.created_at)?;
        Ok(())
    }

    fn prefix() -> Option<&'static TLBPrefix> {
        const PREFIX: TLBPrefix = TLBPrefix::new(2, 0b11);
        Some(&PREFIX)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use tokio_test::assert_ok;

    use crate::cell::BagOfCells;
    use crate::tlb_types::block::coins::{CurrencyCollection, Grams};
    use crate::tlb_types::block::message::{CommonMsgInfo, ExtInMsgInfo, Message};
    use crate::tlb_types::traits::TLBObject;
    use crate::TonAddress;

    #[test]
    fn test_common_msg_info_int() -> anyhow::Result<()> {
        let msg_cell = BagOfCells::parse_hex("b5ee9c720101010100580000ab69fe00000000000000000000000000000000000000000000000000000000000000013fccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccd3050ec744000000617bc90dda80cf41ab8e40")?.into_single_root()?;
        let mut parser = msg_cell.parser();
        let parsed_msg = assert_ok!(Message::read(&mut parser));
        assert!(parsed_msg.init.is_none());
        assert_eq!(parsed_msg.body.value.bit_len(), 0); // quite useless assert, but let it be here

        let info = match parsed_msg.info.clone() {
            CommonMsgInfo::Int(info) => info,
            _ => panic!("Expected CommonMsgInfo::Int"),
        };
        assert!(info.ihr_disabled);
        assert!(info.bounce);
        assert!(!info.bounced);

        let expected_src =
            TonAddress::from_str("Ef8AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADAU")?;
        let expected_dest =
            TonAddress::from_str("Ef8zMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzM0vF")?;
        assert_eq!(TonAddress::try_from(info.src)?, expected_src);
        assert_eq!(
            TonAddress::try_from(info.dest)?.to_string(),
            expected_dest.to_string()
        );
        assert_eq!(info.value, CurrencyCollection::new(3242439121u32.into()));
        assert_eq!(info.ihr_fee, Grams::new(0u32.into()));
        assert_eq!(info.fwd_fee, Grams::new(0u32.into()));
        assert_eq!(info.created_lt, 53592141000000);
        assert_eq!(info.created_at, 1738593735u32);

        let serialized = parsed_msg.to_cell()?;
        let parsed_back = assert_ok!(Message::from_cell(&serialized));
        assert_eq!(parsed_back, parsed_msg);
        Ok(())
    }

    #[test]
    fn test_ext_in_msg_info() -> anyhow::Result<()> {
        let ext_in_msg_info = ExtInMsgInfo::from_boc_hex("b5ee9c7201010101002500004588010319f77e4d761f956e78f9c9fd45f1e914b7ffab9b5c1ea514858979c1560dee10")?;
        let expected_dst =
            TonAddress::from_str("EQCBjPu_JrsPyrc8fOT-ovj0ilv_1c2uD1KKQsS84KsG90PM")?;
        let dst = TonAddress::try_from(ext_in_msg_info.dest.clone())?;
        assert_eq!(dst.to_string(), expected_dst.to_string());
        assert_eq!(ext_in_msg_info.import_fee.clone(), Grams::new(0u32.into()));

        let cell = ext_in_msg_info.to_cell()?;
        let parsed = ExtInMsgInfo::from_cell(&cell)?;
        assert_eq!(parsed, ext_in_msg_info);
        Ok(())
    }
}
