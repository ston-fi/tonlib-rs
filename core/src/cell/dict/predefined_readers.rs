use num_bigint::{BigInt, BigUint};
use num_traits::ToPrimitive;

use crate::cell::TonCellError::{InternalError, InvalidInput};
use crate::cell::{ArcCell, Cell, CellParser, TonCellError};
use crate::types::TON_HASH_BYTES;
use crate::TonHash;

pub fn key_reader_u8(raw_key: &BigUint) -> Result<u8, TonCellError> {
    validate_bit_len(raw_key, 8)?;
    ok_or_err(raw_key.to_u8())
}

pub fn key_reader_u16(raw_key: &BigUint) -> Result<u16, TonCellError> {
    validate_bit_len(raw_key, 16)?;
    ok_or_err(raw_key.to_u16())
}

pub fn key_reader_u32(raw_key: &BigUint) -> Result<u32, TonCellError> {
    validate_bit_len(raw_key, 32)?;
    ok_or_err(raw_key.to_u32())
}

pub fn key_reader_u64(raw_key: &BigUint) -> Result<u64, TonCellError> {
    validate_bit_len(raw_key, 64)?;
    ok_or_err(raw_key.to_u64())
}

pub fn key_reader_256bit(val: &BigUint) -> Result<TonHash, TonCellError> {
    validate_bit_len(val, TON_HASH_BYTES * 8)?;
    let digits = val.to_bytes_be();
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

pub fn key_reader_uint(raw_key: &BigUint) -> Result<BigUint, TonCellError> {
    Ok(raw_key.clone())
}

pub fn key_reader_decimal_string(raw_key: &BigUint) -> Result<String, TonCellError> {
    Ok(key_reader_uint(raw_key)?.to_str_radix(10))
}

pub fn val_reader_cell(parser: &mut CellParser) -> Result<Cell, TonCellError> {
    parser.load_remaining()
}

pub fn val_reader_ref_cell(parser: &mut CellParser) -> Result<ArcCell, TonCellError> {
    parser.next_reference()
}

pub fn val_reader_snake_formatted_string(parser: &mut CellParser) -> Result<Vec<u8>, TonCellError> {
    let mut buffer = Vec::new();
    parser.next_reference()?.parse_snake_data(&mut buffer)?;
    Ok(buffer)
}

pub fn val_reader_uint(parser: &mut CellParser) -> Result<BigUint, TonCellError> {
    let remaining = parser.remaining_bits();
    let result = parser.load_uint(remaining)?;
    Ok(result)
}

pub fn val_reader_int(parser: &mut CellParser) -> Result<BigInt, TonCellError> {
    let remaining = parser.remaining_bits();
    let result = parser.load_int(remaining)?;
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
