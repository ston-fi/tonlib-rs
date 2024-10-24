use std::collections::HashMap;
use std::hash::Hash;

use num_bigint::BigUint;
use num_traits::{One, ToPrimitive};

use super::types::LabelType;
use crate::cell::dict::{KeyReader, ValReader};
use crate::cell::TonCellError::InvalidInput;
use crate::cell::{CellParser, TonCellError};

pub(crate) struct DictParser<K, V> {
    key_len_bits: usize,
    key_reader: KeyReader<K>,
    val_reader: ValReader<V>,
    cur_key_prefix: BigUint, // store leading 1 to determinate len properly
}

impl<K: Eq + Hash, V> DictParser<K, V> {
    pub(crate) fn new(
        key_len_bits: usize,
        key_reader: KeyReader<K>,
        val_reader: ValReader<V>,
    ) -> DictParser<K, V> {
        DictParser {
            key_len_bits,
            key_reader,
            val_reader,
            cur_key_prefix: BigUint::one(),
        }
    }

    pub(crate) fn parse(&mut self, parser: &mut CellParser) -> Result<HashMap<K, V>, TonCellError> {
        // reset state in case of reusing
        self.cur_key_prefix = BigUint::one();

        let mut result = HashMap::new();
        self.parse_impl(parser, &mut result)?;
        Ok(result)
    }

    fn parse_impl(
        &mut self,
        parser: &mut CellParser,
        dst: &mut HashMap<K, V>,
    ) -> Result<(), TonCellError> {
        // will rollback prefix to original value at the end of the function
        let origin_key_prefix_len = self.cur_key_prefix.bits();

        let label_type = self.detect_label_type(parser)?;
        match label_type {
            LabelType::Same => {
                let prefix_val = parser.load_bit()?;
                let prefix_len_len = self.remain_suffix_bit_len();
                let prefix_len = parser.load_uint(prefix_len_len)?;
                let prefix_len_usize = prefix_len.to_usize().ok_or_else(|| {
                    InvalidInput(format!("Failed to convert BigUint to usize: {prefix_len}"))
                })?;
                if prefix_val {
                    self.cur_key_prefix += 1u32;
                    self.cur_key_prefix <<= prefix_len_usize;
                    self.cur_key_prefix -= 1u32;
                } else {
                    self.cur_key_prefix <<= prefix_len_usize;
                }
            }
            LabelType::Short => {
                let prefix_len = parser.load_unary_length()?;
                if prefix_len != 0 {
                    let val = parser.load_uint(prefix_len)?;
                    self.cur_key_prefix <<= prefix_len;
                    self.cur_key_prefix |= val;
                }
            }
            LabelType::Long => {
                let prefix_len_len = self.remain_suffix_bit_len();
                let prefix_len = parser.load_uint(prefix_len_len)?;
                let prefix_len_usize = prefix_len.to_usize().ok_or_else(|| {
                    InvalidInput(format!("Failed to convert BigUint to usize: {prefix_len}"))
                })?;
                if prefix_len_len != 0 {
                    let val = parser.load_uint(prefix_len_usize)?;
                    self.cur_key_prefix <<= prefix_len_usize;
                    self.cur_key_prefix |= val;
                }
            }
        }
        if self.cur_key_prefix.bits() as usize == (self.key_len_bits + 1) {
            let mut key = BigUint::one() << self.key_len_bits;
            key ^= &self.cur_key_prefix;
            let user_key = (self.key_reader)(&key)?;
            let user_value = (self.val_reader)(parser)?;
            dst.insert(user_key, user_value);
        } else {
            let left_ref = parser.next_reference()?;
            self.cur_key_prefix <<= 1;
            self.parse_impl(&mut left_ref.parser(), dst)?;

            let right_ref = parser.next_reference()?;
            self.cur_key_prefix += BigUint::one();
            self.parse_impl(&mut right_ref.parser(), dst)?;
        }
        self.cur_key_prefix >>= self.cur_key_prefix.bits() - origin_key_prefix_len;
        Ok(())
    }

    fn detect_label_type(&self, parser: &mut CellParser) -> Result<LabelType, TonCellError> {
        let label = if parser.load_bit()? {
            if parser.load_bit()? {
                LabelType::Same
            } else {
                LabelType::Long
            }
        } else {
            LabelType::Short
        };
        Ok(label)
    }

    fn remain_suffix_bit_len(&self) -> usize {
        // add 2 because cur_prefix contains leading bit
        let prefix_len_left = self.key_len_bits - self.cur_key_prefix.bits() as usize + 2;
        (prefix_len_left as f32).log2().ceil() as usize
    }
}
