use core::fmt;
use std::collections::HashMap;
use std::hash::Hash;
use std::io::Cursor;
use std::ops::Deref;
use std::sync::Arc;

pub use bag_of_cells::*;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use bit_string::*;
use bitstream_io::{BigEndian, BitReader, BitWrite, BitWriter};
pub use builder::*;
pub use dict_loader::*;
pub use error::*;
use num_bigint::BigUint;
use num_traits::{One, ToPrimitive};
pub use parser::*;
pub use raw::*;
use sha2::{Digest, Sha256};
pub use slice::*;
pub use state_init::*;
pub use util::*;

mod bag_of_cells;
mod bit_string;
mod builder;
mod dict_loader;
mod error;
mod parser;
mod raw;
mod slice;
mod state_init;
mod util;

pub type ArcCell = Arc<Cell>;

#[derive(PartialEq, Eq, Clone, Hash)]
pub struct Cell {
    pub data: Vec<u8>,
    pub bit_len: usize,
    pub references: Vec<ArcCell>,
}

impl Cell {
    pub fn parser(&self) -> CellParser {
        let bit_len = self.bit_len;
        let cursor = Cursor::new(&self.data);
        let bit_reader: BitReader<Cursor<&Vec<u8>>, BigEndian> =
            BitReader::endian(cursor, BigEndian);

        CellParser {
            bit_len,
            bit_reader,
        }
    }

    #[allow(clippy::let_and_return)]
    pub fn parse_fully<F, T>(&self, parse: F) -> Result<T, TonCellError>
    where
        F: FnOnce(&mut CellParser) -> Result<T, TonCellError>,
    {
        let mut reader = self.parser();
        let res = parse(&mut reader);
        reader.ensure_empty()?;
        res
    }

    #[allow(clippy::let_and_return)]
    pub fn parse<F, T>(&self, parse: F) -> Result<T, TonCellError>
    where
        F: FnOnce(&mut CellParser) -> Result<T, TonCellError>,
    {
        let mut reader = self.parser();
        let res = parse(&mut reader);
        res
    }

    pub fn reference(&self, idx: usize) -> Result<&ArcCell, TonCellError> {
        self.references.get(idx).ok_or(TonCellError::InvalidIndex {
            idx,
            ref_count: self.references.len(),
        })
    }

    pub fn get_max_level(&self) -> u8 {
        //TODO level calculation differ for exotic cells
        let mut max_level = 0;
        for k in &self.references {
            let level = k.get_max_level();
            if level > max_level {
                max_level = level;
            }
        }
        max_level
    }

    fn get_max_depth(&self) -> usize {
        let mut max_depth = 0;
        if !self.references.is_empty() {
            for k in &self.references {
                let depth = k.get_max_depth();
                if depth > max_depth {
                    max_depth = depth;
                }
            }
            max_depth += 1;
        }
        max_depth
    }

    fn get_refs_descriptor(&self) -> u8 {
        self.references.len() as u8 + self.get_max_level() * 32
    }

    fn get_bits_descriptor(&self) -> u8 {
        let rest_bits = self.bit_len % 8;
        let full_bytes = rest_bits == 0;
        self.data.len() as u8 * 2 - if full_bytes { 0 } else { 1 } //subtract 1 if the last byte is not full
    }

    pub fn get_repr(&self) -> Result<Vec<u8>, TonCellError> {
        let data_len = self.data.len();
        let rest_bits = self.bit_len % 8;
        let full_bytes = rest_bits == 0;
        let mut writer = BitWriter::endian(Vec::new(), BigEndian);
        let val = self.get_refs_descriptor();
        writer.write(8, val).map_boc_serialization_error()?;
        writer
            .write(8, self.get_bits_descriptor())
            .map_boc_serialization_error()?;
        if !full_bytes {
            writer
                .write_bytes(&self.data[..data_len - 1])
                .map_boc_serialization_error()?;
            let last_byte = self.data[data_len - 1];
            let l = last_byte | 1 << (8 - rest_bits - 1);
            writer.write(8, l).map_boc_serialization_error()?;
        } else {
            writer
                .write_bytes(&self.data)
                .map_boc_serialization_error()?;
        }

        for r in &self.references {
            writer
                .write(8, (r.get_max_depth() / 256) as u8)
                .map_boc_serialization_error()?;
            writer
                .write(8, (r.get_max_depth() % 256) as u8)
                .map_boc_serialization_error()?;
        }
        for r in &self.references {
            writer
                .write_bytes(&r.cell_hash()?)
                .map_boc_serialization_error()?;
        }
        let result = writer
            .writer()
            .ok_or_else(|| TonCellError::cell_builder_error("Stream is not byte-aligned"))
            .map(|b| b.to_vec());
        result
    }

    pub fn cell_hash(&self) -> Result<Vec<u8>, TonCellError> {
        let mut hasher: Sha256 = Sha256::new();
        hasher.update(self.get_repr()?.as_slice());
        Ok(hasher.finalize()[..].to_vec())
    }

    pub fn cell_hash_base64(&self) -> Result<String, TonCellError> {
        Ok(URL_SAFE_NO_PAD.encode(self.cell_hash()?))
    }

    ///Snake format when we store part of the data in a cell and the rest of the data in the first child cell (and so recursively).
    ///
    ///Must be prefixed with 0x00 byte.
    ///### TL-B scheme:
    ///
    /// ``` tail#_ {bn:#} b:(bits bn) = SnakeData ~0; ```
    ///
    /// ``` cons#_ {bn:#} {n:#} b:(bits bn) next:^(SnakeData ~n) = SnakeData ~(n + 1); ```
    pub fn load_snake_formatted_dict(&self) -> Result<HashMap<[u8; 32], Vec<u8>>, TonCellError> {
        //todo: #79 key in hashmap must be [u8;32]
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
        let mut cell: &Cell = self;
        let mut first_cell = true;
        loop {
            let mut reader = cell.parser();
            let first_byte = reader.load_uint(8)?.to_u32().unwrap();

            if first_cell && first_byte != 0 {
                return Err(TonCellError::boc_deserialization_error(
                    "Invalid snake format",
                ));
            }
            let remaining_bytes = reader.remaining_bytes();
            let mut data = reader.load_bytes(remaining_bytes)?;
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
}

impl fmt::Debug for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Cell{{ data: [{}], bit_len: {}, references: [\n",
            self.data
                .iter()
                .map(|&byte| format!("{:02X}", byte))
                .collect::<Vec<_>>()
                .join(""),
            self.bit_len,
        )?;

        for reference in &self.references {
            writeln!(
                f,
                "    {}\n",
                format!("{:?}", reference).replace('\n', "\n    ")
            )?;
        }

        write!(f, "] }}")
    }
}
