use std::collections::HashMap;
use std::sync::Arc;

use num_bigint::BigUint;
use num_traits::{One, Zero};

use super::leading_bit_utils::{
    add_leading_bit, all_bits_same, common_prefix_len, remove_leading_bit,
};
use super::types::LabelType;
use crate::cell::dict::ValWriter;
use crate::cell::TonCellError::InvalidInput;
use crate::cell::{Cell, CellBuilder, TonCellError};

pub(crate) struct DictBuilder<V> {
    value_writer: ValWriter<V>,
    data: HashMap<BigUint, V>,
    keys_sorted: Vec<BigUint>, // keys contain 1 extra leading bit set to 1
    key_len_bits_left: usize,
}

impl<V> DictBuilder<V> {
    pub(crate) fn new<K>(
        key_len_bits: usize,
        value_writer: ValWriter<V>,
        data: HashMap<K, V>,
    ) -> Result<Self, TonCellError>
    where
        BigUint: From<K>,
    {
        let prepared_data = update_keys(key_len_bits, data)?;
        let mut keys: Vec<_> = prepared_data.keys().cloned().collect();
        keys.sort();

        let builder = DictBuilder {
            value_writer,
            data: prepared_data,
            keys_sorted: keys,
            key_len_bits_left: key_len_bits,
        };
        Ok(builder)
    }

    pub(crate) fn build(mut self) -> Result<Cell, TonCellError> {
        let mut builder = CellBuilder::new();
        if self.data.is_empty() {
            return builder.build();
        }
        let keys = self.keys_sorted.iter().cloned().enumerate().collect();
        self.fill_cell(&mut builder, keys)?;
        builder.build()
    }

    // keys: Vec<(original_key_position, remaining_key_part)>
    fn fill_cell(
        &mut self,
        builder: &mut CellBuilder,
        keys: Vec<(usize, BigUint)>,
    ) -> Result<(), TonCellError> {
        if keys.len() == 1 {
            let (orig_key_pos, remaining_key) = &keys[0];
            return self.store_leaf(builder, *orig_key_pos, remaining_key);
        }

        // will restore it at the end
        let key_len_bits_left_original = self.key_len_bits_left;

        let key = &keys[0].1;
        let key_len = key.bits() as usize; // includes leading bit

        let common_prefix_len = common_prefix_len(key, &keys.last().unwrap().1);
        let label = {
            let ignored_suffix_len = key_len - common_prefix_len - 1;
            key >> ignored_suffix_len
        };
        self.store_label(builder, &label)?;

        let mut left_keys = Vec::with_capacity(keys.len() / 2);
        let mut right_keys = Vec::with_capacity(keys.len() / 2);

        let new_key_len = key_len - common_prefix_len - 1;
        let new_key_mask = (BigUint::one() << new_key_len) - 1u32;
        for (pos, key) in keys {
            let new_key = key & new_key_mask.clone();
            let is_right = new_key.bits() as usize == new_key_len;
            let new_key_internal = add_leading_bit(&new_key, new_key_len - 1);
            if is_right {
                right_keys.push((pos, new_key_internal));
            } else {
                left_keys.push((pos, new_key_internal));
            }
        }

        self.key_len_bits_left -= common_prefix_len + 1; // branch consumes 1 more bit
        let mut left_builder = CellBuilder::new();
        self.fill_cell(&mut left_builder, left_keys)?;
        builder.store_reference(&Arc::new(left_builder.build()?))?;

        let mut right_builder = CellBuilder::new();
        self.fill_cell(&mut right_builder, right_keys)?;
        builder.store_reference(&Arc::new(right_builder.build()?))?;

        self.key_len_bits_left = key_len_bits_left_original;
        Ok(())
    }

    fn store_leaf(
        &mut self,
        builder: &mut CellBuilder,
        orig_key_pos: usize,
        label: &BigUint,
    ) -> Result<(), TonCellError> {
        self.store_label(builder, label)?;
        let origin_key = &self.keys_sorted[orig_key_pos];
        let value = self.data.remove(origin_key).unwrap();
        (self.value_writer)(builder, value)?;
        Ok(())
    }

    // expect label with leading one
    fn store_label(&self, builder: &mut CellBuilder, label: &BigUint) -> Result<(), TonCellError> {
        assert!(label.bits() > 0);
        if label.is_one() {
            // it's leading bit => label_type == short, len == 0 => store [false, false]
            builder.store_u8(2, 0)?;
            return Ok(());
        }
        let all_bits_same = all_bits_same(label);

        let label_len = label.bits() as usize - 1;
        let label_len_len = (self.key_len_bits_left as f32 + 1.0).log2().ceil() as usize;
        let fair_label = remove_leading_bit(label);
        let same_label_len = if all_bits_same {
            3 + label_len_len
        } else {
            usize::MAX
        };
        let short_label_len = 2 + label_len * 2;
        let long_label_len = 2 + label_len_len + label_len;

        let mut label_type = LabelType::Short;
        if long_label_len < short_label_len {
            label_type = LabelType::Long;
        }
        if same_label_len < short_label_len {
            label_type = LabelType::Same;
        }
        match label_type {
            LabelType::Same => {
                builder.store_bit(true)?;
                builder.store_bit(true)?;
                builder.store_bit(!fair_label.is_zero())?;
                builder.store_u32(label_len_len, label_len as u32)?;
            }
            LabelType::Short => {
                builder.store_bit(false)?;
                for _ in 0..label_len {
                    builder.store_bit(true)?;
                }
                builder.store_bit(false)?;
                builder.store_uint(label_len, &fair_label)?;
            }
            LabelType::Long => {
                builder.store_bit(true)?;
                builder.store_bit(false)?;
                builder.store_u32(label_len_len, label_len as u32)?;
                builder.store_uint(label_len, &fair_label)?;
            }
        }
        Ok(())
    }
}

fn update_keys<K, V>(
    key_len_bits: usize,
    data: HashMap<K, V>,
) -> Result<HashMap<BigUint, V>, TonCellError>
where
    BigUint: From<K>,
{
    let mut result = HashMap::new();

    for (key, val) in data {
        let key_big = BigUint::from(key);
        let received_len_bits = key_big.bits();
        if received_len_bits as usize > key_len_bits {
            let msg = format!(
                "Invalid key length: Expected max_len={key_len_bits}, got len={received_len_bits}"
            );
            return Err(InvalidInput(msg));
        }
        // add leading bit to maintain proper bits length
        let internal_key = add_leading_bit(&key_big, key_len_bits);
        result.insert(internal_key, val);
    }
    Ok(result)
}
