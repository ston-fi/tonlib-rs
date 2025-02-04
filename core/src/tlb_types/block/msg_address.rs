use crate::cell::{CellBuilder, CellParser, TonCellError};
use crate::tlb_types::traits::TLBObject;
use crate::types::TON_HASH_LEN;

// https://github.com/ton-blockchain/ton/blob/59a8cf0ae5c3062d14ec4c89a04fee80b5fd05c1/crypto/block/block.tlb#L100
#[derive(Debug, Clone, PartialEq)]
pub enum MsgAddress {
    Ext(MsgAddressExt), // Is not covered by tests
    Int(MsgAddressInt),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MsgAddressExt {
    pub address_len_bits: u16,
    pub address: Vec<u8>,
}

// Support serialization only to MsgAddrVar format
#[derive(Debug, Clone, PartialEq)]
pub struct MsgAddressInt {
    pub anycast: Option<Anycast>,
    pub workchain: i32,
    pub address: Vec<u8>,
    pub address_len_bits: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Anycast {
    pub depth: u8, // rewrite_pfx_len_bits
    pub rewrite_pfx: Vec<u8>,
}

impl MsgAddressExt {
    pub fn null() -> &'static Self {
        static VALUE: MsgAddressExt = MsgAddressExt {
            address: vec![],
            address_len_bits: 0,
        };
        &VALUE
    }
}

impl TLBObject for MsgAddress {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        let tag = parser.load_bit()?;
        parser.advance(-1)?;
        match tag {
            false => Ok(MsgAddress::Ext(TLBObject::read(parser)?)),
            true => Ok(MsgAddress::Int(TLBObject::read(parser)?)),
        }
    }

    fn write_to(&self, builder: &mut CellBuilder) -> Result<(), TonCellError> {
        match self {
            MsgAddress::Ext(addr) => {
                addr.write_to(builder)?;
            }
            MsgAddress::Int(addr) => {
                addr.write_to(builder)?;
            }
        };
        Ok(())
    }
}

impl TLBObject for MsgAddressExt {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        if parser.load_bit()? {
            return Err(TonCellError::CellParserError(
                "MsgAddressExt: unexpected tag bit".to_string(),
            ));
        }
        if parser.load_bit()? {
            let len_bits = parser.load_u16(9)?;
            Ok(MsgAddressExt {
                address: parser.load_bits(len_bits as usize)?,
                address_len_bits: len_bits,
            })
        } else {
            Ok(Self::null().clone())
        }
    }

    fn write_to(&self, builder: &mut CellBuilder) -> Result<(), TonCellError> {
        builder.store_bit(false)?;
        if self.address_len_bits == 0 {
            builder.store_bit(false)?;
            return Ok(());
        }
        builder.store_bit(true)?;
        if self.address_len_bits > 512 {
            let err_str = format!(
                "MsgAddressExt len_bits is {}, max=512",
                self.address_len_bits
            );
            return Err(TonCellError::CellBuilderError(err_str));
        }
        builder.store_u16(9, self.address_len_bits)?;
        builder.store_bits(self.address_len_bits as usize, &self.address)?;
        Ok(())
    }
}

impl TLBObject for MsgAddressInt {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        if !parser.load_bit()? {
            return Err(TonCellError::CellParserError(
                "MsgAddressInt: unexpected tag bit".to_string(),
            ));
        }
        let value = match parser.load_bit()? {
            false => {
                // MsgAddrIntStd
                let anycast = TLBObject::read(parser)?;
                let workchain = parser.load_i8(8)? as i32;
                let address_len_bits = TON_HASH_LEN as u16 * 8;
                let address = parser.load_bits(address_len_bits as usize)?;
                MsgAddressInt {
                    anycast,
                    workchain,
                    address,
                    address_len_bits,
                }
            }
            true => {
                // MsgAddrIntVar
                let anycast = TLBObject::read(parser)?;
                let address_len_bits = parser.load_u16(9)?;
                let workchain = parser.load_i32(32)?;
                let address = parser.load_bits(address_len_bits as usize)?;
                MsgAddressInt {
                    anycast,
                    workchain,
                    address,
                    address_len_bits,
                }
            }
        };
        Ok(value)
    }

    fn write_to(&self, builder: &mut CellBuilder) -> Result<(), TonCellError> {
        builder.store_bit(true)?; // tag
        builder.store_bit(true)?; // MsgAddrVar format
        self.anycast.write_to(builder)?;
        builder.store_u16(9, self.address_len_bits)?;
        builder.store_i32(32, self.workchain)?;
        builder.store_bits(self.address_len_bits as usize, &self.address)?;
        Ok(())
    }
}

// https://github.com/ton-blockchain/ton/blob/59a8cf0ae5c3062d14ec4c89a04fee80b5fd05c1/crypto/block/block.tlb#L104
impl TLBObject for Anycast {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        let depth = parser.load_u8(5)?;
        let rewrite_pfx = parser.load_bits(depth as usize)?;
        Ok(Anycast { depth, rewrite_pfx })
    }

    fn write_to(&self, builder: &mut CellBuilder) -> Result<(), TonCellError> {
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
    use crate::tlb_types::traits::TLBObject;

    #[test]
    fn test_read_write_msg_address() -> anyhow::Result<()> {
        // Anyhow read/write is covered under the hood
        let boc = hex::decode("b5ee9c7201010101002800004bbe031053100134ea6c68e2f2cee9619bdd2732493f3a1361eccd7c5267a9eb3c5dcebc533bb6")?;
        let cell = BagOfCells::parse(&boc)?.into_single_root()?;
        let mut parser = cell.parser();
        let parsed = assert_ok!(MsgAddress::read(&mut parser));

        let expected = MsgAddressInt {
            anycast: Some(Anycast {
                depth: 30,
                rewrite_pfx: vec![3, 16, 83, 16],
            }),
            workchain: 0,
            address_len_bits: 256,
            address: vec![
                77, 58, 155, 26, 56, 188, 179, 186, 88, 102, 247, 73, 204, 146, 79, 206, 132, 216,
                123, 51, 95, 20, 153, 234, 122, 207, 23, 115, 175, 20, 206, 237,
            ],
        };
        assert_eq!(parsed, MsgAddress::Int(expected.clone()));

        let serial_cell = parsed.to_cell()?;
        let mut serial_parser = serial_cell.parser();
        let parsed_back = assert_ok!(MsgAddress::read(&mut serial_parser));
        assert_eq!(parsed_back, MsgAddress::Int(expected.clone()));
        Ok(())
    }

    #[test]
    fn test_read_msg_address_int_i8_workchain() -> anyhow::Result<()> {
        let cell = BagOfCells::parse_hex("b5ee9c720101010100240000439fe00000000000000000000000000000000000000000000000000000000000000010")?.into_single_root()?;
        for s in cell.data() {
            print!("{:b}", s);
        }
        println!();
        let mut parser = cell.parser();
        let parsed = assert_ok!(MsgAddress::read(&mut parser));

        let expected = MsgAddressInt {
            anycast: None,
            workchain: -1,
            address_len_bits: 256,
            address: vec![0; 32],
        };
        assert_eq!(parsed, MsgAddress::Int(expected));

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
}
