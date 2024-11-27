use crate::emulator::error::TvmEmulatorError;
use crate::types::TvmStackEntry;
use num_bigint::Sign;
use tonlib_core::cell::{BagOfCells, Cell, CellBuilder, EMPTY_ARC_CELL, EMPTY_CELL};

#[allow(clippy::let_and_return)]
pub(super) fn build_stack_boc(stack: &[TvmStackEntry]) -> Result<Vec<u8>, TvmEmulatorError> {
    let root_cell = if stack.is_empty() {
        // empty stack should contain header cell with 24 bit number containing number of elements (0)
        // and reference to empty cell
        // Cell{ data: [000000], bit_len: 24, references: [
        //     Cell{ data: [], bit_len: 0, references: [
        //     ] }
        // ] }
        let root_cell = CellBuilder::new()
            .store_u64(24, 0)?
            .store_reference(&EMPTY_ARC_CELL.clone())?
            .build()?;
        root_cell
    } else {
        let empty_cell = EMPTY_CELL.clone();
        let mut prev_cell: Cell = empty_cell;
        for i in 0..stack.len() {
            let mut builder = CellBuilder::new();
            builder.store_child(prev_cell)?;
            if i == stack.len() - 1 {
                builder.store_u32(24, stack.len() as u32)?;
            }
            store_stack_entry(&mut builder, &stack[i])?;
            let new_cell = builder.build()?;
            prev_cell = new_cell;
        }
        prev_cell
    };
    log::trace!("Produced stack:\n{:?}", root_cell);
    Ok(BagOfCells::from_root(root_cell).serialize(false)?)
}

fn store_stack_entry(
    builder: &mut CellBuilder,
    entry: &TvmStackEntry,
) -> Result<(), TvmEmulatorError> {
    match entry {
        TvmStackEntry::Null => {
            builder.store_byte(0)?;
            Ok(())
        }
        TvmStackEntry::Nan => {
            builder.store_byte(2)?.store_byte(0xff)?;
            Ok(())
        }
        TvmStackEntry::Int64(val) => {
            builder.store_byte(1)?.store_i64(64, *val)?;
            Ok(())
        }
        TvmStackEntry::Int257(val) => {
            let (sign, mag) = val.clone().into_parts();
            builder.store_byte(2)?;
            if sign == Sign::Minus {
                builder.store_byte(1)?;
            } else {
                builder.store_byte(0)?;
            };
            builder.store_uint(256, &mag)?;
            Ok(())
        }
        TvmStackEntry::Cell(cell) => {
            builder.store_reference(cell)?;
            builder.store_byte(3)?;
            Ok(())
        }
        TvmStackEntry::Slice(slice) => {
            let cell = slice.into_cell()?;
            builder.store_reference(&cell.into())?;
            builder.store_byte(4)?;
            builder.store_u32(10, slice.start_bit as u32)?; // st_bits
            builder.store_u32(10, slice.end_bit as u32)?; // en_bits
            builder.store_u8(3, slice.start_ref as u8)?; // st_ref
            builder.store_u8(3, slice.end_ref as u8)?; // en_ref
            Ok(())
        }
        TvmStackEntry::Unsupported => Err(TvmEmulatorError::EmulatorError(
            "EmulatorStackEntry::Unsupported is not supported".to_string(),
        )),
    }
}
