use std::sync::Arc;

use crate::cell::{ArcCell, BagOfCells, Cell, CellBuilder, CellParser, TonCellError};
use crate::tlb_types::tlb::TLB;
use crate::types::TON_HASH_LEN;
use crate::TonHash;

impl TLB for Cell {
    fn read_definition(parser: &mut CellParser) -> Result<Self, TonCellError> {
        if parser.cell.bit_len() == parser.remaining_bits()
            && parser.remaining_refs() == parser.cell.references().len()
        {
            Ok(parser.cell.clone())
        } else {
            // TODO not clear how to handle exotics with current implementation
            parser.load_remaining()
        }
    }

    fn write_definition(&self, builder: &mut CellBuilder) -> Result<(), TonCellError> {
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

impl TLB for ArcCell {
    fn read_definition(parser: &mut CellParser) -> Result<Self, TonCellError> {
        Ok(Cell::read(parser)?.to_arc())
    }

    fn write_definition(&self, builder: &mut CellBuilder) -> Result<(), TonCellError> {
        self.as_ref().write(builder)?;
        Ok(())
    }

    fn from_boc(boc: &[u8]) -> Result<Self, TonCellError> {
        BagOfCells::parse(boc)?.single_root()
    }
}

impl TLB for TonHash {
    fn read_definition(parser: &mut CellParser) -> Result<Self, TonCellError> {
        let bytes = parser.load_bits(TON_HASH_LEN * 8)?;
        Ok(TonHash::try_from(bytes)?)
    }

    fn write_definition(&self, builder: &mut CellBuilder) -> Result<(), TonCellError> {
        builder.store_bits(TON_HASH_LEN * 8, self.as_slice())?;
        Ok(())
    }
}
