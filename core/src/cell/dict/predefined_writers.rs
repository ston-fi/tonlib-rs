use std::cmp::max;
use std::sync::Arc;

use num_bigint::{BigInt, BigUint};

use crate::cell::{Cell, CellBuilder, TonCellError};

#[allow(dead_code)]
pub fn val_writer_ref_cell(builder: &mut CellBuilder, val: Arc<Cell>) -> Result<(), TonCellError> {
    builder.store_reference(&val)?;
    Ok(())
}

pub fn val_writer_unsigned_min_size<V>(
    builder: &mut CellBuilder,
    val: V,
) -> Result<(), TonCellError>
where
    BigUint: From<V>,
{
    let internal_val = BigUint::from(val);
    let len_bits = max(1, internal_val.bits()) as usize;
    builder.store_uint(len_bits, &internal_val)?;
    Ok(())
}

pub fn val_writer_signed_min_size<V>(builder: &mut CellBuilder, val: V) -> Result<(), TonCellError>
where
    BigInt: From<V>,
{
    let internal_val = BigInt::from(val);
    let len_bits = max(1, internal_val.bits()) as usize;
    builder.store_int(len_bits, &internal_val)?;
    Ok(())
}
