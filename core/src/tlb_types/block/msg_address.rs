use crate::cell::{CellBuilder, CellParser, TonCellError};
use crate::tlb_types::tlb::{TLBPrefix, TLB};

// https://github.com/ton-blockchain/ton/blob/59a8cf0ae5c3062d14ec4c89a04fee80b5fd05c1/crypto/block/block.tlb#L100
#[derive(Debug, Clone, PartialEq)]
pub enum MsgAddress {
    Int(MsgAddressInt),
    Ext(MsgAddressExt),
}

#[derive(Debug, Clone, PartialEq)]
pub enum MsgAddressExt {
    None(MsgAddrNone),
    Extern(MsgAddrExt),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MsgAddrNone {}

#[derive(Debug, Clone, PartialEq)]
pub struct MsgAddrExt {
    pub address_bit_len: u16,
    pub address: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MsgAddressInt {
    Std(MsgAddrIntStd),
    Var(MsgAddrIntVar),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MsgAddrIntStd {
    pub anycast: Option<Anycast>,
    pub workchain: i32,
    pub address: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MsgAddrIntVar {
    pub anycast: Option<Anycast>,
    pub workchain: i32,
    pub address_bit_len: u16,
    pub address: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Anycast {
    pub depth: u8, // rewrite_pfx_len_bits
    pub rewrite_pfx: Vec<u8>,
}

impl MsgAddress {
    pub const NONE: MsgAddress = MsgAddress::Ext(MsgAddressExt::None(MsgAddrNone {}));
}

impl From<MsgAddrNone> for MsgAddress {
    fn from(value: MsgAddrNone) -> Self {
        MsgAddress::Ext(MsgAddressExt::None(value))
    }
}

impl From<MsgAddrExt> for MsgAddress {
    fn from(value: MsgAddrExt) -> Self {
        MsgAddress::Ext(MsgAddressExt::Extern(value))
    }
}

impl From<MsgAddressInt> for MsgAddress {
    fn from(value: MsgAddressInt) -> Self {
        MsgAddress::Int(value)
    }
}

impl From<MsgAddrIntStd> for MsgAddress {
    fn from(value: MsgAddrIntStd) -> Self {
        MsgAddress::Int(MsgAddressInt::Std(value))
    }
}

impl From<MsgAddrIntVar> for MsgAddress {
    fn from(value: MsgAddrIntVar) -> Self {
        MsgAddress::Int(MsgAddressInt::Var(value))
    }
}

impl TLB for MsgAddress {
    fn read_definition(parser: &mut CellParser) -> Result<Self, TonCellError> {
        let tag = parser.load_u8(2)?;
        parser.seek(-2)?;
        match tag {
            0b00 => Ok(MsgAddress::Ext(MsgAddressExt::None(TLB::read(parser)?))),
            0b01 => Ok(MsgAddress::Ext(MsgAddressExt::Extern(TLB::read(parser)?))),
            0b10 => Ok(MsgAddress::Int(MsgAddressInt::Std(TLB::read(parser)?))),
            0b11 => Ok(MsgAddress::Int(MsgAddressInt::Var(TLB::read(parser)?))),
            _ => Err(TonCellError::CellParserError(format!(
                "MsgAddress: unexpected tag {tag}"
            ))),
        }
    }

    fn write_definition(&self, builder: &mut CellBuilder) -> Result<(), TonCellError> {
        match self {
            MsgAddress::Int(MsgAddressInt::Std(addr)) => addr.write(builder)?,
            MsgAddress::Int(MsgAddressInt::Var(addr)) => addr.write(builder)?,
            MsgAddress::Ext(MsgAddressExt::None(addr)) => addr.write(builder)?,
            MsgAddress::Ext(MsgAddressExt::Extern(addr)) => addr.write(builder)?,
        };
        Ok(())
    }
}

impl TLB for MsgAddressInt {
    fn read_definition(parser: &mut CellParser) -> Result<Self, TonCellError> {
        let tag = parser.load_u8(2)?;
        parser.seek(-2)?;
        match tag {
            0b10 => Ok(MsgAddressInt::Std(TLB::read(parser)?)),
            0b11 => Ok(MsgAddressInt::Var(TLB::read(parser)?)),
            _ => Err(TonCellError::CellParserError(format!(
                "MsgAddress: unexpected tag {tag}"
            ))),
        }
    }

    fn write_definition(&self, builder: &mut CellBuilder) -> Result<(), TonCellError> {
        match self {
            MsgAddressInt::Std(addr) => addr.write(builder)?,
            MsgAddressInt::Var(addr) => addr.write(builder)?,
        };
        Ok(())
    }
}

impl TLB for MsgAddrNone {
    const PREFIX: TLBPrefix = TLBPrefix::new(2, 0b00);

    fn read_definition(_: &mut CellParser) -> Result<Self, TonCellError> {
        Ok(MsgAddrNone {})
    }

    fn write_definition(&self, _: &mut CellBuilder) -> Result<(), TonCellError> {
        Ok(())
    }
}

impl TLB for MsgAddrExt {
    const PREFIX: TLBPrefix = TLBPrefix::new(2, 0b01);

    fn read_definition(parser: &mut CellParser) -> Result<Self, TonCellError> {
        let bit_len = parser.load_u16(9)?;
        Ok(MsgAddrExt {
            address_bit_len: bit_len,
            address: parser.load_bits(bit_len as usize)?,
        })
    }

    fn write_definition(&self, builder: &mut CellBuilder) -> Result<(), TonCellError> {
        if self.address_bit_len > 512 {
            let err_str = format!(
                "MsgAddressExt len_bits is {}, max=512 (9 bits)",
                self.address_bit_len
            );
            return Err(TonCellError::CellBuilderError(err_str));
        }
        builder.store_u16(9, self.address_bit_len)?;
        builder.store_bits(self.address_bit_len as usize, &self.address)?;
        Ok(())
    }
}

impl TLB for MsgAddressExt {
    fn read_definition(parser: &mut CellParser) -> Result<Self, TonCellError> {
        let tag = parser.load_u8(2)?;
        parser.seek(-2)?;
        match tag {
            0b00 => Ok(MsgAddressExt::None(TLB::read(parser)?)),
            0b01 => Ok(MsgAddressExt::Extern(TLB::read(parser)?)),
            _ => Err(TonCellError::CellParserError(format!(
                "MsgAddressExt: unexpected tag {tag}"
            ))),
        }
    }

    fn write_definition(&self, builder: &mut CellBuilder) -> Result<(), TonCellError> {
        match self {
            MsgAddressExt::None(addr) => addr.write(builder)?,
            MsgAddressExt::Extern(addr) => addr.write(builder)?,
        };
        Ok(())
    }
}

impl TLB for MsgAddrIntStd {
    const PREFIX: TLBPrefix = TLBPrefix::new(2, 0b10);

    fn read_definition(parser: &mut CellParser) -> Result<Self, TonCellError> {
        Ok(MsgAddrIntStd {
            anycast: TLB::read(parser)?,
            workchain: parser.load_i8(8)? as i32,
            address: parser.load_bits(256)?,
        })
    }

    fn write_definition(&self, builder: &mut CellBuilder) -> Result<(), TonCellError> {
        self.anycast.write(builder)?;
        builder.store_i8(8, self.workchain as i8)?;
        builder.store_bits(256, &self.address)?;
        Ok(())
    }
}

impl TLB for MsgAddrIntVar {
    const PREFIX: TLBPrefix = TLBPrefix::new(2, 0b11);

    fn read_definition(parser: &mut CellParser) -> Result<Self, TonCellError> {
        let anycast = TLB::read(parser)?;
        let address_bit_len = parser.load_u16(9)?;
        let workchain = parser.load_i32(32)?;
        let address = parser.load_bits(address_bit_len as usize)?;
        Ok(MsgAddrIntVar {
            anycast,
            workchain,
            address_bit_len,
            address,
        })
    }

    fn write_definition(&self, builder: &mut CellBuilder) -> Result<(), TonCellError> {
        self.anycast.write(builder)?;
        builder.store_u16(9, self.address_bit_len)?;
        builder.store_i32(32, self.workchain)?;
        builder.store_bits(self.address_bit_len as usize, &self.address)?;
        Ok(())
    }
}

// https://github.com/ton-blockchain/ton/blob/59a8cf0ae5c3062d14ec4c89a04fee80b5fd05c1/crypto/block/block.tlb#L104
impl TLB for Anycast {
    fn read_definition(parser: &mut CellParser) -> Result<Self, TonCellError> {
        let depth = parser.load_u8(5)?;
        let rewrite_pfx = parser.load_bits(depth as usize)?;
        Ok(Anycast { depth, rewrite_pfx })
    }

    fn write_definition(&self, builder: &mut CellBuilder) -> Result<(), TonCellError> {
        builder
            .store_u8(5, self.depth)?
            .store_bits(self.depth as usize, &self.rewrite_pfx)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tokio_test::assert_ok;

    use super::*;
    use crate::cell::BagOfCells;
    use crate::tlb_types::tlb::TLB;

    #[test]
    fn test_read_write_msg_address() -> anyhow::Result<()> {
        // Anyhow read/write is covered under the hood
        let boc = hex::decode("b5ee9c7201010101002800004bbe031053100134ea6c68e2f2cee9619bdd2732493f3a1361eccd7c5267a9eb3c5dcebc533bb6")?;
        let cell = BagOfCells::parse(&boc)?.single_root()?;
        let mut parser = cell.parser();
        let parsed = assert_ok!(MsgAddress::read(&mut parser));

        let expected = MsgAddrIntStd {
            anycast: Some(Anycast {
                depth: 30,
                rewrite_pfx: vec![3, 16, 83, 16],
            }),
            workchain: 0,
            address: vec![
                77, 58, 155, 26, 56, 188, 179, 186, 88, 102, 247, 73, 204, 146, 79, 206, 132, 216,
                123, 51, 95, 20, 153, 234, 122, 207, 23, 115, 175, 20, 206, 237,
            ],
        };
        assert_eq!(
            parsed,
            MsgAddress::Int(MsgAddressInt::Std(expected.clone()))
        );

        let serial_cell = parsed.to_cell()?;
        let mut serial_parser = serial_cell.parser();
        let parsed_back = assert_ok!(MsgAddress::read(&mut serial_parser));
        assert_eq!(
            parsed_back,
            MsgAddress::Int(MsgAddressInt::Std(expected.clone()))
        );
        Ok(())
    }

    #[test]
    fn test_read_msg_address_int_i8_workchain() -> anyhow::Result<()> {
        let cell = BagOfCells::parse_hex("b5ee9c720101010100240000439fe00000000000000000000000000000000000000000000000000000000000000010")?.single_root()?;
        for s in cell.data() {
            print!("{:b}", s);
        }
        println!();
        let mut parser = cell.parser();
        let parsed = assert_ok!(MsgAddress::read(&mut parser));

        let expected = MsgAddrIntStd {
            anycast: None,
            workchain: -1,
            address: vec![0; 32],
        };
        assert_eq!(
            parsed,
            MsgAddress::Int(MsgAddressInt::Std(expected.clone()))
        );

        // don't support same layout, so check deserialized data again
        let serial_cell = parsed.to_cell()?;
        for s in serial_cell.data() {
            print!("{:b}", s);
        }
        println!();
        let mut parser = serial_cell.parser();
        let parsed_back = assert_ok!(MsgAddress::read(&mut parser));
        assert_eq!(parsed, parsed_back);
        Ok(())
    }

    #[test]
    fn test_read_msg_address_int() -> anyhow::Result<()> {
        let cell = BagOfCells::parse_hex("b5ee9c720101010100240000439fe00000000000000000000000000000000000000000000000000000000000000010")?.single_root()?;
        for s in cell.data() {
            print!("{:b}", s);
        }
        println!();
        let mut parser = cell.parser();
        let parsed = assert_ok!(MsgAddressInt::read(&mut parser));

        let expected = MsgAddrIntStd {
            anycast: None,
            workchain: -1,
            address: vec![0; 32],
        };
        assert_eq!(parsed, MsgAddressInt::Std(expected));

        // don't support same layout, so check deserialized data again
        let serial_cell = parsed.to_cell()?;
        for s in serial_cell.data() {
            print!("{:b}", s);
        }
        println!();
        let mut parser = serial_cell.parser();
        let parsed_back = assert_ok!(MsgAddressInt::read(&mut parser));
        assert_eq!(parsed, parsed_back);
        Ok(())
    }
}
