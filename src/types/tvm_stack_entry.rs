use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;

use num_bigint::{BigInt, BigUint};
use strum::Display;

use crate::address::TonAddress;
use crate::cell::{ArcCell, BagOfCells, Cell, CellBuilder, CellSlice, DictLoader};
use crate::tl::{TvmCell, TvmNumber, TvmSlice, TvmStackEntry as TlTvmStackEntry};
use crate::types::StackParseError;

#[derive(Debug, Display, Clone, PartialEq)]
pub enum TvmStackEntry {
    Null,
    Nan,
    Int64(i64),
    Int257(BigInt),
    Cell(ArcCell),
    Slice(CellSlice),
    Unsupported,
}

impl TvmStackEntry {
    pub fn get_bool(&self) -> Result<bool, StackParseError> {
        match self {
            TvmStackEntry::Int64(number) => match number {
                0 => Ok(false),
                -1 => Ok(true),
                n => Err(StackParseError::InvalidEntryValue(format!(
                    "expected boolean, found number:{}",
                    n
                ))),
            },
            TvmStackEntry::Int257(number) => {
                let number: i64 = number.clone().try_into().map_err(|_| {
                    StackParseError::InvalidEntryValue("Received number exceeds i64".to_string())
                })?;
                match number {
                    0 => Ok(false),
                    -1 => Ok(true),
                    n => Err(StackParseError::InvalidEntryValue(format!(
                        "expected boolean, found number:{}",
                        n
                    ))),
                }
            }
            t => Err(StackParseError::InvalidEntryType {
                expected: "Number".to_string(),
                found: t.clone(),
            }),
        }
    }

    //TODO:get_maybe_iXXX(&self) -> Result<Option<iXXX>, StackParseError>

    pub fn get_i64(&self) -> Result<i64, StackParseError> {
        match self {
            TvmStackEntry::Int64(number) => Ok(*number),
            TvmStackEntry::Int257(number) => number.try_into().map_err(|_| {
                StackParseError::InvalidEntryValue("Received number exceeds i64".to_string())
            }),
            t => Err(StackParseError::InvalidEntryType {
                expected: "Number".to_string(),
                found: t.clone(),
            }),
        }
    }

    pub fn get_bigint(&self) -> Result<BigInt, StackParseError> {
        match self {
            TvmStackEntry::Int64(number) => Ok(BigInt::from(*number)),
            TvmStackEntry::Int257(number) => Ok(number.clone()),
            t => Err(StackParseError::InvalidEntryType {
                expected: "Number".to_string(),
                found: t.clone(),
            }),
        }
    }

    pub fn get_biguint(&self) -> Result<BigUint, StackParseError> {
        self.get_bigint()?
            .try_into()
            .map_err(|_| StackParseError::InvalidEntryValue("Positive number expected".to_string()))
    }

    pub fn get_cell(&self) -> Result<ArcCell, StackParseError> {
        match self {
            TvmStackEntry::Cell(cell) => Ok(cell.clone()),
            t => Err(StackParseError::InvalidEntryType {
                expected: "Cell".to_string(),
                found: t.clone(),
            }),
        }
    }

    pub fn get_address(&self) -> Result<TonAddress, StackParseError> {
        match self {
            TvmStackEntry::Cell(cell) => cell
                .parse_fully(|r| r.load_address())
                .map_err(StackParseError::CellError),
            TvmStackEntry::Slice(slice) => slice
                .parse_fully(|r| r.load_address())
                .map_err(StackParseError::CellError),
            t => Err(StackParseError::InvalidEntryType {
                expected: "Slice".to_string(),
                found: t.clone(),
            }),
        }
    }

    pub fn get_dict<K, V, L>(&self, loader: &L) -> Result<HashMap<K, V>, StackParseError>
    where
        K: Hash + Eq + Clone,
        L: DictLoader<K, V>,
    {
        match self {
            TvmStackEntry::Cell(cell) => {
                let result: HashMap<K, V> = cell.load_generic_dict(loader)?;
                Ok(result)
            }

            t => Err(StackParseError::InvalidEntryType {
                expected: "Slice".to_string(),
                found: t.clone(),
            }),
        }
    }
}

impl From<bool> for TvmStackEntry {
    fn from(value: bool) -> Self {
        let i = if value { -1 } else { 0 };
        TvmStackEntry::Int64(i)
    }
}

impl From<i64> for TvmStackEntry {
    fn from(value: i64) -> Self {
        TvmStackEntry::Int64(value)
    }
}

impl From<BigInt> for TvmStackEntry {
    fn from(value: BigInt) -> Self {
        TvmStackEntry::Int257(value)
    }
}

impl From<BigUint> for TvmStackEntry {
    fn from(value: BigUint) -> Self {
        TvmStackEntry::Int257(value.into())
    }
}

impl From<Cell> for TvmStackEntry {
    fn from(value: Cell) -> Self {
        TvmStackEntry::Cell(Arc::new(value))
    }
}

impl TryFrom<&TonAddress> for TvmStackEntry {
    type Error = StackParseError;

    fn try_from(value: &TonAddress) -> Result<Self, Self::Error> {
        let cell = CellBuilder::new().store_address(value)?.build()?;
        Ok(TvmStackEntry::Slice(CellSlice::full_cell(cell)?))
    }
}

impl TryFrom<&TvmStackEntry> for TlTvmStackEntry {
    type Error = StackParseError;

    fn try_from(value: &TvmStackEntry) -> Result<Self, Self::Error> {
        let e = match value {
            TvmStackEntry::Slice(cell_slice) => {
                let c = cell_slice.into_cell()?;
                let b = BagOfCells::from_root(c).serialize(false)?;
                TlTvmStackEntry::Slice {
                    slice: TvmSlice { bytes: b },
                }
            }
            TvmStackEntry::Cell(cell) => {
                let a = BagOfCells::from_root(cell.as_ref().clone()).serialize(false)?;
                TlTvmStackEntry::Cell {
                    cell: TvmCell { bytes: a },
                }
            }
            TvmStackEntry::Int64(number) => TlTvmStackEntry::Number {
                number: TvmNumber {
                    number: number.to_string(),
                },
            },
            TvmStackEntry::Int257(number) => TlTvmStackEntry::Number {
                number: TvmNumber {
                    number: number.to_string(),
                },
            },
            TvmStackEntry::Unsupported => TlTvmStackEntry::Unsupported {},
            TvmStackEntry::Null => TlTvmStackEntry::Unsupported {},
            TvmStackEntry::Nan => TlTvmStackEntry::Unsupported {},
        };
        Ok(e)
    }
}

impl TryFrom<&TlTvmStackEntry> for TvmStackEntry {
    type Error = StackParseError;
    fn try_from(value: &TlTvmStackEntry) -> Result<Self, Self::Error> {
        let entry = match value {
            TlTvmStackEntry::Slice { slice } => {
                let slice = &slice.bytes;
                let boc = BagOfCells::parse(slice.as_slice())?;
                let cell = boc.single_root()?;

                let cell_slice = CellSlice {
                    cell: cell.clone(),
                    start_bit: 0,
                    start_ref: 0,
                    end_bit: cell.bit_len,
                    end_ref: 0,
                };
                TvmStackEntry::Slice(cell_slice)
            }
            TlTvmStackEntry::Cell { cell } => {
                let boc = BagOfCells::parse(cell.bytes.as_slice())?;
                let cell = boc.single_root()?;
                TvmStackEntry::Cell(cell.clone())
            }
            TlTvmStackEntry::Number { number } => {
                let number = number
                    .number
                    .parse()
                    .map_err(|_| StackParseError::InvalidEntryValue(number.number.clone()))?;
                TvmStackEntry::Int257(number)
            }

            TlTvmStackEntry::Tuple { tuple: _ } => TvmStackEntry::Unsupported,

            TlTvmStackEntry::List { list: _ } => TvmStackEntry::Unsupported,

            TlTvmStackEntry::Unsupported {} => TvmStackEntry::Unsupported,
        };
        Ok(entry)
    }
}
