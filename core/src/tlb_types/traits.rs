use std::ops::Deref;
use std::sync::Arc;

use base64::prelude::BASE64_STANDARD;
use base64::Engine;

use crate::cell::{ArcCell, BagOfCells, Cell, CellBuilder, CellParser, TonCellError};

pub trait TLBObject: Sized {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError>;

    fn write_to(&self, dst: &mut CellBuilder) -> Result<(), TonCellError>;

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

    fn to_boc(&self) -> Result<Vec<u8>, TonCellError> {
        BagOfCells::from_root(self.to_cell()?).serialize(false)
    }

    fn to_boc_hex(&self) -> Result<String, TonCellError> {
        Ok(hex::encode(self.to_boc()?))
    }

    fn to_boc_b64(&self) -> Result<String, TonCellError> {
        Ok(BASE64_STANDARD.encode(self.to_boc()?))
    }
}

/// Dependencies implementation
///
impl TLBObject for Cell {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        parser.load_remaining()
    }

    fn write_to(&self, builder: &mut CellBuilder) -> Result<(), TonCellError> {
        builder.store_cell(self)?;
        Ok(())
    }
}

impl TLBObject for ArcCell {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        parser.load_remaining().map(Arc::new)
    }

    fn write_to(&self, builder: &mut CellBuilder) -> Result<(), TonCellError> {
        builder.store_cell(self)?;
        Ok(())
    }
}
