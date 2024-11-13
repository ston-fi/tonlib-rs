use std::collections::HashMap;
use std::hash::Hash;
use std::io::Cursor;
use std::sync::Arc;

use bitstream_io::{BigEndian, BitRead, BitReader, Numeric};
use num_bigint::{BigInt, BigUint, Sign};
use num_traits::identities::Zero;

use super::dict::{DictParser, KeyReader, SnakeFormatDict, ValReader};
use super::{ArcCell, Cell, CellBuilder};
use crate::cell::dict::predefined_readers::{key_reader_256bit, val_reader_snake_formatted_string};
use crate::cell::util::*;
use crate::cell::{MapTonCellError, TonCellError};
use crate::TonAddress;

pub struct CellParser<'a> {
    pub(crate) bit_len: usize,
    pub(crate) bit_reader: BitReader<Cursor<&'a [u8]>, BigEndian>,
    pub(crate) references: &'a [ArcCell],
    next_ref: usize,
}

impl<'a> CellParser<'a> {
    pub fn new(bit_len: usize, data: &'a [u8], references: &'a [ArcCell]) -> Self {
        let cursor = Cursor::new(data);
        let bit_reader = BitReader::endian(cursor, BigEndian);
        CellParser {
            bit_len,
            bit_reader,
            references,
            next_ref: 0,
        }
    }

    pub fn remaining_bits(&mut self) -> usize {
        let pos = self.bit_reader.position_in_bits().unwrap_or_default() as usize;
        if self.bit_len > pos {
            self.bit_len - pos
        } else {
            0
        }
    }

    /// Return number of full bytes remaining
    pub fn remaining_bytes(&mut self) -> usize {
        self.remaining_bits() / 8
    }

    pub fn load_bit(&mut self) -> Result<bool, TonCellError> {
        self.ensure_enough_bits(1)?;
        self.bit_reader.read_bit().map_cell_parser_error()
    }

    pub fn load_u8(&mut self, bit_len: usize) -> Result<u8, TonCellError> {
        self.load_number(bit_len)
    }

    pub fn load_i8(&mut self, bit_len: usize) -> Result<i8, TonCellError> {
        self.load_number(bit_len)
    }

    pub fn load_u16(&mut self, bit_len: usize) -> Result<u16, TonCellError> {
        self.load_number(bit_len)
    }

    pub fn load_i16(&mut self, bit_len: usize) -> Result<i16, TonCellError> {
        self.load_number(bit_len)
    }

    pub fn load_u32(&mut self, bit_len: usize) -> Result<u32, TonCellError> {
        self.load_number(bit_len)
    }

    pub fn load_i32(&mut self, bit_len: usize) -> Result<i32, TonCellError> {
        self.load_number(bit_len)
    }

    pub fn load_u64(&mut self, bit_len: usize) -> Result<u64, TonCellError> {
        self.load_number(bit_len)
    }

    pub fn load_i64(&mut self, bit_len: usize) -> Result<i64, TonCellError> {
        self.load_number(bit_len)
    }

    pub fn load_uint(&mut self, bit_len: usize) -> Result<BigUint, TonCellError> {
        self.ensure_enough_bits(bit_len)?;
        let num_words = (bit_len + 31) / 32;
        let high_word_bits = if bit_len % 32 == 0 { 32 } else { bit_len % 32 };
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
        let num_words = (bit_len + 31) / 32;
        let high_word_bits = if bit_len % 32 == 0 { 32 } else { bit_len % 32 };
        let mut words: Vec<u32> = vec![0_u32; num_words];
        let high_word = self.load_u32(high_word_bits)?;
        let sign = if (high_word & (1 << 31)) == 0 {
            Sign::Plus
        } else {
            Sign::Minus
        };
        words[num_words - 1] = high_word;
        for i in (0..num_words - 1).rev() {
            let word = self.load_u32(32)?;
            words[i] = word;
        }
        let big_uint = BigInt::new(sign, words);
        Ok(big_uint)
    }

    pub fn load_byte(&mut self) -> Result<u8, TonCellError> {
        self.load_u8(8)
    }

    pub fn load_slice(&mut self, slice: &mut [u8]) -> Result<(), TonCellError> {
        self.ensure_enough_bits(slice.len() * 8)?;
        self.bit_reader.read_bytes(slice).map_cell_parser_error()
    }

    pub fn load_bytes(&mut self, num_bytes: usize) -> Result<Vec<u8>, TonCellError> {
        let mut res = vec![0_u8; num_bytes];
        self.load_slice(res.as_mut_slice())?;
        Ok(res)
    }

    pub fn load_bits_to_slice(
        &mut self,
        num_bits: usize,
        slice: &mut [u8],
    ) -> Result<(), TonCellError> {
        self.ensure_enough_bits(num_bits)?;
        self.bit_reader.read_bits(num_bits, slice)?;
        Ok(())
    }

    pub fn load_bits(&mut self, num_bits: usize) -> Result<Vec<u8>, TonCellError> {
        let total_bytes = (num_bits + 7) / 8;
        let mut res = vec![0_u8; total_bytes];
        self.load_bits_to_slice(num_bits, res.as_mut_slice())?;
        Ok(res)
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
        builder.store_references(&self.references[self.next_ref..])?;
        let cell = builder.build();
        self.next_ref = self.references.len();
        cell
    }

    pub fn load_address(&mut self) -> Result<TonAddress, TonCellError> {
        self.ensure_enough_bits(2)?;
        let tp = self.bit_reader.read::<u8>(2).map_cell_parser_error()?;
        match tp {
            0 => Ok(TonAddress::null()),
            2 => {
                self.ensure_enough_bits(1 + 8 + 32 * 8)?;
                let _res1 = self.bit_reader.read::<u8>(1).map_cell_parser_error()?;
                let wc = self.bit_reader.read::<u8>(8).map_cell_parser_error()?;
                let mut hash_part = [0_u8; 32];
                self.bit_reader
                    .read_bytes(&mut hash_part)
                    .map_cell_parser_error()?;
                let addr = TonAddress::new(wc as i32, &hash_part);
                Ok(addr)
            }
            _ => Err(TonCellError::InvalidAddressType(tp)),
        }
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
        let remaining_refs = self.references.len() - self.next_ref;
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

    pub fn skip_bits(&mut self, num_bits: usize) -> Result<(), TonCellError> {
        self.ensure_enough_bits(num_bits)?;
        self.bit_reader
            .skip(num_bits as u32)
            .map_cell_parser_error()
    }

    fn load_number<N: Numeric>(&mut self, bit_len: usize) -> Result<N, TonCellError> {
        self.ensure_enough_bits(bit_len)?;

        self.bit_reader
            .read::<N>(bit_len as u32)
            .map_cell_parser_error()
    }

    fn ensure_enough_bits(&mut self, bit_len: usize) -> Result<(), TonCellError> {
        if self.remaining_bits() < bit_len {
            return Err(TonCellError::CellParserError(
                "Not enough bits to read".to_owned(),
            ));
        }
        Ok(())
    }

    pub fn next_reference(&mut self) -> Result<ArcCell, TonCellError> {
        if self.next_ref < self.references.len() {
            let reference = self.references[self.next_ref].clone();
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
            let remaining_ref_count = self.references.len() - self.next_ref;
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
}

#[cfg(test)]
mod tests {

    use std::sync::Arc;

    use num_bigint::{BigInt, BigUint};

    use crate::cell::{Cell, CellBuilder, EitherCellLayout};
    use crate::TonAddress;

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
        assert_eq!(parser.load_int(10).unwrap(), BigInt::from(0b1010101001));
        assert!(parser.load_int(5).is_err());
    }

    #[test]
    fn test_load_uint() {
        let cell = Cell::new([0b10101010, 0b01010101].to_vec(), 14, vec![], false).unwrap();
        let mut parser = cell.parser();
        assert_eq!(
            parser.load_uint(10).unwrap(),
            BigUint::from(0b1010101001u64)
        );
        assert!(parser.load_uint(5).is_err());
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
        let cell = Cell::new([0].to_vec(), 3, vec![], false).unwrap();
        let mut parser = cell.parser();
        assert_eq!(parser.load_address().unwrap(), TonAddress::null());
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
        assert_eq!(parser.load_address().unwrap(), TonAddress::null());
        assert_eq!(parser.load_address().unwrap(), TonAddress::null());
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
    fn test_either_with_references() {
        let reference_cell = Cell::new([0xA5, 0x5A].to_vec(), 12, vec![], false).unwrap();
        let cell_either = Arc::new(
            Cell::new(
                [0xFF, 0xB0].to_vec(),
                12,
                vec![reference_cell.into()],
                false,
            )
            .unwrap(),
        );
        let cell = CellBuilder::new()
            .store_bit(true)
            .unwrap()
            .store_either_cell_or_cell_ref(&cell_either, EitherCellLayout::Native)
            .unwrap()
            .build()
            .unwrap();

        let mut parser = cell.parser();

        let result_first_bit = parser.load_bit().unwrap();
        let result_cell_either = parser.load_either_cell_or_cell_ref().unwrap();

        assert!(result_first_bit);
        assert_eq!(result_cell_either, cell_either);
    }
}
