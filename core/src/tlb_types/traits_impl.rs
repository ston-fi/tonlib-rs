use std::sync::Arc;

use crate::cell::{ArcCell, BagOfCells, Cell, CellBuilder, CellParser, TonCellError};
use crate::tlb_types::traits::TLBObject;
use crate::types::TON_HASH_LEN;
use crate::TonHash;

impl TLBObject for Cell {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        if parser.cell.bit_len() == parser.remaining_bits()
            && parser.remaining_refs() == parser.cell.references().len()
        {
            Ok(parser.cell.clone())
        } else {
            // TODO not clear how to handle exotics with current implementation
            parser.load_remaining()
        }
    }

    fn write_to(&self, builder: &mut CellBuilder) -> Result<(), TonCellError> {
        builder.set_cell_is_exotic(self.is_exotic());
        builder.store_cell(self)?;
        Ok(())
    }

    fn from_boc(boc: &[u8]) -> Result<Self, TonCellError> {
        let arc_cell = BagOfCells::parse(boc)?.single_root()?;
        let cell = match Arc::try_unwrap(arc_cell) {
            Ok(cell) => cell,
            Err(arc_cell) => {
                // we just constructed the cell, so this should never happen
                panic!("Failed to unwrap Arc: {arc_cell:?}")
            }
        };
        Ok(cell)
    }
}

impl TLBObject for ArcCell {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        Cell::read(parser).map(Arc::new)
    }

    fn write_to(&self, builder: &mut CellBuilder) -> Result<(), TonCellError> {
        self.as_ref().write_to(builder)?;
        Ok(())
    }

    fn from_boc(boc: &[u8]) -> Result<Self, TonCellError> {
        BagOfCells::parse(boc)?.single_root()
    }
}

impl TLBObject for TonHash {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        let byes = parser.load_bytes(TON_HASH_LEN)?;
        Ok(TonHash::try_from(byes)?)
    }

    fn write_to(&self, builder: &mut CellBuilder) -> Result<(), TonCellError> {
        builder.store_bits(TON_HASH_LEN * 8, self.as_slice())?;
        Ok(())
    }
}
