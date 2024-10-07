use std::collections::HashMap;
use std::ops::BitXorAssign;
use std::sync::Arc;

use bitvec::prelude::{BitVec, Msb0};
use num_bigint::BigUint;

use crate::cell::{Cell, CellBuilder, TonCellError};

pub type ValueWriter<V> = fn(&mut CellBuilder, &V) -> Result<(), TonCellError>;

pub(crate) fn serialize_dict<K, V>(
    data: HashMap<K, V>,
    key_len_bits: usize,
    value_writer: ValueWriter<V>,
) -> Result<Cell, TonCellError>
where
    BigUint: From<K>,
{
    let data_big_keys = data
        .into_iter()
        .map(|(k, v)| (BigUint::from(k), v))
        .collect();
    let root = build_tree(data_big_keys, key_len_bits);
    let mut builder = CellBuilder::new();
    if let Some(root) = root {
        store_node(&root, key_len_bits, &mut builder, value_writer)?;
    }
    builder.build()
}

fn store_node<V>(
    node: &TreeNode<V>,
    key_len_bits: usize,
    builder: &mut CellBuilder,
    value_writer: ValueWriter<V>,
) -> Result<(), TonCellError> {
    store_label(builder, &node.prefix, key_len_bits)?;

    match &node.data {
        Data::Edge { left, right } => {
            let mut left_builder = CellBuilder::new();
            let mut right_builder = CellBuilder::new();
            // branch implicitly contains 0/1 bit, so we need to subtract 1 more bit
            let sub_key_len = key_len_bits - node.prefix.len() - 1;
            store_node(left, sub_key_len, &mut left_builder, value_writer)?;
            store_node(right, sub_key_len, &mut right_builder, value_writer)?;
            builder.store_reference(&Arc::new(left_builder.build()?))?;
            builder.store_reference(&Arc::new(right_builder.build()?))?;
        }
        Data::Leaf(val) => value_writer(builder, val)?,
    }
    Ok(())
}

fn store_label(
    builder: &mut CellBuilder,
    prefix: &BitVecBE,
    key_len_bits: usize,
) -> Result<(), TonCellError> {
    let prefix_len = prefix.len();
    let prefix_zero_count = prefix.count_zeros();
    let prefix_stored_len = (key_len_bits as f32 + 1.0).log2().ceil() as usize;

    let short_label_len = 2 + prefix_len * 2;
    let long_label_len = 2 + prefix_stored_len + prefix_len;
    let same_label_len = if prefix_zero_count == prefix_len || prefix_zero_count == 0 {
        3 + prefix_stored_len
    } else {
        usize::MAX
    };

    enum LabelType {
        Short,
        Long,
        Same,
    }

    let mut label_type = LabelType::Short;
    if long_label_len < short_label_len {
        label_type = LabelType::Long;
    }
    if same_label_len < short_label_len {
        label_type = LabelType::Same;
    }

    match label_type {
        LabelType::Short => {
            builder.store_bit(false)?;
            for _ in 0..prefix_len {
                builder.store_bit(true)?;
            }
            builder.store_bit(false)?;
            for bit in prefix.iter() {
                builder.store_bit(*bit)?;
            }
        }
        LabelType::Long => {
            builder.store_bit(true)?;
            builder.store_bit(false)?;
            builder.store_u32(prefix_stored_len, prefix_len as u32)?;
            for bit in prefix.iter() {
                builder.store_bit(*bit)?;
            }
        }
        LabelType::Same => {
            builder.store_bit(true)?;
            builder.store_bit(true)?;
            builder.store_bit(prefix[0])?;
            builder.store_u32(prefix_stored_len, prefix_len as u32)?;
        }
    }
    Ok(())
}

type BitVecBE = BitVec<u32, Msb0>;

struct TreeNode<V> {
    prefix: BitVecBE, // common subtree prefix
    data: Data<V>,
}

enum Data<V> {
    Edge {
        left: Box<TreeNode<V>>,  // 0-prefix subtree
        right: Box<TreeNode<V>>, // 1-prefix subtree
    },
    Leaf(V),
}

impl<V> TreeNode<V> {
    fn new(prefix: BitVecBE, data: Data<V>) -> Self {
        TreeNode { prefix, data }
    }

    fn build(
        keys: Vec<BitVecBE>,
        origin_keys: Vec<BigUint>,
        data: &mut HashMap<BigUint, V>,
    ) -> Self {
        if keys.len() == 1 {
            let value = data.remove(&origin_keys[0]).unwrap();
            return TreeNode::new(keys[0].clone(), Data::Leaf(value));
        }
        let common_prefix_len = calc_common_prefix_len(&keys[0], keys.last().unwrap());
        let cur_prefix = keys[0][..common_prefix_len].into();

        let mut left_keys = Vec::with_capacity(keys.len() / 2);
        let mut left_origin_keys = Vec::with_capacity(keys.len() / 2);
        let mut right_keys = Vec::with_capacity(keys.len() / 2);
        let mut right_origin_keys = Vec::with_capacity(keys.len() / 2);

        for (key, origin_key) in keys.into_iter().zip(origin_keys.into_iter()) {
            let is_right = key[common_prefix_len];
            let new_key: BitVecBE = key[common_prefix_len + 1..].into();
            if is_right {
                right_keys.push(new_key);
                right_origin_keys.push(origin_key);
            } else {
                left_keys.push(new_key);
                left_origin_keys.push(origin_key);
            }
        }

        TreeNode::new(
            cur_prefix,
            Data::Edge {
                left: Box::new(TreeNode::build(left_keys, left_origin_keys, data)),
                right: Box::new(TreeNode::build(right_keys, right_origin_keys, data)),
            },
        )
    }
}

#[allow(dead_code)]
fn traverse<V, F>(node: &TreeNode<V>, func: &mut F, prev_prefix: &mut BitVecBE)
where
    F: FnMut(&TreeNode<V>, &BitVecBE),
{
    func(node, prev_prefix);
    if let Data::Edge { left, right } = &node.data {
        let init_len = prev_prefix.len();

        prev_prefix.extend(node.prefix.iter());
        prev_prefix.push(false);
        traverse(left, func, prev_prefix);
        *prev_prefix = prev_prefix[..init_len].into(); // backtrack

        prev_prefix.extend(node.prefix.iter());
        prev_prefix.push(true);
        traverse(right, func, prev_prefix);
        *prev_prefix = prev_prefix[..init_len].into(); // backtrack
    }
}

fn build_tree<V>(mut data: HashMap<BigUint, V>, key_len_bits: usize) -> Option<TreeNode<V>> {
    if data.is_empty() {
        return None;
    }
    let mut origin_keys = data.keys().cloned().collect::<Vec<_>>();
    origin_keys.sort();
    let keys: Vec<_> = origin_keys
        .iter()
        .map(|k| {
            let mut val = BitVecBE::from_vec(k.to_u32_digits());
            if key_len_bits > val.len() {
                let padding = key_len_bits - val.len();
                val.extend(vec![false; padding]);
                val.shift_right(padding);
            }
            BitVecBE::from(&val[val.len() - key_len_bits..])
        })
        .collect();
    Some(TreeNode::build(keys.clone(), origin_keys, &mut data))
}

fn calc_common_prefix_len(a: &BitVecBE, b: &BitVecBE) -> usize {
    let mut res = a.clone();
    res.bitxor_assign(b);
    res.leading_zeros()
}

#[cfg(test)]
mod tests {
    use tokio_test::assert_ok;

    use super::*;
    use crate::cell::{key_extractor_u8, value_extractor_uint, BagOfCells, GenericDictLoader};

    #[test]
    fn test_build_uint_key() {
        let data = HashMap::from([
            (BigUint::from(0x1212u32), 89),
            (BigUint::from(0x1111u32), 16),
            (BigUint::from(307u32), 42),
        ]);

        let root = build_tree(data.clone(), 32);
        assert!(root.is_some());
        let root = root.unwrap();
        assert_eq!(root.prefix, BitVecBE::from_slice(&[0])[..19]);
        if let Data::Leaf(_) = &root.data {}
        let (left, right) = match root.data {
            Data::Edge { left, right } => (left, right),
            _ => {
                assert!(false, "Expected edge, got leaf");
                unreachable!()
            }
        };
        assert_eq!(
            left.prefix,
            BitVecBE::from_slice(&[0b0000100110011])[32 - 12..]
        );
        match left.data {
            Data::Leaf(val) => assert_eq!(val, 42),
            _ => assert!(false, "Expected leaf, got edge"),
        };

        assert_eq!(right.prefix, BitVecBE::from_slice(&[0b100])[32 - 2..]);
        let (left, right) = match right.data {
            Data::Edge { left, right } => (left, right),
            _ => {
                assert!(false, "Expected edge, got leaf");
                unreachable!()
            }
        };
        assert_eq!(left.prefix, BitVecBE::from_slice(&[0b0100010001])[32 - 9..]);
        match left.data {
            Data::Leaf(val) => assert_eq!(val, 16),
            _ => assert!(false, "Expected leaf, got edge"),
        };

        assert_eq!(
            right.prefix,
            BitVecBE::from_slice(&[0b1000010010])[32 - 9..]
        );
        match left.data {
            Data::Leaf(val) => assert_eq!(val, 16),
            _ => assert!(false, "Expected leaf, got edge"),
        };
    }

    #[test]
    fn test_traverse() {
        let data = HashMap::from([
            (BigUint::from(4626u32), 89),
            (BigUint::from(4369u32), 16),
            (BigUint::from(307u32), 42),
            (BigUint::from(86u32), 57),
            (BigUint::from(4660u32), 73),
        ]);

        let root = build_tree(data.clone(), 256).unwrap();

        let mut traversed_data = HashMap::new();
        let mut func = |node: &TreeNode<i32> /* Type */, prev_prefix: &BitVecBE| {
            let padding = "_".repeat(prev_prefix.len());
            match &node.data {
                Data::Edge { .. } => {
                    println!("{}{:0b}", padding, node.prefix);
                }
                Data::Leaf(val) => {
                    let mut key = prev_prefix.clone();
                    key.extend(node.prefix.iter());
                    key.set_uninitialized(false);
                    let key = BitVecBE::from(&key[key.leading_zeros()..]);
                    let origin_key = BigUint::from_slice(&key.into_vec());
                    println!(
                        "{}{:0b} key_u256: {}, val: {}",
                        padding, node.prefix, origin_key, val
                    );
                    traversed_data.insert(origin_key, *val);
                }
            }
        };
        traverse(&root, &mut func, &mut BitVecBE::new());
        assert_eq!(data, traversed_data);
        // for pretty print
        // assert!(false);
    }

    #[test]
    fn test_serialize_dict() -> Result<(), TonCellError> {
        let data: HashMap<u8, BigUint> = HashMap::from([
            (0, BigUint::from(25965603044000000000u128)),
            (1, BigUint::from(5173255344000000000u64)),
            (2, BigUint::from(344883687000000000u64)),
        ]);

        let writer = |builder: &mut CellBuilder, val: &BigUint| {
            builder.store_uint(150, val)?;
            Ok(())
        };
        let cell = assert_ok!(serialize_dict(data.clone(), 8, writer));

        let dict_loader = GenericDictLoader::new(key_extractor_u8, value_extractor_uint, 8);
        let parsed_data = cell.load_generic_dict(&dict_loader)?;
        assert_eq!(data, parsed_data);

        let bc_boc_b64 = "te6cckEBBgEAWgABGccNPKUADZm5MepOjMABAgHNAgMCASAEBQAnQAAAAAAAAAAAAAABMlF4tR2RgCAAJgAAAAAAAAAAAAABaFhaZZhr6AAAJgAAAAAAAAAAAAAAR8sYU4eC4AA1PIC5";
        let bc_cell = BagOfCells::parse_base64(bc_boc_b64)?;
        let bc_cell = bc_cell.single_root()?.references[0].clone();
        assert_eq!(cell, *bc_cell);
        Ok(())
    }
}
