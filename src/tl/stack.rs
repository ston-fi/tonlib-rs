use std::collections::HashMap;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::hash::Hash;

use num_bigint::{BigInt, BigUint};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::address::TonAddress;
use crate::cell::{BagOfCells, DictLoader};
use crate::tl::error::TvmStackError;
use crate::tl::Base64Standard;

// tonlib_api.tl, line 166
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct TvmSlice {
    #[serde(with = "Base64Standard")]
    pub bytes: Vec<u8>,
}

impl Debug for TvmSlice {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TvmSlice{{ bytes: [{}]}}",
            self.bytes
                .iter()
                .map(|&byte| format!("{:02X}", byte))
                .collect::<Vec<_>>()
                .join(""),
        )?;
        Ok(())
    }
}

// tonlib_api.tl, line 167
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct TvmCell {
    #[serde(with = "Base64Standard")]
    pub bytes: Vec<u8>,
}

impl Debug for TvmCell {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Print bytes as a hexadecimal string
        write!(f, "TvmCell {{ bytes: 0x")?;

        for byte in &self.bytes {
            write!(f, "{:02x}", byte)?;
        }

        write!(f, " }}")
    }
}

// tonlib_api.tl, line 168
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TvmNumber {
    pub number: String,
}

// tonlib_api.tl, line 169
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TvmTuple {
    pub elements: Vec<TvmStackEntry>,
}

// tonlib_api.tl, line 170
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TvmList {
    pub elements: Vec<TvmStackEntry>,
}

// tonlib_api.tl, line 172
#[derive(Serialize, Deserialize, strum::Display, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(tag = "@type")]
pub enum TvmStackEntry {
    // tonlib_api.tl, line 172
    #[serde(rename = "tvm.stackEntrySlice")]
    // tonlib_api.tl, line 173
    Slice { slice: TvmSlice },
    #[serde(rename = "tvm.stackEntryCell")]
    Cell { cell: TvmCell },
    // tonlib_api.tl, line 174
    #[serde(rename = "tvm.stackEntryNumber")]
    Number { number: TvmNumber },
    // tonlib_api.tl, line 175
    #[serde(rename = "tvm.stackEntryTuple")]
    Tuple { tuple: TvmTuple },
    // tonlib_api.tl, line 176
    #[serde(rename = "tvm.stackEntryList")]
    List { list: TvmList },
    // tonlib_api.tl, line 177
    #[serde(rename = "tvm.stackEntryUnsupported")]
    Unsupported {},
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TvmStack {
    pub elements: Vec<TvmStackEntry>,
}

impl<'de> Deserialize<'de> for TvmStack {
    fn deserialize<D>(deserializer: D) -> Result<TvmStack, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).map(|e| TvmStack { elements: e })
    }
}

impl Serialize for TvmStack {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.elements.serialize(serializer)
    }
}

impl TvmStack {
    pub fn new() -> TvmStack {
        TvmStack {
            elements: Vec::new(),
        }
    }

    pub fn from(elements: &[TvmStackEntry]) -> TvmStack {
        TvmStack {
            elements: elements.to_vec(),
        }
    }

    pub fn get_string(&self, index: usize) -> Result<String, TvmStackError> {
        self.get(index, TvmStack::extract_string)
    }

    pub fn get_i32(&self, index: usize) -> Result<i32, TvmStackError> {
        self.get(index, TvmStack::extract_i32)
    }

    pub fn get_i64(&self, index: usize) -> Result<i64, TvmStackError> {
        self.get(index, TvmStack::extract_i64)
    }

    pub fn get_biguint(&self, index: usize) -> Result<BigUint, TvmStackError> {
        self.get(index, TvmStack::extract_biguint)
    }

    pub fn get_bigint(&self, index: usize) -> Result<BigInt, TvmStackError> {
        self.get(index, TvmStack::extract_bigint)
    }

    pub fn get_boc(&self, index: usize) -> Result<BagOfCells, TvmStackError> {
        self.get(index, TvmStack::extract_boc)
    }

    pub fn get_address(&self, index: usize) -> Result<TonAddress, TvmStackError> {
        self.get_boc(index)?
            .single_root()?
            .parse_fully(|r| r.load_address())
            .map_err(TvmStackError::TonCellError)
    }

    pub fn get_dict<K, V, L>(
        &self,
        index: usize,
        loader: &L,
    ) -> Result<HashMap<K, V>, TvmStackError>
    where
        K: Hash + Eq + Clone,
        L: DictLoader<K, V>,
    {
        let boc = self.get_boc(index)?;
        let cell = boc.single_root()?;
        Ok(cell.load_generic_dict(loader)?)
    }

    fn get<T>(
        &self,
        index: usize,
        extract: fn(&TvmStackEntry, usize) -> Result<T, TvmStackError>,
    ) -> Result<T, TvmStackError> {
        let maybe_elem = self.elements.get(index);
        match maybe_elem {
            None => Err(TvmStackError::InvalidTvmStackIndex {
                index,
                len: self.elements.len(),
            }),
            Some(e) => extract(e, index),
        }
    }

    fn extract_string(e: &TvmStackEntry, index: usize) -> Result<String, TvmStackError> {
        if let TvmStackEntry::Number { number } = e {
            number
                .number
                .parse()
                .map_err(|_| TvmStackError::StringConversion {
                    e: e.clone(),
                    index,
                })
        } else {
            Err(TvmStackError::StringConversion {
                e: e.clone(),
                index,
            })
        }
    }

    fn extract_i32(e: &TvmStackEntry, index: usize) -> Result<i32, TvmStackError> {
        if let TvmStackEntry::Number { number } = e {
            number
                .number
                .parse()
                .map_err(|_| TvmStackError::I32Conversion {
                    e: e.clone(),
                    index,
                })
        } else {
            Err(TvmStackError::I32Conversion {
                e: e.clone(),
                index,
            })
        }
    }

    fn extract_i64(e: &TvmStackEntry, index: usize) -> Result<i64, TvmStackError> {
        if let TvmStackEntry::Number { number } = e {
            number
                .number
                .parse()
                .map_err(|_| TvmStackError::I64Conversion {
                    e: e.clone(),
                    index,
                })
        } else {
            Err(TvmStackError::I64Conversion {
                e: e.clone(),
                index,
            })
        }
    }

    fn extract_biguint(e: &TvmStackEntry, index: usize) -> Result<BigUint, TvmStackError> {
        if let TvmStackEntry::Number { number } = e {
            number
                .number
                .parse()
                .map_err(|_| TvmStackError::BigUintConversion {
                    e: e.clone(),
                    index,
                })
        } else {
            Err(TvmStackError::BigUintConversion {
                e: e.clone(),
                index,
            })
        }
    }

    fn extract_bigint(e: &TvmStackEntry, index: usize) -> Result<BigInt, TvmStackError> {
        if let TvmStackEntry::Number { number } = e {
            number
                .number
                .parse()
                .map_err(|_| TvmStackError::BigIntConversion {
                    e: e.clone(),
                    index,
                })
        } else {
            Err(TvmStackError::BigIntConversion {
                e: e.clone(),
                index,
            })
        }
    }

    fn extract_boc(e: &TvmStackEntry, index: usize) -> Result<BagOfCells, TvmStackError> {
        match e {
            TvmStackEntry::Slice { slice } => {
                BagOfCells::parse(&slice.bytes).map_err(|_| TvmStackError::BoCConversion {
                    e: e.clone(),
                    index,
                })
            }
            TvmStackEntry::Cell { cell } => {
                BagOfCells::parse(&cell.bytes).map_err(|_| TvmStackError::BoCConversion {
                    e: e.clone(),
                    index,
                })
            }
            _ => Err(TvmStackError::BoCConversion {
                e: e.clone(),
                index,
            }),
        }
    }
}

impl Default for TvmStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {

    use crate::tl::stack::{TvmNumber, TvmStack, TvmStackEntry};

    const SERIAL: &str = r#"[{"@type":"tvm.stackEntryNumber","number":{"number":"100500"}}]"#;

    #[test]
    fn serialize_works() {
        let mut stack = TvmStack::default();
        stack.elements.push(TvmStackEntry::Number {
            number: TvmNumber {
                number: String::from("100500"),
            },
        });
        let serial = serde_json::to_string(&stack).unwrap();
        println!("{}", serial);
        assert_eq!(serial.as_str(), SERIAL)
    }

    #[test]
    fn deserialize_works() {
        let stack: TvmStack = serde_json::from_str(SERIAL).unwrap();
        assert_eq!(stack.elements.len(), 1);
        assert_eq!(100500, stack.get_i32(0).unwrap());
    }
}
