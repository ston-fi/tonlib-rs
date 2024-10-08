use crate::cell::{CellBuilder, CellSlice, TonCellError};
use crate::TonHash;
use num_bigint::BigUint;
use std::collections::HashMap;

#[derive(Debug)]
pub(crate) enum LabelType {
    Short, // high bit is 0
    Long,  // high bits are 10
    Same,  // high bits are 11
}

pub type SnakeFormatDict = HashMap<TonHash, Vec<u8>>;
pub type KeyExtractor<K> = fn(&BigUint) -> Result<K, TonCellError>;
pub type ValExtractor<V> = fn(&CellSlice) -> Result<V, TonCellError>;
pub type ValWriter<V> = fn(&mut CellBuilder, V) -> Result<(), TonCellError>;
