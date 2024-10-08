use crate::cell::TonCellError::{InternalError, InvalidInput};
use crate::cell::{Cell, CellSlice, TonCellError};
use crate::types::TON_HASH_BYTES;
use crate::TonHash;
use num_bigint::{BigInt, BigUint};
use num_traits::ToPrimitive;

pub fn key_extractor_u8(raw_key: &BigUint) -> Result<u8, TonCellError> {
    validate_bit_len(raw_key, 8)?;
    ok_or_err(raw_key.to_u8())
}

pub fn key_extractor_u16(raw_key: &BigUint) -> Result<u16, TonCellError> {
    validate_bit_len(raw_key, 16)?;
    ok_or_err(raw_key.to_u16())
}

pub fn key_extractor_u32(raw_key: &BigUint) -> Result<u32, TonCellError> {
    validate_bit_len(raw_key, 32)?;
    ok_or_err(raw_key.to_u32())
}

pub fn key_extractor_u64(raw_key: &BigUint) -> Result<u64, TonCellError> {
    validate_bit_len(raw_key, 64)?;
    ok_or_err(raw_key.to_u64())
}

pub fn key_extractor_256bit(val: &BigUint) -> Result<TonHash, TonCellError> {
    validate_bit_len(val, TON_HASH_BYTES * 8)?;
    let digits = val.to_bytes_le();
    let key_digits = if digits.len() < 32 {
        let mut tmp = vec![0u8; 32 - digits.len()];
        tmp.extend(digits);
        tmp
    } else {
        digits
    };
    let slice: [u8; 32] = key_digits.try_into().map_err(|_| {
        let msg = format!("Fail to get [u8; 32] from {}", val);
        InternalError(msg)
    })?;
    Ok(slice)
}

pub fn key_extractor_uint(raw_key: &BigUint) -> Result<BigUint, TonCellError> {
    Ok(raw_key.clone())
}

pub fn key_extractor_decimal_string(raw_key: &BigUint) -> Result<String, TonCellError> {
    Ok(key_extractor_uint(raw_key)?.to_str_radix(10))
}

pub fn val_extractor_cell(cell_slice: &CellSlice) -> Result<Cell, TonCellError> {
    cell_slice.into_cell()
}

pub fn val_extractor_snake_formatted_string(
    cell_slice: &CellSlice,
) -> Result<Vec<u8>, TonCellError> {
    let mut buffer = Vec::new();
    cell_slice.reference(0)?.parse_snake_data(&mut buffer)?;
    Ok(buffer)
}

pub fn val_extractor_uint(cell_slice: &CellSlice) -> Result<BigUint, TonCellError> {
    let bit_len = cell_slice.end_bit - cell_slice.start_bit;
    let mut parser = cell_slice.cell.parser();
    parser.skip_bits(cell_slice.start_bit)?;
    let result = parser.load_uint(bit_len)?;
    Ok(result)
}

pub fn val_extractor_int(cell_slice: &CellSlice) -> Result<BigInt, TonCellError> {
    let bit_len = cell_slice.end_bit - cell_slice.start_bit;
    let mut parser = cell_slice.cell.parser();
    parser.skip_bits(cell_slice.start_bit)?;
    let result = parser.load_int(bit_len)?;
    Ok(result)
}

fn validate_bit_len(val: &BigUint, max_bits: usize) -> Result<(), TonCellError> {
    if val.bits() > max_bits as u64 {
        let msg = format!(
            "Invalid value len: {}, expected {max_bits} bits",
            val.bits()
        );
        return Err(InvalidInput(msg));
    }
    Ok(())
}

fn ok_or_err<T>(val: Option<T>) -> Result<T, TonCellError> {
    val.ok_or_else(|| {
        let msg = format!(
            "Fail to extract {} from BigUint",
            std::any::type_name::<T>()
        );
        InternalError(msg)
    })
}
