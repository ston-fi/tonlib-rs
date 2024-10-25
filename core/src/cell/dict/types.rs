use std::collections::HashMap;

use num_bigint::BigUint;

use crate::cell::{CellBuilder, CellParser, TonCellError};
use crate::TonHash;

#[derive(Debug)]
pub(crate) enum LabelType {
    Short, // high bit is 0
    Long,  // high bits are 10
    Same,  // high bits are 11
}

pub type SnakeFormatDict = HashMap<TonHash, Vec<u8>>;
pub type KeyReader<K> = fn(&BigUint) -> Result<K, TonCellError>;
pub type ValReader<V> = fn(&mut CellParser) -> Result<V, TonCellError>;
pub type ValWriter<V> = fn(&mut CellBuilder, V) -> Result<(), TonCellError>;
