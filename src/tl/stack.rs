use std::str::FromStr;

use anyhow::anyhow;
use num_bigint::{BigInt, BigUint};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::cell::BagOfCells;

use crate::tl::Base64Standard;

// tonlib_api.tl, line 164
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TvmSlice {
    #[serde(with = "Base64Standard")]
    pub bytes: Vec<u8>,
}

// tonlib_api.tl, line 165
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TvmCell {
    #[serde(with = "Base64Standard")]
    pub bytes: Vec<u8>,
}

// tonlib_api.tl, line 166
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TvmNumber {
    pub number: String, // TODO: Deserialize i256
}

// tonlib_api.tl, line 163
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TvmTuple {
    pub elements: Vec<TvmStackEntry>,
}

// tonlib_api.tl, line 168
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TvmList {
    pub elements: Vec<TvmStackEntry>,
}

// tonlib_api.tl, line 170
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(tag = "@type")]
pub enum TvmStackEntry {
    // tonlib_api.tl, line 170
    #[serde(rename = "tvm.stackEntrySlice")]
    Slice { slice: TvmSlice },
    // tonlib_api.tl, line 171
    #[serde(rename = "tvm.stackEntryCell")]
    Cell { cell: TvmCell },
    // tonlib_api.tl, line 172
    #[serde(rename = "tvm.stackEntryNumber")]
    Number { number: TvmNumber },
    // tonlib_api.tl, line 173
    #[serde(rename = "tvm.stackEntryTuple")]
    Tuple { tuple: TvmTuple },
    // tonlib_api.tl, line 174
    #[serde(rename = "tvm.stackEntryList")]
    List { list: TvmList },
    // tonlib_api.tl, line 175
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

    pub fn get_string(&self, index: usize) -> anyhow::Result<String> {
        self.get(index, TvmStack::extract_string)
    }

    pub fn get_i32(&self, index: usize) -> anyhow::Result<i32> {
        self.get(index, TvmStack::extract_i32)
    }

    pub fn get_i64(&self, index: usize) -> anyhow::Result<i64> {
        self.get(index, TvmStack::extract_i64)
    }

    pub fn get_biguint(&self, index: usize) -> anyhow::Result<BigUint> {
        self.get(index, TvmStack::extract_biguint)
    }

    pub fn get_bigint(&self, index: usize) -> anyhow::Result<BigInt> {
        self.get(index, TvmStack::extract_bigint)
    }

    pub fn get_boc(&self, index: usize) -> anyhow::Result<BagOfCells> {
        self.get(index, TvmStack::extract_boc)
    }

    fn get<T>(
        &self,
        index: usize,
        extract: fn(&TvmStackEntry) -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        let maybe_elem = self.elements.get(index);
        match maybe_elem {
            None => Err(anyhow!(
                "Invalid index: {}, total length: {}",
                index,
                self.elements.len()
            )),
            Some(e) => extract(e),
        }
    }

    fn extract_string(e: &TvmStackEntry) -> anyhow::Result<String> {
        match e {
            TvmStackEntry::Number { number } => Ok(number.number.clone()),
            _ => Err(anyhow!("Unsupported conversion to string from {:?}", e)),
        }
    }

    fn extract_i32(e: &TvmStackEntry) -> anyhow::Result<i32> {
        match e {
            TvmStackEntry::Number { number } => {
                let n = number.number.parse::<i32>()?;
                Ok(n)
            }
            _ => Err(anyhow!("Unsupported conversion to i32 from {:?}", e)),
        }
    }

    fn extract_i64(e: &TvmStackEntry) -> anyhow::Result<i64> {
        match e {
            TvmStackEntry::Number { number } => {
                let n = number.number.parse::<i64>()?;
                Ok(n)
            }
            _ => Err(anyhow!("Unsupported conversion to i64 from {:?}", e)),
        }
    }

    fn extract_biguint(e: &TvmStackEntry) -> anyhow::Result<BigUint> {
        match e {
            TvmStackEntry::Number { number } => {
                let n: BigUint = BigUint::from_str(number.number.as_str())?;
                Ok(n)
            }
            _ => Err(anyhow!("Unsupported conversion to i64 from {:?}", e)),
        }
    }

    fn extract_bigint(e: &TvmStackEntry) -> anyhow::Result<BigInt> {
        match e {
            TvmStackEntry::Number { number } => {
                let n: BigInt = BigInt::from_str(number.number.as_str())?;
                Ok(n)
            }
            _ => Err(anyhow!("Unsupported conversion to i64 from {:?}", e)),
        }
    }

    fn extract_boc(e: &TvmStackEntry) -> anyhow::Result<BagOfCells> {
        match e {
            TvmStackEntry::Cell { cell } => {
                let boc = BagOfCells::parse(cell.bytes.as_slice())?;
                Ok(boc)
            }
            _ => Err(anyhow!("Unsupported conversion to BagOfCells from {:?}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tl::stack::{TvmNumber, TvmStack, TvmStackEntry};

    const SERIAL: &str = r#"[{"@type":"tvm.stackEntryNumber","number":{"number":"100500"}}]"#;

    #[test]
    fn serialize_works() {
        let mut stack = TvmStack::new();
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
