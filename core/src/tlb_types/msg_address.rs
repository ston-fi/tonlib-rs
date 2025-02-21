use crate::cell::{CellBuilder, CellParser, TonCellError};
use crate::tlb_types::traits::TLBObject;
use crate::types::{TON_HASH_LEN, ZERO_HASH};
use crate::TonHash;
// https://github.com/ton-blockchain/ton/blob/59a8cf0ae5c3062d14ec4c89a04fee80b5fd05c1/crypto/block/block.tlb#L100

#[derive(Debug, Clone, PartialEq)]
pub enum MsgAddress {
    Ext(MsgAddressExt),
    Int(MsgAddressInt),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MsgAddressExt {
    pub address_len_bits: u16,
    pub address: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MsgAddressInt {
    pub anycast: Option<Anycast>,
    pub workchain: i32,
    pub address_len_bits: u32,
    pub address: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Anycast {
    pub depth: u8, // rewrite_pfx_len_bits
    pub rewrite_pfx: Vec<u8>,
}

impl MsgAddressExt {
    pub fn null() -> &'static Self {
        static VALUE: MsgAddressExt = MsgAddressExt {
            address_len_bits: 0,
            address: vec![],
        };
        &VALUE
    }
}

impl TLBObject for MsgAddress {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        match parser.load_bit()? {
            false => Ok(MsgAddress::Ext(parser.load_tlb()?)),
            true => Ok(MsgAddress::Int(parser.load_tlb()?)),
        }
    }

    fn write(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        match self {
            MsgAddress::Ext(addr) => {
                dst.store_bit(false)?;
                addr.write(dst)
            }
            MsgAddress::Int(addr) => {
                dst.store_bit(true)?;
                addr.write(dst)
            }
        }
    }
}

impl TLBObject for MsgAddressExt {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        if parser.load_bit()? {
            let len_bits = parser.load_u16(9)?;
            Ok(MsgAddressExt {
                address_len_bits: len_bits,
                address: parser.load_bits(len_bits as usize)?,
            })
        } else {
            Ok(Self::null().clone())
        }
    }

    fn write(&self, builder: &mut CellBuilder) -> Result<(), TonCellError> {
        if self.address_len_bits == 0 {
            builder.store_bit(false)?;
            return Ok(());
        }
        if self.address_len_bits > 512 {
            let err_str = format!(
                "MsgAddressExt len_bits is {}, max=512",
                self.address_len_bits
            );
            return Err(TonCellError::CellBuilderError(err_str));
        }
        builder.store_u32(9, self.address_len_bits as u32)?;
        builder.store_bits(self.address_len_bits as usize, &self.address)?;
        Ok(())
    }
}

impl TLBObject for MsgAddressInt {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        if parser.load_bit()? {
            let anycast = parser.load_tlb_optional::<Anycast>()?;
            let addr_len = parser.load_u32(9)?;
            let workchain = parser.load_i32(32)?;
            let address = parser.load_bytes(addr_len as usize)?;
            Ok(MsgAddressInt {
                anycast,
                workchain,
                address_len_bits: addr_len,
                address,
            })
        } else {
            let anycast = parser.load_tlb_optional::<Anycast>()?;
            let workchain = parser.load_i32(8)?;
            let address_len_bits = TON_HASH_LEN as u32 * 8;
            let address = parser.load_bits(address_len_bits as usize)?;
            Ok(MsgAddressInt {
                anycast,
                workchain,
                address_len_bits,
                address,
            })
        }
    }

    fn write(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        let is_zero_addr = TonHash::try_from(self.address.as_slice()).ok() == Some(ZERO_HASH);

        if self.anycast.is_none() && is_zero_addr && self.workchain == 0 {
            dst.store_u8(2, 0)?;
            return Ok(());
        }

        // for now, we support only tag == 0 for serialization
        dst.store_bit(false)?;
        dst.store_tlb_optional(self.anycast.as_ref())?;
        dst.store_i32(8, self.workchain)?;
        dst.store_bits(TON_HASH_LEN * 8, &self.address)?;
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

    fn write(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        dst.store_u8(5, self.depth)?;
        dst.store_bits(self.depth as usize, &self.rewrite_pfx)?;
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
        let msg_addr: MsgAddress = assert_ok!(parser.load_tlb::<MsgAddress>());

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
        assert_eq!(msg_addr, MsgAddress::Int(expected));

        let mut builder = CellBuilder::new();
        assert_ok!(msg_addr.write(&mut builder));
        let serialized_boc = BagOfCells::from_root(builder.build()?).serialize(false)?;
        assert_eq!(serialized_boc, boc);

        // TODO MsgAddress::Ext is not covered by tests
        Ok(())
    }
}
