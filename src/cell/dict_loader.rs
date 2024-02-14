use std::hash::Hash;

use num_bigint::BigUint;

use crate::cell::{Cell, TonCellError};

pub trait DictLoader<K, V>
where
    K: Hash + Eq,
{
    fn extract_key(&self, key: &[u8]) -> Result<K, TonCellError>;

    fn extract_value(&self, value: &Cell) -> Result<V, TonCellError>;
    fn key_bit_len(&self) -> usize;
}

pub fn bytes_to_decimal_string(slice: &[u8]) -> Result<String, TonCellError> {
    Ok(BigUint::from_bytes_be(slice).to_str_radix(10))
}

pub fn bytes_to_slice(slice: &[u8]) -> Result<[u8; 32], TonCellError> {
    TryInto::<[u8; 32]>::try_into(slice).map_err(|e| TonCellError::InternalError(e.to_string()))
}

pub fn cell_to_snake_formatted_string(cell: &Cell) -> Result<Vec<u8>, TonCellError> {
    let mut buffer = Vec::new();
    cell.reference(0)?.parse_snake_data(&mut buffer)?;

    Ok(buffer)
}

pub struct GenericDictLoader<K, V, KX, VX>
where
    KX: FnOnce(&[u8]) -> Result<K, TonCellError> + Copy,
    VX: FnOnce(&Cell) -> Result<V, TonCellError>,
{
    key_extractor: KX,
    value_extractor: VX,
    bit_len: usize,
}

impl<K, V, KX, VX> GenericDictLoader<K, V, KX, VX>
where
    KX: FnOnce(&[u8]) -> Result<K, TonCellError> + Copy,
    VX: FnOnce(&Cell) -> Result<V, TonCellError>,
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

impl<K, V, KC, VX> DictLoader<K, V> for GenericDictLoader<K, V, KC, VX>
where
    K: Hash + Eq,
    KC: FnOnce(&[u8]) -> Result<K, TonCellError> + Copy,
    VX: FnOnce(&Cell) -> Result<V, TonCellError> + Copy,
{
    fn extract_key(&self, key: &[u8]) -> Result<K, TonCellError> {
        (self.key_extractor)(key)
    }

    fn extract_value(&self, value: &Cell) -> Result<V, TonCellError> {
        (self.value_extractor)(value)
    }
    fn key_bit_len(&self) -> usize {
        self.bit_len
    }
}
