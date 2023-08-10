use std::sync::Arc;
use std::{
    collections::{HashMap, HashSet},
    io::Cursor,
};

use anyhow::anyhow;
use bitstream_io::{BigEndian, BitReader, BitWrite, BitWriter};
use num_bigint::BigInt;
use num_traits::{Num, ToPrimitive};
use sha2::{Digest, Sha256};

pub use builder::*;
pub use parser::*;

use crate::cell::raw::{RawBagOfCells, RawCell};

mod builder;
mod parser;
mod raw;

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub struct Cell {
    pub data: Vec<u8>,
    pub bit_len: usize,
    pub references: Vec<Arc<Cell>>,
}

impl Cell {
    pub fn parser<'a>(&'a self) -> CellParser {
        let bit_len = self.bit_len;
        let cursor = Cursor::new(&self.data);
        let bit_reader: BitReader<Cursor<&Vec<u8>>, BigEndian> =
            BitReader::endian(cursor, BigEndian);

        CellParser {
            bit_len: bit_len,
            bit_reader: bit_reader,
        }
    }

    pub fn parse_fully<F, T>(&self, parse: F) -> anyhow::Result<T>
    where
        F: FnOnce(&mut CellParser) -> anyhow::Result<T>,
    {
        let mut reader = self.parser();
        let res = parse(&mut reader);
        reader.ensure_empty()?;
        res
    }

    pub fn parse<F, T>(&self, parse: F) -> anyhow::Result<T>
    where
        F: FnOnce(&mut CellParser) -> anyhow::Result<T>,
    {
        let mut reader = self.parser();
        let res = parse(&mut reader);
        res
    }

    pub fn reference(&self, idx: usize) -> anyhow::Result<&Arc<Cell>> {
        self.references.get(idx).ok_or(anyhow!(
            "Invalid index: {}, Cell contains {} references",
            idx,
            self.references.len()
        ))
    }

    fn get_max_level(&self) -> u8 {
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

    pub fn get_repr(&self) -> anyhow::Result<Vec<u8>> {
        let data_len = self.data.len();
        let rest_bits = self.bit_len % 8;
        let full_bytes = rest_bits == 0;
        let mut writer = BitWriter::endian(Vec::new(), BigEndian);
        writer.write(8, self.get_refs_descriptor())?;
        writer.write(8, self.get_bits_descriptor())?;
        if !full_bytes {
            writer.write_bytes(&self.data[..data_len - 1])?;
            let last_byte = self.data[data_len - 1];
            let l = last_byte | (1 << 8 - rest_bits - 1);
            writer.write(8, l)?;
        } else {
            writer.write_bytes(&self.data)?;
        }

        for r in &self.references {
            writer.write(8, (r.get_max_depth() / 256) as u8)?;
            writer.write(8, (r.get_max_depth() % 256) as u8)?;
        }
        for r in &self.references {
            writer.write_bytes(&r.cell_hash()?)?;
        }
        writer
            .writer()
            .ok_or(anyhow!("Stream is not byte-aligned"))
            .map(|b| b.to_vec())
    }

    pub fn cell_hash(&self) -> anyhow::Result<Vec<u8>> {
        let mut hasher: Sha256 = Sha256::new();
        hasher.update(self.get_repr()?.as_slice());
        Ok(hasher.finalize()[..].to_vec())
    }

    ///Snake format when we store part of the data in a cell and the rest of the data in the first child cell (and so recursively).
    ///
    ///Must be prefixed with 0x00 byte.
    ///### TL-B scheme:
    ///
    /// ``` tail#_ {bn:#} b:(bits bn) = SnakeData ~0; ```
    ///
    /// ``` cons#_ {bn:#} {n:#} b:(bits bn) next:^(SnakeData ~n) = SnakeData ~(n + 1); ```
    pub fn load_snake_formatted_dict(&self) -> anyhow::Result<HashMap<String, String>> {
        let map = self.load_dict(|cell| {
            let mut buffer = Vec::new();
            if cell.references.is_empty() {
                cell.parse_snake_data(&mut buffer, true)?;
            } else {
                cell.reference(0)?.parse_snake_data(&mut buffer, true)?;
            }
            Ok(buffer.to_vec())
        })?;
        Ok(map)
    }

    fn parse_snake_data(&self, buffer: &mut Vec<u8>, is_first: bool) -> anyhow::Result<()> {
        let mut reader = self.parser();
        if is_first && reader.load_uint(8)?.to_u32().unwrap() != 0 {
            return Err(anyhow!("Only snake format is supported"));
        }

        let mut data = reader.load_bytes(reader.remaining_bytes())?;
        buffer.append(&mut data);

        if self.references.len() == 1 {
            self.reference(0)?.parse_snake_data(buffer, false)?;
        }

        Ok(())
    }

    pub fn load_dict<F>(&self, extractor: F) -> anyhow::Result<HashMap<String, String>>
    where
        F: Fn(&Cell) -> anyhow::Result<Vec<u8>>,
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
    ) -> anyhow::Result<()>
    where
        F: Fn(&Cell) -> anyhow::Result<Vec<u8>>,
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
            let data = String::from_utf8(r)?;
            map.insert(
                BigInt::from_str_radix(pp.as_str(), 2)?.to_str_radix(10),
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

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub struct BagOfCells {
    pub roots: Vec<Arc<Cell>>,
}

impl BagOfCells {
    pub fn new(roots: &[Arc<Cell>]) -> BagOfCells {
        BagOfCells {
            roots: roots.to_vec(),
        }
    }

    pub fn from_root(root: Cell) -> BagOfCells {
        let arc = Arc::new(root);
        BagOfCells { roots: vec![arc] }
    }

    pub fn add_root(&mut self, root: Cell) {
        let arc = Arc::new(root);
        self.roots.push(arc)
    }

    pub fn num_roots(&self) -> usize {
        self.roots.len()
    }

    pub fn root(&self, idx: usize) -> anyhow::Result<&Arc<Cell>> {
        self.roots.get(idx).ok_or(anyhow!(
            "Invalid index: {}, BoC contains {} roots",
            idx,
            self.roots.len()
        ))
    }

    pub fn single_root(&self) -> anyhow::Result<&Arc<Cell>> {
        if self.roots.len() == 1 {
            Ok(&self.roots[0])
        } else {
            Err(anyhow!("Single root expected, got {}", self.roots.len()))
        }
    }

    pub fn parse(serial: &[u8]) -> anyhow::Result<BagOfCells> {
        let raw = RawBagOfCells::parse(serial)?;
        let num_cells = raw.cells.len();
        let mut cells: Vec<Arc<Cell>> = Vec::new();
        for i in (0..num_cells).rev() {
            let raw_cell = &raw.cells[i];
            let mut cell = Cell {
                data: raw_cell.data.clone(),
                bit_len: raw_cell.bit_len,
                references: Vec::new(),
            };
            for r in &raw_cell.references {
                if *r <= i {
                    return Err(anyhow!("References to previous cells are not supported"));
                }
                cell.references.push(cells[num_cells - 1 - r].clone());
            }
            cells.push(Arc::new(cell));
        }
        let roots: Vec<Arc<Cell>> = raw
            .roots
            .iter()
            .map(|r| cells[num_cells - 1 - r].clone())
            .collect();
        Ok(BagOfCells { roots })
    }

    pub fn parse_hex(hex: &str) -> anyhow::Result<BagOfCells> {
        let str: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
        let bin = hex::decode(str.as_str())?;
        Self::parse(&bin)
    }

    pub fn serialize(&self, has_crc32: bool) -> anyhow::Result<Vec<u8>> {
        let raw = self.to_raw()?;
        raw.serialize(has_crc32)
    }

    /// Traverses all cells, fills all_cells set and inbound references map.
    fn traverse_cell_tree(
        cell: &Arc<Cell>,
        all_cells: &mut HashSet<Arc<Cell>>,
        in_refs: &mut HashMap<Arc<Cell>, HashSet<Arc<Cell>>>,
    ) -> anyhow::Result<()> {
        if !all_cells.contains(cell) {
            all_cells.insert(cell.clone());
            for r in &cell.references {
                if r == cell {
                    return Err(anyhow!("Cell must not reference itself"));
                }
                let maybe_refs = in_refs.get_mut(&r.clone());
                match maybe_refs {
                    Some(refs) => {
                        refs.insert(cell.clone());
                    }
                    None => {
                        let mut refs: HashSet<Arc<Cell>> = HashSet::new();
                        refs.insert(cell.clone());
                        in_refs.insert(r.clone(), refs);
                    }
                }
                Self::traverse_cell_tree(&r, all_cells, in_refs)?;
            }
        }
        Ok(())
    }

    /// Constructs raw representation of BagOfCells
    fn to_raw(&self) -> anyhow::Result<RawBagOfCells> {
        let mut all_cells: HashSet<Arc<Cell>> = HashSet::new();
        let mut in_refs: HashMap<Arc<Cell>, HashSet<Arc<Cell>>> = HashMap::new();
        for r in &self.roots {
            Self::traverse_cell_tree(&r, &mut all_cells, &mut in_refs)?;
        }
        let mut no_in_refs: HashSet<Arc<Cell>> = HashSet::new();
        for c in &all_cells {
            if !in_refs.contains_key(c) {
                no_in_refs.insert(c.clone());
            }
        }
        let mut ordered_cells: Vec<Arc<Cell>> = Vec::new();
        let mut indices: HashMap<Arc<Cell>, usize> = HashMap::new();
        while !no_in_refs.is_empty() {
            let cell = no_in_refs.iter().next().unwrap().clone();
            ordered_cells.push(cell.clone());
            indices.insert(cell.clone(), indices.len());
            for child in &cell.references {
                if let Some(refs) = in_refs.get_mut(child) {
                    refs.remove(&cell);
                    if refs.is_empty() {
                        no_in_refs.insert(child.clone());
                        in_refs.remove(child);
                    }
                }
            }
            no_in_refs.remove(&cell);
        }
        if !in_refs.is_empty() {
            return Err(anyhow!(
                "Can't construct topological ordering: cycle detected"
            ));
        }
        let mut cells: Vec<RawCell> = Vec::new();
        for cell in &ordered_cells {
            let refs: Vec<usize> = cell
                .references
                .iter()
                .map(|c| indices.get(c).unwrap().clone())
                .collect();
            let raw = RawCell {
                data: cell.data.clone(),
                bit_len: cell.bit_len,
                references: refs,
                max_level: cell.get_max_level(),
            };
            cells.push(raw);
        }
        let roots: Vec<usize> = self
            .roots
            .iter()
            .map(|c| indices.get(c).unwrap().clone())
            .collect();
        Ok(RawBagOfCells { cells, roots })
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Instant};

    use num_bigint::BigUint;
    use num_traits::Zero;

    use crate::cell::{BagOfCells, CellBuilder};

    #[test]
    fn it_constructs_raw() -> anyhow::Result<()> {
        let leaf = CellBuilder::new().store_byte(10)?.build()?;
        let inter = CellBuilder::new()
            .store_byte(20)?
            .store_child(leaf)?
            .build()?;
        let root = CellBuilder::new()
            .store_byte(30)?
            .store_child(inter)?
            .build()?;
        let boc = BagOfCells::from_root(root);
        let _raw = boc.to_raw()?;
        Ok(())
    }

    #[test]
    fn cell_repr_works() -> anyhow::Result<()> {
        let hole_address = "EQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAM9c".parse()?;
        let contract = "EQDwHr48oKCFD5od9u_TnsCOhe7tGZIei-5ESWfzhlWLRYvW".parse()?;
        let token0 = "EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR".parse()?;
        let token1 = "EQAIcb1WqNr0E7rOXgO0cbAZQnVbS06mgH2vgBvtBE6p0T2a".parse()?;
        let raw =
            "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\
        1MALHGAGMjLBVADzxZw+gISy2rMyYMG+wBA0lqA";

        let boc = BagOfCells::parse(&base64::decode(raw)?)?;
        let cell = boc.single_root()?;

        let jetton_wallet_code_lp = cell.reference(0)?;
        let pool_code = cell.reference(1)?;
        let account_lp_code = cell.reference(2)?;

        let protocol_fee = CellBuilder::new()
            .store_coins(&BigUint::zero())?
            .store_coins(&BigUint::zero())?
            .store_raw_address(&hole_address)?
            .store_coins(&BigUint::zero())?
            .store_coins(&BigUint::zero())?
            .build()?;

        let data = CellBuilder::new()
            .store_address(&contract)?
            .store_u8(4, 2)?
            .store_u8(4, 0)?
            .store_u8(4, 1)?
            .store_address(&token0)?
            .store_address(&token1)?
            .store_coins(&BigUint::zero())?
            .store_reference(&Arc::new(protocol_fee))?
            .store_reference(jetton_wallet_code_lp)?
            .store_reference(account_lp_code)?
            .build()?;

        let state = CellBuilder::new()
            .store_bit(false)? //Split depth
            .store_bit(false)? //Ticktock
            .store_bit(true)? //Code
            .store_bit(true)? //Data
            .store_bit(false)? //Library
            .store_reference(pool_code)?
            .store_reference(&Arc::new(data))?
            .build()?;

        assert_eq!(
            hex::encode(state.get_repr()?),
            "0201340009000838eee530fd07306581470adf04f707ca92198672c6e4186c331954d4a82151\
                   d553f1bdeac386cb209570c7d74fac7b2b938896147530e3fb4459f46f7b0a18a0"
        );

        Ok(())
    }

    #[ignore]
    #[test]
    fn benchmark_cell_repr() -> anyhow::Result<()> {
        let now = Instant::now();
        for _ in 1..10000 {
            let result = cell_repr_works();
            match result {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        }
        let elapsed = now.elapsed();
        println!("Elapsed: {:.2?}", elapsed);
        Ok(())
        // initially it works for 10.39seceonds
    }
}
