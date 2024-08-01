use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use std::ops::Deref;
use std::sync::Arc;
use std::{fmt, io};

pub use bag_of_cells::*;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use bit_string::*;
use bitstream_io::{BigEndian, BitWrite, BitWriter};
pub use builder::*;
pub use dict_loader::*;
pub use error::*;
use hmac::digest::Digest;
use lazy_static::lazy_static;
use num_bigint::BigUint;
use num_traits::{One, ToPrimitive};
pub use parser::*;
pub use raw::*;
use sha2::Sha256;
pub use slice::*;
pub use state_init::*;
pub use util::*;

use crate::cell::cell_type::CellType;
use crate::cell::level_mask::LevelMask;
use crate::types::DEFAULT_CELL_HASH;
use crate::TonHash;

mod bag_of_cells;
mod bit_string;
mod builder;

mod cell_type;
mod dict_loader;
mod error;
mod level_mask;
mod parser;
mod raw;
mod raw_boc_from_boc;
mod slice;
mod state_init;
mod util;

const DEPTH_BYTES: usize = 2;
const MAX_LEVEL: u8 = 3;

pub type ArcCell = Arc<Cell>;

pub type SnakeFormattedDict = HashMap<TonHash, Vec<u8>>;

lazy_static! {
    pub static ref EMPTY_CELL: Cell = Cell::default();
    pub static ref EMPTY_ARC_CELL: ArcCell = Arc::new(Cell::default());
}

#[derive(PartialEq, Eq, Clone, Hash)]
pub struct Cell {
    data: Vec<u8>,
    bit_len: usize,
    references: Vec<ArcCell>,
    cell_type: CellType,
    level_mask: LevelMask,
    hashes: [TonHash; 4],
    depths: [u16; 4],
}

impl Cell {
    pub fn new(
        data: Vec<u8>,
        bit_len: usize,
        references: Vec<ArcCell>,
        is_exotic: bool,
    ) -> Result<Self, TonCellError> {
        let cell_type = if is_exotic {
            CellType::determine_exotic_cell_type(&data)?
        } else {
            CellType::Ordinary
        };

        cell_type.validate(&data, bit_len, &references)?;
        let level_mask = cell_type.level_mask(&data, bit_len, &references)?;
        let (hashes, depths) =
            calculate_hashes_and_depths(cell_type, &data, bit_len, &references, level_mask)?;

        let result = Self {
            data,
            bit_len,
            references,
            level_mask,
            cell_type,
            hashes,
            depths,
        };

        Ok(result)
    }

    pub fn parser(&self) -> CellParser {
        CellParser::new(self.bit_len, &self.data, &self.references)
    }

    #[allow(clippy::let_and_return)]
    pub fn parse<F, T>(&self, parse: F) -> Result<T, TonCellError>
    where
        F: FnOnce(&mut CellParser) -> Result<T, TonCellError>,
    {
        let mut parser = self.parser();
        let res = parse(&mut parser);
        res
    }

    pub fn parse_fully<F, T>(&self, parse: F) -> Result<T, TonCellError>
    where
        F: FnOnce(&mut CellParser) -> Result<T, TonCellError>,
    {
        let mut reader = self.parser();
        let res = parse(&mut reader);
        reader.ensure_empty()?;
        res
    }

    pub fn reference(&self, idx: usize) -> Result<&ArcCell, TonCellError> {
        self.references.get(idx).ok_or(TonCellError::InvalidIndex {
            idx,
            ref_count: self.references.len(),
        })
    }

    pub fn data(&self) -> &[u8] {
        self.data.as_slice()
    }

    pub fn bit_len(&self) -> usize {
        self.bit_len
    }

    pub fn references(&self) -> &[ArcCell] {
        self.references.as_slice()
    }

    pub(crate) fn get_level_mask(&self) -> u32 {
        self.level_mask.mask()
    }

    pub fn cell_depth(&self) -> u16 {
        self.get_depth(MAX_LEVEL)
    }

    pub fn get_depth(&self, level: u8) -> u16 {
        self.depths[level.min(3) as usize]
    }

    pub fn cell_hash(&self) -> TonHash {
        self.get_hash(MAX_LEVEL)
    }

    pub fn get_hash(&self, level: u8) -> TonHash {
        self.hashes[level.min(3) as usize]
    }

    pub fn is_exotic(&self) -> bool {
        self.cell_type != CellType::Ordinary
    }

    pub fn cell_hash_base64(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.cell_hash())
    }

    ///Snake format when we store part of the data in a cell and the rest of the data in the first child cell (and so recursively).
    ///
    ///Must be prefixed with 0x00 byte.
    ///### TL-B scheme:
    ///
    /// ``` tail#_ {bn:#} b:(bits bn) = SnakeData ~0; ```
    ///
    /// ``` cons#_ {bn:#} {n:#} b:(bits bn) next:^(SnakeData ~n) = SnakeData ~(n + 1); ```
    pub fn load_snake_formatted_dict(&self) -> Result<SnakeFormattedDict, TonCellError> {
        let dict_loader = GenericDictLoader::new(
            key_extractor_256bit,
            value_extractor_snake_formatted_string,
            256,
        );
        self.load_generic_dict(&dict_loader)
    }

    pub fn load_snake_formatted_string(&self) -> Result<String, TonCellError> {
        let mut cell: &Cell = self;
        let mut first_cell = true;
        let mut uri = String::new();
        loop {
            let parsed_cell = if first_cell {
                String::from_utf8_lossy(&cell.data[1..]).to_string()
            } else {
                String::from_utf8_lossy(&cell.data).to_string()
            };
            uri.push_str(&parsed_cell);
            match cell.references.len() {
                0 => return Ok(uri),
                1 => {
                    cell = cell.references[0].deref();
                    first_cell = false;
                }
                n => {
                    return Err(TonCellError::boc_deserialization_error(format!(
                        "Invalid snake format string: found cell with {} references",
                        n
                    )))
                }
            }
        }
    }

    fn parse_snake_data(&self, buffer: &mut Vec<u8>) -> Result<(), TonCellError> {
        let mut cell = self;
        let mut first_cell = true;
        loop {
            let mut parser = cell.parser();
            if first_cell {
                let first_byte = parser.load_u8(8)?;

                if first_byte != 0 {
                    return Err(TonCellError::boc_deserialization_error(
                        "Invalid snake format",
                    ));
                }
            }
            let remaining_bytes = parser.remaining_bytes();
            let mut data = parser.load_bytes(remaining_bytes)?;
            buffer.append(&mut data);
            match cell.references.len() {
                0 => return Ok(()),
                1 => {
                    cell = cell.references[0].deref();
                    first_cell = false;
                }
                n => {
                    return Err(TonCellError::boc_deserialization_error(format!(
                        "Invalid snake format string: found cell with {} references",
                        n
                    )))
                }
            }
        }
    }

    pub fn load_generic_dict<K, V, L>(&self, dict_loader: &L) -> Result<HashMap<K, V>, TonCellError>
    where
        K: Hash + Eq + Clone,
        L: DictLoader<K, V>,
    {
        let mut map: HashMap<K, V> = HashMap::new();
        self.dict_to_hashmap::<K, V, L>(BitString::new(), &mut map, dict_loader)?;
        Ok(map)
    }

    ///Port of https://github.com/ton-community/ton/blob/17b7e9e6154131399d57507b0c4a178752342fd8/src/boc/dict/parseDict.ts#L55
    fn dict_to_hashmap<K, V, L>(
        &self,
        prefix: BitString,
        map: &mut HashMap<K, V>,
        dict_loader: &L,
    ) -> Result<(), TonCellError>
    where
        K: Hash + Eq,
        L: DictLoader<K, V>,
    {
        let mut parser = self.parser();

        let lb0 = parser.load_bit()?;
        let mut pp = prefix;
        let prefix_length;
        if !lb0 {
            // Short label detected
            prefix_length = parser.load_unary_length()?;
            // Read prefix
            if prefix_length != 0 {
                let val = parser.load_uint(prefix_length)?;
                pp.shl_assign_and_add(prefix_length, val);
            }
        } else {
            let lb1 = parser.load_bit()?;
            if !lb1 {
                // Long label detected
                prefix_length = parser
                    .load_uint(
                        ((dict_loader.key_bit_len() - pp.bit_len() + 1) as f32)
                            .log2()
                            .ceil() as usize,
                    )?
                    .to_usize()
                    .unwrap();
                if prefix_length != 0 {
                    let val = parser.load_uint(prefix_length)?;
                    pp.shl_assign_and_add(prefix_length, val);
                }
            } else {
                // Same label detected
                let bit = parser.load_bit()?;
                prefix_length = parser
                    .load_uint(
                        ((dict_loader.key_bit_len() - pp.bit_len() + 1) as f32)
                            .log2()
                            .ceil() as usize,
                    )?
                    .to_usize()
                    .unwrap();
                if bit {
                    pp.shl_assign_and_fill(prefix_length);
                } else {
                    pp.shl_assign(prefix_length)
                }
            }
        }

        if dict_loader.key_bit_len() - pp.bit_len() == 0 {
            let bytes = pp.get_value_as_bytes();
            let key = dict_loader.extract_key(bytes.as_slice())?;
            let offset = self.bit_len - parser.remaining_bits();
            let cell_slice = CellSlice::new_with_offset(self, offset)?;
            let value = dict_loader.extract_value(&cell_slice)?;
            map.insert(key, value);
        } else {
            // NOTE: Left and right branches are implicitly contain prefixes '0' and '1'
            let left = self.reference(0)?;
            let right = self.reference(1)?;
            pp.shl_assign(1);
            left.dict_to_hashmap(pp.clone(), map, dict_loader)?;
            pp = pp + BigUint::one();
            right.dict_to_hashmap(pp, map, dict_loader)?;
        }
        Ok(())
    }

    pub fn to_arc(self) -> ArcCell {
        Arc::new(self)
    }

    /// It is recommended to use CellParser::next_reference() instead
    #[deprecated]
    pub fn expect_reference_count(&self, expected_refs: usize) -> Result<(), TonCellError> {
        let ref_count = self.references.len();
        if ref_count != expected_refs {
            Err(TonCellError::CellParserError(format!(
                "Cell should contain {} reference cells, actual: {}",
                expected_refs, ref_count
            )))
        } else {
            Ok(())
        }
    }
}

impl Debug for Cell {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let t = match self.cell_type {
            CellType::Ordinary | CellType::Library => 'x',
            CellType::PrunedBranch | CellType::MerkleProof => 'p',
            CellType::MerkleUpdate => 'u',
        };

        // Our completion tag ONLY shows that the last byte is incomplete
        // It does not correspond to real completion tag defined in
        // p1.0.2 of https://docs.ton.org/tvm.pdf for details
        // Null termination of bit-string defined in that document is omitted for clarity
        let completion_tag = if self.bit_len % 8 != 0 { "_" } else { "" };
        writeln!(
            f,
            "Cell {}{{ data: [{}{}]\n, bit_len: {}\n, references: [",
            t,
            self.data
                .iter()
                .map(|&byte| format!("{:02X}", byte))
                .collect::<Vec<_>>()
                .join(""),
            completion_tag,
            self.bit_len,
        )?;

        for reference in &self.references {
            writeln!(
                f,
                "    {}\n",
                format!("{:?}", reference).replace('\n', "\n    ")
            )?;
        }

        write!(
            f,
            "]\n cell_type: {:?}\n level_mask: {:?}\n hashes {:?}\n depths {:?}\n }}",
            self.cell_type,
            self.level_mask,
            self.hashes
                .iter()
                .map(|h| h
                    .iter()
                    .map(|&byte| format!("{:02X}", byte))
                    .collect::<Vec<_>>()
                    .join(""))
                .collect::<Vec<_>>(),
            self.depths
        )
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            data: Default::default(),
            bit_len: Default::default(),
            references: Default::default(),
            cell_type: Default::default(),
            level_mask: Default::default(),
            hashes: [DEFAULT_CELL_HASH; 4],
            depths: Default::default(),
        }
    }
}

fn get_repr_for_data(
    (original_data, original_data_bit_len): (&[u8], usize),
    (data, data_bit_len): (&[u8], usize),
    refs: &[ArcCell],
    level_mask: LevelMask,
    level: u8,
    cell_type: CellType,
) -> Result<Vec<u8>, TonCellError> {
    // Allocate
    let data_len = data.len();
    // descriptors + data + (hash + depth) * refs_count
    let buffer_len = 2 + data_len + (32 + 2) * refs.len();

    let mut writer = BitWriter::endian(Vec::with_capacity(buffer_len), BigEndian);
    let d1 = get_refs_descriptor(cell_type, refs, level_mask.apply(level).mask());
    let d2 = get_bits_descriptor(original_data, original_data_bit_len);

    // Write descriptors
    writer.write(8, d1).map_cell_parser_error()?;
    writer.write(8, d2).map_cell_parser_error()?;
    // Write main data
    write_data(&mut writer, data, data_bit_len).map_cell_parser_error()?;
    // Write ref data
    write_ref_depths(&mut writer, refs, cell_type, level)?;
    write_ref_hashes(&mut writer, refs, cell_type, level)?;

    let result = writer
        .writer()
        .ok_or_else(|| TonCellError::cell_builder_error("Stream for cell repr is not byte-aligned"))
        .map(|b| b.to_vec());

    result
}

/// This function replicates unknown logic of resolving cell data
/// https://github.com/ton-blockchain/ton/blob/24dc184a2ea67f9c47042b4104bbb4d82289fac1/crypto/vm/cells/DataCell.cpp#L214
fn calculate_hashes_and_depths(
    cell_type: CellType,
    data: &[u8],
    bit_len: usize,
    references: &[ArcCell],
    level_mask: LevelMask,
) -> Result<([TonHash; 4], [u16; 4]), TonCellError> {
    let hash_count = if cell_type == CellType::PrunedBranch {
        1
    } else {
        level_mask.hash_count()
    };

    let total_hash_count = level_mask.hash_count();
    let hash_i_offset = total_hash_count - hash_count;

    let mut depths: Vec<u16> = Vec::with_capacity(hash_count);
    let mut hashes: Vec<TonHash> = Vec::with_capacity(hash_count);

    // Iterate through significant levels
    for (hash_i, level_i) in (0..=level_mask.level())
        .filter(|&i| level_mask.is_significant(i))
        .enumerate()
    {
        if hash_i < hash_i_offset {
            continue;
        }

        let (current_data, current_bit_len) = if hash_i == hash_i_offset {
            (data, bit_len)
        } else {
            let previous_hash = hashes
                .get(hash_i - hash_i_offset - 1)
                .ok_or_else(|| TonCellError::InternalError("Can't get right hash".to_owned()))?;
            (previous_hash.as_slice(), 256)
        };

        // Calculate Depth
        let depth = if references.is_empty() {
            0
        } else {
            let max_ref_depth = references.iter().fold(0, |max_depth, reference| {
                let child_depth = cell_type.child_depth(reference, level_i);
                max_depth.max(child_depth)
            });

            max_ref_depth + 1
        };

        // Calculate Hash
        let repr = get_repr_for_data(
            (data, bit_len),
            (current_data, current_bit_len),
            references,
            level_mask,
            level_i,
            cell_type,
        )?;
        let hash = Sha256::new_with_prefix(repr).finalize()[..]
            .try_into()
            .map_err(|error| {
                TonCellError::InternalError(format!(
                    "Can't get [u8; 32] from finalized hash with error: {error}"
                ))
            })?;

        depths.push(depth);
        hashes.push(hash);
    }

    cell_type.resolve_hashes_and_depths(hashes, depths, data, bit_len, level_mask)
}

fn get_refs_descriptor(cell_type: CellType, references: &[ArcCell], level_mask: u32) -> u8 {
    let cell_type_var = (cell_type != CellType::Ordinary) as u8;
    references.len() as u8 + 8 * cell_type_var + level_mask as u8 * 32
}

fn get_bits_descriptor(data: &[u8], bit_len: usize) -> u8 {
    let rest_bits = bit_len % 8;
    let full_bytes = rest_bits == 0;
    data.len() as u8 * 2 - !full_bytes as u8 // subtract 1 if the last byte is not full
}

fn write_data(
    writer: &mut BitWriter<Vec<u8>, BigEndian>,
    data: &[u8],
    bit_len: usize,
) -> Result<(), io::Error> {
    let data_len = data.len();
    let rest_bits = bit_len % 8;
    let full_bytes = rest_bits == 0;

    if !full_bytes {
        writer.write_bytes(&data[..data_len - 1])?;
        let last_byte = data[data_len - 1];
        let l = last_byte | 1 << (8 - rest_bits - 1);
        writer.write(8, l)?;
    } else {
        writer.write_bytes(data)?;
    }

    Ok(())
}

fn write_ref_depths(
    writer: &mut BitWriter<Vec<u8>, BigEndian>,
    refs: &[ArcCell],
    parent_cell_type: CellType,
    level: u8,
) -> Result<(), TonCellError> {
    for reference in refs {
        let child_depth = if matches!(
            parent_cell_type,
            CellType::MerkleProof | CellType::MerkleUpdate
        ) {
            reference.get_depth(level + 1)
        } else {
            reference.get_depth(level)
        };

        writer.write(8, child_depth / 256).map_cell_parser_error()?;
        writer.write(8, child_depth % 256).map_cell_parser_error()?;
    }

    Ok(())
}

fn write_ref_hashes(
    writer: &mut BitWriter<Vec<u8>, BigEndian>,
    refs: &[ArcCell],
    parent_cell_type: CellType,
    level: u8,
) -> Result<(), TonCellError> {
    for reference in refs {
        let child_hash = if matches!(
            parent_cell_type,
            CellType::MerkleProof | CellType::MerkleUpdate
        ) {
            reference.get_hash(level + 1)
        } else {
            reference.get_hash(level)
        };

        writer.write_bytes(&child_hash).map_cell_parser_error()?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::Cell;

    #[test]
    fn default_cell() {
        let result = Cell::default();

        let expected = Cell::new(vec![], 0, vec![], false).unwrap();

        assert_eq!(result, expected)
    }
}
