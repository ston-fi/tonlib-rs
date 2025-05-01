use std::fmt::Debug;
use std::ops::Deref;

use base64::prelude::BASE64_STANDARD;
use base64::Engine;

use crate::cell::{BagOfCells, Cell, CellBuilder, CellParser, TonCellError};
use crate::TonHash;

pub trait TLB: Sized + Clone + Debug {
    const PREFIX: TLBPrefix = TLBPrefix::NULL;

    /// read-write definition
    /// https://docs.ton.org/v3/documentation/data-formats/tlb/tl-b-language#overview
    /// must be implemented by all TLB objects
    /// doesn't include prefix handling
    fn read_definition(parser: &mut CellParser) -> Result<Self, TonCellError>;
    fn write_definition(&self, dst: &mut CellBuilder) -> Result<(), TonCellError>;

    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        Self::verify_prefix(parser)?;
        Self::read_definition(parser)
    }

    fn write(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        Self::write_prefix(dst)?;
        self.write_definition(dst)
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
        let cell = BagOfCells::parse(boc)?.single_root()?;
        Self::from_cell(cell.deref())
    }

    fn from_boc_hex(boc_hex: &str) -> Result<Self, TonCellError> {
        let cell = BagOfCells::parse_hex(boc_hex)?.single_root()?;
        Self::from_cell(cell.deref())
    }

    fn from_boc_b64(boc_b64: &str) -> Result<Self, TonCellError> {
        let cell = BagOfCells::parse_base64(boc_b64)?.single_root()?;
        Self::from_cell(cell.deref())
    }

    /// Serialization
    ///
    fn to_cell(&self) -> Result<Cell, TonCellError> {
        let mut builder = CellBuilder::new();
        self.write(&mut builder)?;
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
        if Self::PREFIX == TLBPrefix::NULL {
            return Ok(());
        }

        if parser.remaining_bits() < Self::PREFIX.bit_len {
            return Err(TonCellError::tlb_prefix_error(
                Self::PREFIX,
                0,
                parser.remaining_bits(),
            ));
        }

        // we handle cell_underflow above - all other errors can be rethrown
        let actual_prefix: u64 = parser.load_number(Self::PREFIX.bit_len)?;

        if actual_prefix != Self::PREFIX.value {
            parser.seek(-(Self::PREFIX.bit_len as i64))?; // revert reader position
            return Err(TonCellError::tlb_prefix_error(
                Self::PREFIX,
                actual_prefix,
                parser.remaining_bits(),
            ));
        }
        Ok(())
    }

    fn write_prefix(builder: &mut CellBuilder) -> Result<(), TonCellError> {
        if Self::PREFIX != TLBPrefix::NULL {
            builder.store_number(Self::PREFIX.bit_len, &Self::PREFIX.value)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TLBPrefix {
    pub bit_len: usize,
    pub value: u64,
}

impl TLBPrefix {
    pub const NULL: TLBPrefix = TLBPrefix::new(0, 0);
    pub const fn new(bit_len: usize, value: u64) -> Self {
        TLBPrefix { bit_len, value }
    }
}
