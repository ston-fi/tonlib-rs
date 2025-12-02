use std::collections::HashMap;
use std::hash::Hash;
use std::io::{Cursor, SeekFrom};
use std::sync::Arc;

use bitstream_io::{BigEndian, BitRead, BitReader};
use num_bigint::{BigInt, BigUint};
use num_traits::identities::Zero;

use super::dict::{DictParser, KeyReader, SnakeFormatDict, ValReader};
use super::{ArcCell, Cell, CellBuilder, TonCellNum};
use crate::cell::dict::predefined_readers::{key_reader_256bit, val_reader_snake_formatted_string};
use crate::cell::util::*;
use crate::cell::{MapTonCellError, TonCellError};
use crate::tlb_types::block::msg_address::MsgAddress;
use crate::tlb_types::tlb::TLB;
use crate::types::TON_HASH_LEN;
use crate::{TonAddress, TonHash};

pub struct CellParser<'a> {
    pub cell: &'a Cell,
    data_bit_reader: BitReader<Cursor<&'a [u8]>, BigEndian>,
    next_ref: usize,
}

impl<'a> CellParser<'a> {
    pub fn new(cell: &'a Cell) -> Self {
        let cursor = Cursor::new(cell.data.as_slice());
        let data_bit_reader = BitReader::endian(cursor, BigEndian);
        CellParser {
            cell,
            data_bit_reader,
            next_ref: 0,
        }
    }

    pub fn remaining_bits(&mut self) -> usize {
        let pos = self.data_bit_reader.position_in_bits().unwrap_or_default() as usize;
        self.cell.bit_len.saturating_sub(pos)
    }

    pub fn remaining_refs(&self) -> usize {
        self.cell.references.len() - self.next_ref
    }

    /// Return number of full bytes remaining
    pub fn remaining_bytes(&mut self) -> usize {
        self.remaining_bits() / 8
    }

    pub fn load_bit(&mut self) -> Result<bool, TonCellError> {
        self.ensure_enough_bits(1)?;
        self.data_bit_reader.read_bit().map_cell_parser_error()
    }

    pub fn seek(&mut self, num_bits: i64) -> Result<(), TonCellError> {
        let cur_pos = self
            .data_bit_reader
            .position_in_bits()
            .map_cell_parser_error()?;
        let new_pos = cur_pos as i64 + num_bits;
        if new_pos < 0 || new_pos > self.cell.bit_len as i64 {
            let err_msg = format!(
                "Attempt to advance beyond data range (new_pos: {new_pos}, bit_len: {})",
                self.cell.bit_len
            );
            return Err(TonCellError::CellParserError(err_msg));
        }
        self.data_bit_reader
            .seek_bits(SeekFrom::Current(num_bits))
            .map_cell_parser_error()?;
        Ok(())
    }

    pub fn load_u8(&mut self, bit_len: usize) -> Result<u8, TonCellError> {
        self.load_number(bit_len)
    }

    pub fn load_i8(&mut self, bit_len: usize) -> Result<i8, TonCellError> {
        Ok(self.load_number::<u8>(bit_len)? as i8)
    }

    pub fn load_u16(&mut self, bit_len: usize) -> Result<u16, TonCellError> {
        self.load_number(bit_len)
    }

    pub fn load_i16(&mut self, bit_len: usize) -> Result<i16, TonCellError> {
        Ok(self.load_number::<u16>(bit_len)? as i16)
    }

    pub fn load_u32(&mut self, bit_len: usize) -> Result<u32, TonCellError> {
        self.load_number(bit_len)
    }

    pub fn load_i32(&mut self, bit_len: usize) -> Result<i32, TonCellError> {
        Ok(self.load_number::<u32>(bit_len)? as i32)
    }

    pub fn load_u64(&mut self, bit_len: usize) -> Result<u64, TonCellError> {
        self.load_number(bit_len)
    }

    pub fn load_i64(&mut self, bit_len: usize) -> Result<i64, TonCellError> {
        Ok(self.load_number::<u64>(bit_len)? as i64)
    }

    pub fn load_uint(&mut self, bit_len: usize) -> Result<BigUint, TonCellError> {
        self.ensure_enough_bits(bit_len)?;
        let num_words = bit_len.div_ceil(32);
        let high_word_bits = if bit_len.is_multiple_of(32) {
            32
        } else {
            bit_len % 32
        };
        let mut words: Vec<u32> = vec![0_u32; num_words];
        let high_word = self.load_u32(high_word_bits)?;
        words[num_words - 1] = high_word;
        for i in (0..num_words - 1).rev() {
            let word = self.load_u32(32)?;
            words[i] = word;
        }
        let big_uint = BigUint::new(words);
        Ok(big_uint)
    }

    pub fn load_int(&mut self, bit_len: usize) -> Result<BigInt, TonCellError> {
        self.ensure_enough_bits(bit_len)?;
        let bytes = self.load_bits(bit_len)?;
        let res = BigInt::from_signed_bytes_be(&bytes);
        let extra_bits = bit_len % 8;
        if extra_bits != 0 {
            return Ok(res >> (8 - extra_bits));
        }
        Ok(res)
    }

    pub fn load_byte(&mut self) -> Result<u8, TonCellError> {
        self.load_u8(8)
    }

    pub fn load_slice(&mut self, slice: &mut [u8]) -> Result<(), TonCellError> {
        self.ensure_enough_bits(slice.len() * 8)?;
        self.data_bit_reader
            .read_bytes(slice)
            .map_cell_parser_error()
    }

    pub fn load_bytes(&mut self, num_bytes: usize) -> Result<Vec<u8>, TonCellError> {
        let mut res = vec![0_u8; num_bytes];
        self.load_slice(res.as_mut_slice())?;
        Ok(res)
    }

    pub fn load_ref_cell_optional(&mut self) -> Result<Option<ArcCell>, TonCellError> {
        if self.load_bit()? {
            Ok(Some(self.next_reference()?))
        } else {
            Ok(None)
        }
    }

    pub fn load_bits_to_slice(
        &mut self,
        num_bits: usize,
        slice: &mut [u8],
    ) -> Result<(), TonCellError> {
        self.ensure_enough_bits(num_bits)?;
        self.data_bit_reader.read_bits(num_bits, slice)?;
        Ok(())
    }

    pub fn load_bits(&mut self, bit_len: usize) -> Result<Vec<u8>, TonCellError> {
        self.ensure_enough_bits(bit_len)?;
        let mut dst = vec![0; bit_len.div_ceil(8)];
        let full_bytes = bit_len / 8;
        let remaining_bits = bit_len % 8;

        self.data_bit_reader.read_bytes(&mut dst[..full_bytes])?;

        if remaining_bits != 0 {
            let last_byte = self.data_bit_reader.read_var::<u8>(remaining_bits as u32)?;
            dst[full_bytes] = last_byte << (8 - remaining_bits);
        }
        Ok(dst)
    }

    pub fn load_utf8(&mut self, num_bytes: usize) -> Result<String, TonCellError> {
        let bytes = self.load_bytes(num_bytes)?;
        String::from_utf8(bytes).map_cell_parser_error()
    }

    pub fn load_coins(&mut self) -> Result<BigUint, TonCellError> {
        let num_bytes = self.load_u8(4)?;
        if num_bytes == 0 {
            Ok(BigUint::zero())
        } else {
            self.load_uint(num_bytes as usize * 8)
        }
    }

    pub fn load_remaining(&mut self) -> Result<Cell, TonCellError> {
        let mut builder = CellBuilder::new();
        builder.store_remaining_bits(self)?;
        builder.store_references(&self.cell.references[self.next_ref..])?;
        let cell = builder.build();
        self.next_ref = self.cell.references.len();
        cell
    }

    pub fn load_msg_address(&mut self) -> Result<MsgAddress, TonCellError> {
        MsgAddress::read(self)
    }

    pub fn load_address(&mut self) -> Result<TonAddress, TonCellError> {
        let msg_addr = MsgAddress::read(self)?;
        TonAddress::from_msg_address(msg_addr)
            .map_err(|e| TonCellError::InvalidCellData(e.to_string()))
    }

    pub fn load_unary_length(&mut self) -> Result<usize, TonCellError> {
        let mut res = 0;
        while self.load_bit()? {
            res += 1;
        }
        Ok(res)
    }

    pub fn load_dict_data<K: Eq + Hash, V>(
        &mut self,
        key_len: usize,
        key_reader: KeyReader<K>,
        val_reader: ValReader<V>,
    ) -> Result<HashMap<K, V>, TonCellError> {
        let mut dict_parser = DictParser::new(key_len, key_reader, val_reader);
        dict_parser.parse(self)
    }

    pub fn load_dict<K: Eq + Hash, V>(
        &mut self,
        key_len: usize,
        key_reader: KeyReader<K>,
        val_reader: ValReader<V>,
    ) -> Result<HashMap<K, V>, TonCellError> {
        let has_data = self.load_bit()?;
        if !has_data {
            Ok(HashMap::new())
        } else {
            let reference_cell = self.next_reference()?;
            let mut reference_parser = reference_cell.parser();
            reference_parser.load_dict_data(key_len, key_reader, val_reader)
        }
    }
    ///Snake format when we store part of the data in a cell and the rest of the data in the first child cell (and so recursively).
    ///
    ///Must be prefixed with 0x00 byte.
    ///### TL-B scheme:
    ///
    /// ``` tail#_ {bn:#} b:(bits bn) = SnakeData ~0; ```
    ///
    /// ``` cons#_ {bn:#} {n:#} b:(bits bn) next:^(SnakeData ~n) = SnakeData ~(n + 1); ```
    pub fn load_dict_snake_format(&mut self) -> Result<SnakeFormatDict, TonCellError> {
        self.load_dict(256, key_reader_256bit, val_reader_snake_formatted_string)
    }

    pub fn load_dict_data_snake_format(&mut self) -> Result<SnakeFormatDict, TonCellError> {
        self.load_dict_data(256, key_reader_256bit, val_reader_snake_formatted_string)
    }

    pub fn ensure_empty(&mut self) -> Result<(), TonCellError> {
        let remaining_bits = self.remaining_bits();
        let remaining_refs = self.cell.references.len() - self.next_ref;
        // if remaining_bits == 0 && remaining_refs == 0 { // todo: We will restore reference checking in in 0.18
        if remaining_bits == 0 {
            Ok(())
        } else {
            Err(TonCellError::NonEmptyReader {
                remaining_bits,
                remaining_refs,
            })
        }
    }

    pub fn load_remaining_data_aligned(&mut self) -> Result<Vec<u8>, TonCellError> {
        let remaining = self.remaining_bytes();
        self.load_bytes(remaining)
    }

    // https://docs.ton.org/v3/guidelines/dapps/asset-processing/nft-processing/metadata-parsing#snake-data-encoding
    pub fn load_snake_format_aligned(&mut self, has_prefix: bool) -> Result<Vec<u8>, TonCellError> {
        if has_prefix {
            let prefix = self.load_byte()?;
            if prefix != 0x00 {
                let err_str = format!("Expected snake_format prefix: 0x00, got={prefix}");
                return Err(TonCellError::CellParserError(err_str));
            }
        }
        let mut buffer = self.load_remaining_data_aligned()?;
        if self.next_ref >= self.cell.references.len() {
            return Ok(buffer);
        }
        let mut cur_child = self.next_reference()?;
        let mut cur_parser = cur_child.parser();
        buffer.extend(cur_parser.load_remaining_data_aligned()?);
        while let Ok(next_child) = cur_parser.next_reference() {
            cur_child = next_child;
            cur_parser = cur_child.parser();
            buffer.extend(cur_parser.load_remaining_data_aligned()?);
        }
        Ok(buffer)
    }

    pub fn skip_bits(&mut self, num_bits: usize) -> Result<(), TonCellError> {
        self.ensure_enough_bits(num_bits)?;
        self.data_bit_reader
            .skip(num_bits as u32)
            .map_cell_parser_error()
    }

    pub fn load_number<N: TonCellNum>(&mut self, bit_len: usize) -> Result<N, TonCellError> {
        self.ensure_enough_bits(bit_len)?;
        if bit_len == 0 {
            Ok(N::tcn_from_primitive(N::Primitive::zero()))
        } else if N::IS_PRIMITIVE {
            let primitive = self
                .data_bit_reader
                .read_var::<N::Primitive>(bit_len as u32)?;
            Ok(N::tcn_from_primitive(primitive))
        } else {
            let bytes = self.load_bits(bit_len)?;
            let res = N::tcn_from_bytes(&bytes);
            if !bit_len.is_multiple_of(8) {
                Ok(res.tcn_shr(8 - bit_len as u32 % 8))
            } else {
                Ok(res)
            }
        }
    }

    pub fn load_number_optional<N: TonCellNum>(
        &mut self,
        bit_len: usize,
    ) -> Result<Option<N>, TonCellError> {
        if self.load_bit()? {
            self.load_number(bit_len).map(Some)
        } else {
            Ok(None)
        }
    }

    fn ensure_enough_bits(&mut self, bit_len: usize) -> Result<(), TonCellError> {
        if self.remaining_bits() < bit_len {
            return Err(TonCellError::CellParserError(format!(
                "Not enough bits to read (requested: {}, remaining: {})",
                bit_len,
                self.remaining_bits()
            )));
        }
        Ok(())
    }

    pub fn next_reference(&mut self) -> Result<ArcCell, TonCellError> {
        if self.next_ref < self.cell.references.len() {
            let reference = self.cell.references[self.next_ref].clone();
            self.next_ref += 1;

            Ok(reference)
        } else {
            Err(TonCellError::CellParserError(
                "Not enough references to read".to_owned(),
            ))
        }
    }
    // https://docs.ton.org/develop/data-formats/tl-b-types#eiher
    pub fn load_either_cell_or_cell_ref(&mut self) -> Result<ArcCell, TonCellError> {
        // TODO: think about how we can make it generic
        let is_ref = self.load_bit()?;
        if is_ref {
            Ok(self.next_reference()?)
        } else {
            let remaining_bits = self.remaining_bits();
            let data = self.load_bits(remaining_bits)?;
            let remaining_ref_count = self.cell.references.len() - self.next_ref;
            let mut references = vec![];
            for _ in 0..remaining_ref_count {
                references.push(self.next_reference()?)
            }
            let result = Arc::new(Cell::new(data, remaining_bits, references, false)?);
            Ok(result)
        }
    }
    // https://docs.ton.org/develop/data-formats/tl-b-types#maybe
    pub fn load_maybe_cell_ref(&mut self) -> Result<Option<ArcCell>, TonCellError> {
        let is_some = self.load_bit()?;
        if is_some {
            Ok(Some(self.next_reference()?))
        } else {
            Ok(None)
        }
    }

    pub fn load_tlb<T: TLB>(&mut self) -> Result<T, TonCellError> {
        T::read(self)
    }

    pub fn load_tonhash(&mut self) -> Result<TonHash, TonCellError> {
        let mut res = [0_u8; TON_HASH_LEN];
        self.load_slice(&mut res)?;
        Ok(TonHash::from(res))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use num_bigint::{BigInt, BigUint};
    use tokio_test::{assert_err, assert_ok};

    use crate::cell::parser::TonAddress;
    use crate::cell::{BagOfCells, Cell, CellBuilder, EitherCellLayout};
    use crate::TonHash;

    #[test]
    fn test_remaining_bits() -> anyhow::Result<()> {
        let cell = Cell::new([0b10101010, 0b01010101].to_vec(), 13, vec![], false)?;
        let mut parser = cell.parser();
        assert_eq!(parser.remaining_bits(), 13);
        parser.load_bit()?;
        assert_eq!(parser.remaining_bits(), 12);
        parser.load_u8(4)?;
        assert_eq!(parser.remaining_bits(), 8);
        parser.load_u8(8)?;
        assert_eq!(parser.remaining_bits(), 0);
        Ok(())
    }

    #[test]
    fn test_load_bit() {
        let cell = Cell::new([0b10101010].to_vec(), 4, vec![], false).unwrap();
        let mut parser = cell.parser();
        assert!(parser.load_bit().unwrap());
        assert!(!parser.load_bit().unwrap());
        assert!(parser.load_bit().unwrap());
        assert!(!parser.load_bit().unwrap());
        assert!(parser.load_bit().is_err());
    }

    #[test]
    fn test_load_u8() {
        let cell = Cell::new([0b10101010].to_vec(), 4, vec![], false).unwrap();
        let mut parser = cell.parser();
        assert_eq!(parser.load_u8(4).unwrap(), 0b1010);
        assert!(parser.load_u8(1).is_err());
    }

    #[test]
    fn test_load_i8() {
        let cell = Cell::new([0b10101010].to_vec(), 4, vec![], false).unwrap();
        let mut parser = cell.parser();
        assert_eq!(parser.load_i8(4).unwrap(), 0b1010);
        assert!(parser.load_i8(2).is_err());

        let cell = Cell::new([0b10100110, 0b10101010].to_vec(), 13, vec![], false).unwrap();
        let mut parser = cell.parser();
        assert_eq!(parser.load_i8(4).unwrap(), 0b1010);
        assert_eq!(parser.load_i8(8).unwrap(), 0b01101010);
        assert!(parser.load_i8(2).is_err());
    }

    #[test]
    fn test_load_u16() {
        let cell = Cell::new([0b10101010, 0b01010101].to_vec(), 12, vec![], false).unwrap();
        let mut parser = cell.parser();
        assert_eq!(parser.load_u16(8).unwrap(), 0b10101010);
        assert!(parser.load_u16(8).is_err());
    }

    #[test]
    fn test_load_i16() {
        let cell = Cell::new([0b10101010, 0b01010101].to_vec(), 12, vec![], false).unwrap();
        let mut parser = cell.parser();
        assert_eq!(parser.load_i16(9).unwrap(), 0b101010100);
        assert!(parser.load_i16(4).is_err());
    }

    #[test]
    fn test_load_u32() {
        let cell = Cell::new([0b10101010, 0b01010101].to_vec(), 13, vec![], false).unwrap();
        let mut parser = cell.parser();

        assert_eq!(parser.load_u32(8).unwrap(), 0b10101010);
        assert!(parser.load_u32(8).is_err());
    }

    #[test]
    fn test_load_i32() {
        let cell = Cell::new([0b10101010, 0b01010101].to_vec(), 14, vec![], false).unwrap();
        let mut parser = cell.parser();
        assert_eq!(parser.load_i32(10).unwrap(), 0b1010101001);
        assert!(parser.load_i32(5).is_err());
    }

    #[test]
    fn test_load_u64() {
        let cell = Cell::new([0b10101010, 0b01010101].to_vec(), 13, vec![], false).unwrap();
        let mut parser = cell.parser();
        assert_eq!(parser.load_u64(8).unwrap(), 0b10101010);
        assert!(parser.load_u64(8).is_err());
    }

    #[test]
    fn test_load_i64() {
        let cell = Cell::new([0b10101010, 0b01010101].to_vec(), 14, vec![], false).unwrap();
        let mut parser = cell.parser();
        assert_eq!(parser.load_i64(10).unwrap(), 0b1010101001);
        assert!(parser.load_i64(5).is_err());
    }

    #[test]
    fn test_load_int() {
        let cell = Cell::new([0b10101010, 0b01010101].to_vec(), 14, vec![], false).unwrap();
        let mut parser = cell.parser();
        assert_eq!(parser.load_int(10).unwrap(), BigInt::from(-343));
        assert!(parser.load_int(5).is_err());

        let cell = Cell::new([0b0010_1000].to_vec(), 5, vec![], false).unwrap();
        let mut parser = cell.parser();
        assert_eq!(parser.load_int(5).unwrap(), BigInt::from(5));

        let cell = Cell::new([0b0000_1010].to_vec(), 7, vec![], false).unwrap();
        let mut parser = cell.parser();
        assert_eq!(parser.load_int(7).unwrap(), BigInt::from(5));

        let cell = Cell::new([0b1111_0110].to_vec(), 7, vec![], false).unwrap();
        let mut parser = cell.parser();
        assert_eq!(parser.load_int(7).unwrap(), BigInt::from(-5));

        let cell = Cell::new([0b1101_1000].to_vec(), 5, vec![], false).unwrap();
        let mut parser = cell.parser();
        assert_eq!(parser.load_int(5).unwrap(), BigInt::from(-5));

        let cell = Cell::new([0b11101111].to_vec(), 8, vec![], false).unwrap();
        let mut parser = cell.parser();
        assert_eq!(parser.load_int(8).unwrap(), BigInt::from(-17));
    }

    #[test]
    fn test_store_load_int() -> anyhow::Result<()> {
        let cell = CellBuilder::new()
            .store_int(15, &BigInt::from(0))?
            .store_int(15, &BigInt::from(15))?
            .store_int(123, &BigInt::from(-16))?
            .store_int(123, &BigInt::from(75))?
            .store_int(15, &BigInt::from(-93))?
            .store_int(32, &BigInt::from(83))?
            .store_int(64, &BigInt::from(-183))?
            .store_int(32, &BigInt::from(1401234567u32))?
            .store_int(64, &BigInt::from(-1200617341))?
            .build()?;

        println!("{cell:?}");

        let mut parser = cell.parser();

        assert_eq!(parser.load_int(15)?, BigInt::ZERO);
        assert_eq!(parser.load_int(15)?, BigInt::from(15));
        assert_eq!(parser.load_int(123)?, BigInt::from(-16));
        assert_eq!(parser.load_int(123)?, BigInt::from(75));
        assert_eq!(parser.load_int(15)?, BigInt::from(-93));
        assert_eq!(parser.load_int(32)?, BigInt::from(83));
        assert_eq!(parser.load_int(64)?, BigInt::from(-183));
        assert_eq!(parser.load_int(32)?, BigInt::from(1401234567u32));
        assert_eq!(parser.load_int(64)?, BigInt::from(-1200617341));

        assert!(parser.ensure_empty().is_ok());
        Ok(())
    }

    #[test]
    fn test_load_uint() -> anyhow::Result<()> {
        let cell = Cell::new([0b10101010, 0b01010101].to_vec(), 14, vec![], false)?;
        let mut parser = cell.parser();
        assert_eq!(parser.load_uint(10)?, BigUint::from(0b1010101001u64));
        assert!(parser.load_uint(5).is_err());
        Ok(())
    }

    #[test]
    fn test_load_byte() {
        let cell = Cell::new([0b10101010, 0b01010101].to_vec(), 15, vec![], false).unwrap();
        let mut parser = cell.parser();
        parser.load_bit().unwrap();
        assert_eq!(parser.load_byte().unwrap(), 0b01010100u8);
        assert!(parser.load_byte().is_err());
    }

    #[test]
    fn test_load_slice() {
        let cell = Cell::new(
            [0b10101010, 0b01010101, 0b10101010, 0b10101010, 0b10101010].to_vec(),
            32,
            vec![],
            false,
        )
        .unwrap();
        let mut parser = cell.parser();
        parser.load_bit().unwrap();
        let mut slice = [0; 2];
        parser.load_slice(&mut slice).unwrap();
        assert_eq!(slice, [0b01010100, 0b10101011]);
        assert!(parser.load_slice(&mut slice).is_err());
    }

    #[test]
    fn test_load_bytes() {
        let cell = Cell::new(
            [0b10101010, 0b01010101, 0b10101010, 0b10101010, 0b10101010].to_vec(),
            32,
            vec![],
            false,
        )
        .unwrap();
        let mut parser = cell.parser();
        parser.load_bit().unwrap();
        let slice = parser.load_bytes(2).unwrap();
        assert_eq!(slice, [0b01010100, 0b10101011]);
        assert!(parser.load_bytes(2).is_err());
    }

    #[test]
    fn test_load_bits_to_slice() {
        let cell = Cell::new(
            [0b10101010, 0b01010101, 0b10101010, 0b10101010, 0b10101010].to_vec(),
            22,
            vec![],
            false,
        )
        .unwrap();
        let mut parser = cell.parser();
        parser.load_bit().unwrap();
        let mut slice = [0; 2];
        parser.load_bits_to_slice(12, &mut slice).unwrap();
        assert_eq!(slice, [0b01010100, 0b10100000]);
        assert!(parser.load_bits_to_slice(10, &mut slice).is_err());
    }

    #[test]
    fn test_load_bits() {
        let cell = Cell::new(
            [0b10101010, 0b01010101, 0b10101010, 0b10101010, 0b10101010].to_vec(),
            25,
            vec![],
            false,
        )
        .unwrap();
        let mut parser = cell.parser();
        parser.load_bit().unwrap();
        let slice = parser.load_bits(5).unwrap();
        assert_eq!(slice, [0b01010000]);
        let slice = parser.load_bits(15).unwrap();
        assert_eq!(slice, [0b10010101, 0b01101010]);
        assert!(parser.load_bits(5).is_err());
    }

    #[test]
    fn test_load_utf8() {
        let cell = Cell::new("a1j\0".as_bytes().to_vec(), 31, vec![], false).unwrap();
        let mut parser = cell.parser();
        let string = parser.load_utf8(2).unwrap();
        assert_eq!(string, "a1");
        let string = parser.load_utf8(1).unwrap();
        assert_eq!(string, "j");
        assert!(parser.load_utf8(1).is_err());
    }

    #[test]
    fn test_load_coins() {
        let cell = Cell::new(
            [
                0b00011111, 0b11110011, 0b11110011, 0b11110011, 0b11110011, 0b00011111, 0b11110011,
            ]
            .to_vec(),
            48,
            vec![],
            false,
        )
        .unwrap();
        let mut parser = cell.parser();

        assert_eq!(parser.load_coins().unwrap(), BigUint::from(0b11111111u64));
        assert_eq!(
            parser.load_coins().unwrap(),
            BigUint::from(0b111100111111001111110011u64)
        );
        assert!(parser.load_coins().is_err());
    }

    #[test]
    fn test_load_address() {
        let cell = Cell::new([0].to_vec(), 2, vec![], false).unwrap();
        let mut parser = cell.parser();
        assert_eq!(parser.load_address().unwrap(), TonAddress::NULL);
        assert!(parser.load_address().is_err());

        // with full addresses
        let cell = Cell::new(
            [
                0b10000000, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0b00010000, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0b00000010, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ]
            .to_vec(),
            (3 + 8 + 32 * 8) * 3 - 1,
            vec![],
            false,
        )
        .unwrap();
        let mut parser = cell.parser();
        assert_eq!(parser.load_address().unwrap(), TonAddress::NULL);
        assert_eq!(parser.load_address().unwrap(), TonAddress::NULL);
        assert!(parser.load_address().is_err());
    }

    #[test]
    fn test_ensure_empty() {
        let cell = Cell::new([0b10101010].to_vec(), 7, vec![], false).unwrap();
        let mut parser = cell.parser();
        parser.load_u8(4).unwrap();
        assert!(parser.ensure_empty().is_err());
        parser.load_u8(3).unwrap();
        assert!(parser.ensure_empty().is_ok());
    }

    #[test]
    fn test_skip_bits_not_enough_bits() {
        let cell = Cell::new([0b11111001, 0b00001010].to_vec(), 12, vec![], false).unwrap();
        let mut parser = cell.parser();
        assert!(parser.skip_bits(5).is_ok());
        assert_eq!(parser.load_bits(5).unwrap(), [0b00100000]);
        assert!(parser.skip_bits(3).is_err());
    }

    #[test]
    fn test_parser_with_refs() {
        let ref1 = Cell::new([0b11111001, 0b00001010].to_vec(), 12, vec![], false).unwrap();
        let ref2 = Cell::new([0b11111001, 0b00001010].to_vec(), 12, vec![], false).unwrap();
        let cell = Cell::new(
            [0b11111001, 0b00001010].to_vec(),
            12,
            vec![ref1.into(), ref2.into()],
            false,
        )
        .unwrap();
        let mut parser = cell.parser();

        assert!(parser.next_reference().is_ok());
        assert!(parser.next_reference().is_ok());
        assert!(parser.next_reference().is_err());
    }

    #[test]
    fn test_either_with_references() -> anyhow::Result<()> {
        let reference_cell = Cell::new([0xA5, 0x5A].to_vec(), 12, vec![], false)?;
        let cell_either = Arc::new(Cell::new(
            [0xFF, 0xB0].to_vec(),
            12,
            vec![reference_cell.into()],
            false,
        )?);
        let cell = CellBuilder::new()
            .store_bit(true)?
            .store_either_cell_or_cell_ref(&cell_either, EitherCellLayout::Native)?
            .build()?;

        let mut parser = cell.parser();

        let result_first_bit = parser.load_bit()?;
        let result_cell_either = parser.load_either_cell_or_cell_ref()?;

        assert!(result_first_bit);
        assert_eq!(result_cell_either, cell_either);
        Ok(())
    }

    #[test]
    fn test_load_tonhash() {
        let ton_hash =
            TonHash::from_hex("9f31f4f413a3accb706c88962ac69d59103b013a0addcfaeed5dd73c18fa98a8")
                .unwrap();
        let cell = Cell::new(ton_hash.to_vec(), 256, vec![], false).unwrap();
        let mut parser = cell.parser();
        let loaded = parser.load_tonhash().unwrap();
        assert_eq!(loaded, ton_hash);
    }

    #[test]
    fn test_load_address_anycast() -> anyhow::Result<()> {
        let addr_boc = hex::decode("b5ee9c7201010101002800004bbe031053100134ea6c68e2f2cee9619bdd2732493f3a1361eccd7c5267a9eb3c5dcebc533bb6")?;
        let addr_cell = BagOfCells::parse(&addr_boc)?.single_root()?;
        let mut parser = addr_cell.parser();
        let parsed = assert_ok!(parser.load_address());
        let expected: TonAddress = "EQADEFMSOLyzulhm90nMkk_OhNh7M18Umep6zxdzrxTO7Zz7".parse()?;
        assert_eq!(parsed, expected);

        let addr_boc = hex::decode("b5ee9c7201010101002800004bbe779dcc80039c768512c82704ef59297e7991b21b469367a4aac9d9ae9fe74a834b2448490e")?;
        let addr_cell = BagOfCells::parse(&addr_boc)?.single_root()?;
        let mut parser = addr_cell.parser();
        let parsed = assert_ok!(parser.load_address());
        let expected: TonAddress = "EQB3ncyAsgnBO9ZKX55kbIbRpNnpKrJ2a6f50qDSyRISQ19D".parse()?;
        assert_eq!(parsed, expected);
        Ok(())
    }

    #[test]
    fn test_load_remaining_data_aligned() -> anyhow::Result<()> {
        let cell = CellBuilder::new()
            .store_bits(512, &[0b10101010; 64])?
            .build()?;
        let mut parser = cell.parser();
        parser.load_u8(8)?;
        let remaining_data = parser.load_remaining_data_aligned()?;
        assert_eq!(remaining_data, &[0b10101010; 63]);
        Ok(())
    }

    #[test]
    fn test_snake_format_aligned() -> anyhow::Result<()> {
        let child2 = CellBuilder::new()
            .store_bits(512, &[0b10101010; 64])?
            .build()?;
        let child1 = CellBuilder::new()
            .store_bits(512, &[0b01010101; 64])?
            .store_reference(&child2.to_arc())?
            .build()?;
        let cell = CellBuilder::new()
            .store_bits(512, &[0b00000000; 64])?
            .store_reference(&child1.to_arc())?
            .build()?;
        let mut expected = vec![0b00000000; 64];
        expected.extend(vec![0b01010101; 64]);
        expected.extend(vec![0b10101010; 64]);

        let snake_data = cell.parser().load_snake_format_aligned(false)?;
        assert_eq!(snake_data, expected);
        let snake_data = cell.parser().load_snake_format_aligned(true)?;
        assert_eq!(snake_data, expected[1..]);
        Ok(())
    }

    #[test]
    fn test_seek() -> anyhow::Result<()> {
        let cell = Cell::new([0b11000011].to_vec(), 8, vec![], false)?;
        let mut parser = cell.parser();
        assert_ok!(parser.seek(4));
        assert_eq!(parser.load_u8(4)?, 0b0011);
        assert_ok!(parser.seek(-8));
        assert_eq!(parser.load_u8(4)?, 0b1100);
        assert_ok!(parser.seek(-4));
        assert_eq!(parser.load_u8(4)?, 0b1100);
        assert_err!(parser.seek(-5));
        assert_eq!(parser.load_u8(4)?, 0b0011);
        Ok(())
    }
}
