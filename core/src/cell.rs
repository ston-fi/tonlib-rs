use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use std::ops::Deref;
use std::sync::Arc;
use std::{fmt, io};

pub use bag_of_cells::*;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use bitstream_io::{BigEndian, BitWrite, BitWriter};
pub use builder::*;
pub use error::*;
use hmac::digest::Digest;
use lazy_static::lazy_static;
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
mod builder;

mod cell_type;
pub mod dict;
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
    original_data_bit_len: usize,
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
    let d1 = get_refs_descriptor(cell_type, refs, level_mask.apply(level).mask())?;
    let d2 = get_bits_descriptor(original_data_bit_len)?;

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
            bit_len,
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

/// Calculates d1 descriptor for cell
/// See https://docs.ton.org/tvm.pdf 3.1.4 for details
fn get_refs_descriptor(
    cell_type: CellType,
    references: &[ArcCell],
    level_mask: u32,
) -> Result<u8, TonCellError> {
    if references.len() > MAX_CELL_REFERENCES {
        Err(TonCellError::InvalidCellData(
            "Cell should not contain more than 4 references".to_string(),
        ))
    } else if level_mask > MAX_LEVEL_MASK {
        Err(TonCellError::InvalidCellData(
            "Cell level mask can not be higher than 3".to_string(),
        ))
    } else {
        let cell_type_var = (cell_type != CellType::Ordinary) as u8;
        let d1 = references.len() as u8 + 8 * cell_type_var + level_mask as u8 * 32;
        Ok(d1)
    }
}

/// Calculates d2 descriptor for cell
/// See https://docs.ton.org/tvm.pdf 3.1.4 for details
fn get_bits_descriptor(bit_len: usize) -> Result<u8, TonCellError> {
    if bit_len > MAX_CELL_BITS {
        Err(TonCellError::InvalidCellData(
            "Cell data length should not contain more than 1023 bits".to_string(),
        ))
    } else {
        let d2 = (bit_len / 8 + (bit_len + 7) / 8) as u8;
        Ok(d2)
    }
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
    use std::sync::Arc;

    use super::cell_type::CellType;
    use super::{get_bits_descriptor, get_refs_descriptor, Cell};
    use crate::cell::CellBuilder;

    #[test]
    fn default_cell() {
        let result = Cell::default();

        let expected = Cell::new(vec![], 0, vec![], false).unwrap();

        assert_eq!(result, expected)
    }

    #[test]
    fn d1_descriptor_test() {
        let empty_cell = Arc::new(CellBuilder::new().build().unwrap());

        let r1 = get_refs_descriptor(CellType::Ordinary, &[], 0).unwrap();
        assert_eq!(r1, 0);

        let r2 = get_refs_descriptor(CellType::Ordinary, &[], 4).is_err();
        assert!(r2);

        let r3 = get_refs_descriptor(CellType::Ordinary, &[empty_cell.clone()], 3).unwrap();
        assert_eq!(r3, 97);

        let r4 =
            get_refs_descriptor(CellType::Ordinary, vec![empty_cell; 5].as_slice(), 3).is_err();
        assert!(r4);
    }

    #[test]
    fn d2_descriptor_test() {
        let r1 = get_bits_descriptor(0).unwrap();
        assert_eq!(r1, 0);

        let r2 = get_bits_descriptor(1023).unwrap();
        assert_eq!(r2, 255);

        let r3 = get_bits_descriptor(1024).is_err();
        assert!(r3)
    }
}
