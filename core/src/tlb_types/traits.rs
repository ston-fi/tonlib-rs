use std::any::type_name;
use std::ops::Deref;

use base64::prelude::BASE64_STANDARD;
use base64::Engine;

use crate::cell::{BagOfCells, Cell, CellBuilder, CellParser, TonCellError};
use crate::TonHash;

pub trait TLBObject: Sized {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError>;

    fn write_to(&self, dst: &mut CellBuilder) -> Result<(), TonCellError>;

    fn prefix() -> &'static TLBPrefix {
        &TLBPrefix::NULL
    }

    /// Utilities
    ///
    fn cell_hash(&self) -> Result<TonHash, TonCellError> {
        Ok(self.to_cell()?.cell_hash())
    }

    /// Parsing
    ///
    fn from_cell(cell: &Cell) -> Result<Self, TonCellError> {
        Self::read(&mut cell.parser())
    }

    fn from_boc(boc: &[u8]) -> Result<Self, TonCellError> {
        let cell = BagOfCells::parse(boc)?.into_single_root()?;
        Self::from_cell(cell.deref())
    }

    fn from_boc_hex(boc_hex: &str) -> Result<Self, TonCellError> {
        let cell = BagOfCells::parse_hex(boc_hex)?.into_single_root()?;
        Self::from_cell(cell.deref())
    }

    fn from_boc_b64(boc_b64: &str) -> Result<Self, TonCellError> {
        let cell = BagOfCells::parse_base64(boc_b64)?.into_single_root()?;
        Self::from_cell(cell.deref())
    }

    /// Serialization
    ///
    fn to_cell(&self) -> Result<Cell, TonCellError> {
        let mut builder = CellBuilder::new();
        self.write_to(&mut builder)?;
        builder.build()
    }

    fn to_boc(&self, add_crc32: bool) -> Result<Vec<u8>, TonCellError> {
        BagOfCells::from_root(self.to_cell()?).serialize(add_crc32)
    }

    fn to_boc_hex(&self, add_crc32: bool) -> Result<String, TonCellError> {
        Ok(hex::encode(self.to_boc(add_crc32)?))
    }

    fn to_boc_b64(&self, add_crc32: bool) -> Result<String, TonCellError> {
        Ok(BASE64_STANDARD.encode(self.to_boc(add_crc32)?))
    }

    /// Helpers - for internal use
    ///
    fn verify_prefix(parser: &mut CellParser) -> Result<(), TonCellError> {
        let prefix = Self::prefix();
        if prefix == &TLBPrefix::NULL {
            return Ok(());
        }
        let value = parser.load_u64(prefix.bit_len as usize)?;
        if value != prefix.value {
            let err_str = format!(
                "[{}] Invalid prefix: {value:X} (expected: {:X})",
                type_name::<Self>(),
                prefix.value
            );
            return Err(TonCellError::InvalidCellData(err_str));
        }
        Ok(())
    }

    fn write_prefix(builder: &mut CellBuilder) -> Result<(), TonCellError> {
        let prefix = Self::prefix();
        if prefix != &TLBPrefix::NULL {
            builder.store_u64(prefix.bit_len as usize, prefix.value)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TLBPrefix {
    pub bit_len: u8,
    pub value: u64,
}

impl TLBPrefix {
    pub const NULL: TLBPrefix = TLBPrefix {
        bit_len: 0,
        value: 0,
    };
    pub const fn new(bit_len: u8, value: u64) -> Self {
        Self { bit_len, value }
    }
}
