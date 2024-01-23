use std::io::{Cursor, SeekFrom};

use bitstream_io::{BigEndian, BitRead, BitReader};
use num_bigint::BigUint;
use num_traits::identities::Zero;

use crate::address::TonAddress;
use crate::cell::{MapTonCellError, TonCellError};

pub struct CellParser<'a> {
    pub(crate) bit_len: usize,
    pub(crate) bit_reader: BitReader<Cursor<&'a Vec<u8>>, BigEndian>,
}

impl CellParser<'_> {
    pub fn remaining_bits(&self) -> usize {
        let pos = self
            .bit_reader
            .clone()
            .seek_bits(SeekFrom::Current(0))
            .ok()
            .unwrap_or_default() as usize;
        if self.bit_len > pos {
            self.bit_len - pos
        } else {
            0
        }
    }

    /// Return number of full bytes remaining
    pub fn remaining_bytes(&self) -> usize {
        self.remaining_bits() / 8
    }

    pub fn load_bit(&mut self) -> Result<bool, TonCellError> {
        self.bit_reader.read_bit().map_cell_parser_error()
    }

    pub fn load_u8(&mut self, bit_len: usize) -> Result<u8, TonCellError> {
        self.bit_reader
            .read::<u8>(bit_len as u32)
            .map_cell_parser_error()
    }

    pub fn load_u32(&mut self, bit_len: usize) -> Result<u32, TonCellError> {
        self.bit_reader
            .read::<u32>(bit_len as u32)
            .map_cell_parser_error()
    }

    pub fn load_u64(&mut self, bit_len: usize) -> Result<u64, TonCellError> {
        self.bit_reader
            .read::<u64>(bit_len as u32)
            .map_cell_parser_error()
    }

    pub fn load_uint(&mut self, bit_len: usize) -> Result<BigUint, TonCellError> {
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

    pub fn load_byte(&mut self) -> Result<u8, TonCellError> {
        self.load_u8(8)
    }

    pub fn load_slice(&mut self, slice: &mut [u8]) -> Result<(), TonCellError> {
        self.bit_reader.read_bytes(slice).map_cell_parser_error()
    }

    pub fn load_bytes(&mut self, num_bytes: usize) -> Result<Vec<u8>, TonCellError> {
        let mut res = vec![0_u8; num_bytes];
        self.load_slice(res.as_mut_slice())?;
        Ok(res)
    }

    pub fn load_utf8(&mut self, num_bytes: usize) -> Result<String, TonCellError> {
        let bytes = self.load_bytes(num_bytes)?;
        String::from_utf8(bytes).map_cell_parser_error()
    }

    pub fn load_utf8_lossy(&mut self, num_bytes: usize) -> Result<String, TonCellError> {
        let bytes = self.load_bytes(num_bytes)?;
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }

    pub fn load_coins(&mut self) -> Result<BigUint, TonCellError> {
        let num_bytes = self.load_u8(4)?;
        if num_bytes == 0 {
            Ok(BigUint::zero())
        } else {
            self.load_uint((num_bytes * 8) as usize)
        }
    }

    pub fn load_address(&mut self) -> Result<TonAddress, TonCellError> {
        let tp = self.bit_reader.read::<u8>(2).map_cell_parser_error()?;
        match tp {
            0 => Ok(TonAddress::null()),
            2 => {
                let _res1 = self.bit_reader.read::<u8>(1).map_cell_parser_error()?;
                let wc = self.bit_reader.read::<u8>(8).map_cell_parser_error()?;
                let mut hash_part = [0_u8; 32];
                self.bit_reader
                    .read_bytes(&mut hash_part)
                    .map_cell_parser_error()?;
                let addr = TonAddress::new(wc as i32, &hash_part);
                Ok(addr)
            }
            _ => Err(TonCellError::InvalidAddressType { tp }),
        }
    }

    pub fn load_unary_length(&mut self) -> Result<usize, TonCellError> {
        let mut res = 0;
        while self.load_bit()? {
            res += 1;
        }
        Ok(res)
    }

    pub fn ensure_empty(&self) -> Result<(), TonCellError> {
        let remaining_bits = self.remaining_bits();
        if remaining_bits == 0 {
            Ok(())
        } else {
            Err(TonCellError::NonEmptyReader { remaining_bits })
        }
    }
}
