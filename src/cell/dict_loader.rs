use std::hash::Hash;
use std::ops::ShrAssign;

use num_bigint::{BigInt, BigUint};

use crate::cell::{CellSlice, TonCellError};

pub trait DictLoader<K, V>
where
    K: Hash + Eq,
{
    fn extract_key(&self, key: &[u8]) -> Result<K, TonCellError>;

    fn extract_value(&self, value: &CellSlice) -> Result<V, TonCellError>;
    fn key_bit_len(&self) -> usize;
}

pub fn key_extractor_u8(bit_len: usize, key: &[u8]) -> Result<u8, TonCellError> {
    if bit_len == 8 {
        Ok(key[0])
    } else {
        Err(TonCellError::CellParserError(format!(
            "Invalid key len: {}, expected 8 bits",
            bit_len
        )))
    }
}

pub fn key_extractor_u16(bit_len: usize, key: &[u8]) -> Result<u16, TonCellError> {
    if bit_len == 16 {
        let arr: &[u8; 2] = key.try_into().map_err(|_| {
            TonCellError::CellParserError("Insufficient bytes in the dictionary key.".to_string())
        })?;
        Ok(u16::from_be_bytes(*arr))
    } else {
        Err(TonCellError::CellParserError(format!(
            "Invalid key len: {}, expected 16 bits",
            bit_len
        )))
    }
}

pub fn key_extractor_u32(bit_len: usize, key: &[u8]) -> Result<u32, TonCellError> {
    if bit_len == 32 {
        let arr: &[u8; 4] = key.try_into().map_err(|_| {
            TonCellError::CellParserError("Insufficient bytes in the dictionary key.".to_string())
        })?;
        Ok(u32::from_be_bytes(*arr))
    } else {
        Err(TonCellError::CellParserError(format!(
            "Invalid key len: {}, expected 32 bits",
            bit_len
        )))
    }
}

pub fn key_extractor_u64(bit_len: usize, key: &[u8]) -> Result<u64, TonCellError> {
    if bit_len == 64 {
        let arr: &[u8; 8] = key.try_into().map_err(|_| {
            TonCellError::CellParserError("Insufficient bytes in the dictionary key.".to_string())
        })?;
        Ok(u64::from_be_bytes(*arr))
    } else {
        Err(TonCellError::CellParserError(format!(
            "Invalid key len: {}, expected 64 bits",
            bit_len
        )))
    }
}

pub fn key_extractor_256bit(bit_len: usize, key: &[u8]) -> Result<[u8; 32], TonCellError> {
    if bit_len == 256 {
        TryInto::<[u8; 32]>::try_into(key).map_err(|e| TonCellError::InternalError(e.to_string()))
    } else {
        Err(TonCellError::CellParserError(format!(
            "Invalid key len: {}, expected 256 bits",
            bit_len
        )))
    }
}
pub fn key_extractor_uint(bit_len: usize, key: &[u8]) -> Result<BigUint, TonCellError> {
    let mut extracted_key: BigUint = BigUint::from_bytes_be(key);
    let remainder = bit_len % 8;
    if remainder != 0 {
        extracted_key.shr_assign(8 - remainder);
    }
    Ok(extracted_key)
}

pub fn key_extractor_decimal_string(bit_len: usize, key: &[u8]) -> Result<String, TonCellError> {
    Ok(key_extractor_uint(bit_len, key)?.to_str_radix(10))
}

pub fn value_extractor_snake_formatted_string(
    cell_slice: &CellSlice,
) -> Result<Vec<u8>, TonCellError> {
    let mut buffer = Vec::new();
    cell_slice.reference(0)?.parse_snake_data(&mut buffer)?;
    Ok(buffer)
}

pub fn value_extractor_uint(cell_slice: &CellSlice) -> Result<BigUint, TonCellError> {
    let bit_len = cell_slice.end_bit - cell_slice.start_bit;
    cell_slice.parser()?.skip_bits(cell_slice.start_bit)?;
    cell_slice.parser()?.load_uint(bit_len)
}

pub fn value_extractor_int(cell_slice: &CellSlice) -> Result<BigInt, TonCellError> {
    let bit_len = cell_slice.end_bit - cell_slice.start_bit;
    cell_slice.parser()?.skip_bits(cell_slice.start_bit)?;
    cell_slice.parser()?.load_int(bit_len)
}

pub struct GenericDictLoader<K, V, KX, VX>
where
    KX: FnOnce(usize, &[u8]) -> Result<K, TonCellError> + Copy,
    VX: FnOnce(&CellSlice) -> Result<V, TonCellError>,
{
    key_extractor: KX,
    value_extractor: VX,
    bit_len: usize,
}

impl<K, V, KX, VX> GenericDictLoader<K, V, KX, VX>
where
    KX: FnOnce(usize, &[u8]) -> Result<K, TonCellError> + Copy,
    VX: FnOnce(&CellSlice) -> Result<V, TonCellError>,
{
    pub fn new(
        key_extractor: KX,
        value_extractor: VX,
        bit_len: usize,
    ) -> GenericDictLoader<K, V, KX, VX> {
        GenericDictLoader {
            key_extractor,
            value_extractor,
            bit_len,
        }
    }
}

impl<K, V, KX, VX> DictLoader<K, V> for GenericDictLoader<K, V, KX, VX>
where
    K: Hash + Eq,
    KX: FnOnce(usize, &[u8]) -> Result<K, TonCellError> + Copy,
    VX: FnOnce(&CellSlice) -> Result<V, TonCellError> + Copy,
{
    fn extract_key(&self, key: &[u8]) -> Result<K, TonCellError> {
        (self.key_extractor)(self.bit_len, key)
    }

    fn extract_value(&self, value: &CellSlice) -> Result<V, TonCellError> {
        (self.value_extractor)(value)
    }
    fn key_bit_len(&self) -> usize {
        self.bit_len
    }
}
