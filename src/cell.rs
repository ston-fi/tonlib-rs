mod bag_of_cells;
mod builder;
mod error;
mod parser;
mod raw;
mod state_init;

pub use bag_of_cells::*;
pub use builder::*;
pub use error::*;
pub use parser::*;
pub use raw::*;
pub use state_init::*;

use std::{collections::HashMap, io::Cursor, ops::Deref, sync::Arc};

use bitstream_io::{BigEndian, BitReader, BitWrite, BitWriter};
use num_bigint::BigInt;
use num_traits::{Num, ToPrimitive};
use sha2::{Digest, Sha256};

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub struct Cell {
    pub data: Vec<u8>,
    pub bit_len: usize,
    pub references: Vec<Arc<Cell>>,
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

    pub fn parse_fully<F, T>(&self, parse: F) -> Result<T, TonCellError>
    where
        F: FnOnce(&mut CellParser) -> Result<T, TonCellError>,
    {
        let mut reader = self.parser();
        let res = parse(&mut reader);
        reader.ensure_empty()?;
        res
    }

    pub fn parse<F, T>(&self, parse: F) -> Result<T, TonCellError>
    where
        F: FnOnce(&mut CellParser) -> Result<T, TonCellError>,
    {
        let mut reader = self.parser();
        let res = parse(&mut reader);
        res
    }

    pub fn reference(&self, idx: usize) -> Result<&Arc<Cell>, TonCellError> {
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
        return max_level;
    }

    fn get_max_depth(&self) -> usize {
        let mut max_depth = 0;
        if self.references.len() > 0 {
            for k in &self.references {
                let depth = k.get_max_depth();
                if depth > max_depth {
                    max_depth = depth;
                }
            }
            max_depth = max_depth + 1;
        }
        return max_depth;
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
            let l = last_byte | (1 << 8 - rest_bits - 1);
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
        Ok(result?)
    }

    pub fn cell_hash(&self) -> Result<Vec<u8>, TonCellError> {
        let mut hasher: Sha256 = Sha256::new();
        hasher.update(self.get_repr()?.as_slice());
        Ok(hasher.finalize()[..].to_vec())
    }

    pub fn cell_hash_base64(&self) -> Result<String, TonCellError> {
        Ok(base64::encode(self.cell_hash()?))
    }

    ///Snake format when we store part of the data in a cell and the rest of the data in the first child cell (and so recursively).
    ///
    ///Must be prefixed with 0x00 byte.
    ///### TL-B scheme:
    ///
    /// ``` tail#_ {bn:#} b:(bits bn) = SnakeData ~0; ```
    ///
    /// ``` cons#_ {bn:#} {n:#} b:(bits bn) next:^(SnakeData ~n) = SnakeData ~(n + 1); ```
    pub fn load_snake_formatted_dict(&self) -> Result<HashMap<String, String>, TonCellError> {
        let map = self.load_dict(|cell| {
            let mut buffer = Vec::new();
            cell.reference(0)?.parse_snake_data(&mut buffer)?;
            Ok(buffer.to_vec())
        })?;
        Ok(map)
    }

    pub fn load_snake_formatted_string(&self) -> Result<String, TonCellError> {
        let mut cell: &Cell = self;
        let mut first_cell = true;
        let mut uri = String::new();
        loop {
            let parsed_cell = if first_cell {
                std::str::from_utf8(&cell.data[1..])
                    .map_boc_deserialization_error()?
                    .to_string()
            } else {
                std::str::from_utf8(&cell.data)
                    .map_boc_deserialization_error()?
                    .to_string()
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
            let mut data = reader.load_bytes(reader.remaining_bytes())?;
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

    pub fn load_dict<F>(&self, extractor: F) -> Result<HashMap<String, String>, TonCellError>
    where
        F: Fn(&Cell) -> Result<Vec<u8>, TonCellError>,
    {
        let mut map: HashMap<String, String> = HashMap::new();
        self.parse_dict("".to_string(), 256, &mut map, &extractor)?;
        Ok(map)
    }

    ///Port of https://github.com/ton-community/ton/blob/17b7e9e6154131399d57507b0c4a178752342fd8/src/boc/dict/parseDict.ts#L55
    fn parse_dict<F>(
        &self,
        prefix: String,
        n: usize,
        map: &mut HashMap<String, String>,
        extractor: &F,
    ) -> Result<(), TonCellError>
    where
        F: Fn(&Cell) -> Result<Vec<u8>, TonCellError>,
    {
        let mut reader = self.parser();

        let lb0 = reader.load_bit()?;
        let mut pp = prefix;
        let prefix_length;
        if !lb0 {
            // Short label detected
            prefix_length = reader.load_unary_length()?;
            // Read prefix
            for _i in 0..prefix_length {
                pp = format!("{}{}", pp, if reader.load_bit()? { '1' } else { '0' });
            }
        } else {
            let lb1 = reader.load_bit()?;
            if !lb1 {
                // Long label detected
                prefix_length = reader
                    .load_uint(((n + 1) as f32).log2().ceil() as usize)?
                    .to_usize()
                    .unwrap();
                for _i in 0..prefix_length {
                    pp = format!("{}{}", pp, if reader.load_bit()? { '1' } else { '0' });
                }
            } else {
                // Same label detected
                let bit = reader.load_bit()?;
                prefix_length = reader
                    .load_uint(((n + 1) as f32).log2().ceil() as usize)?
                    .to_usize()
                    .unwrap();
                for _i in 0..prefix_length {
                    pp = format!("{}{}", pp, if bit { '1' } else { '0' });
                }
            }
        }

        if n - prefix_length == 0 {
            let r = extractor(&self)?;
            let data = String::from_utf8(r).map_cell_parser_error()?;
            map.insert(
                BigInt::from_str_radix(pp.as_str(), 2)
                    .map_cell_parser_error()?
                    .to_str_radix(10),
                data,
            );
        } else {
            // NOTE: Left and right branches are implicitly contain prefixes '0' and '1'
            let left = self.reference(0)?;
            let right = self.reference(1)?;

            left.parse_dict(
                format!("{}{}", pp, 0),
                n - prefix_length - 1,
                map,
                extractor,
            )?;
            right.parse_dict(
                format!("{}{}", pp, 1),
                n - prefix_length - 1,
                map,
                extractor,
            )?;
        }
        Ok(())
    }
}
