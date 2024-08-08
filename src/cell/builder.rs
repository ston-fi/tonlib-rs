use std::ops::Add;
use std::sync::Arc;

use bitstream_io::{BigEndian, BitWrite, BitWriter};
use num_bigint::{BigInt, BigUint, Sign};
use num_traits::{One, Zero};

use crate::address::TonAddress;
use crate::cell::error::{MapTonCellError, TonCellError};
use crate::cell::{ArcCell, Cell, CellParser};

const MAX_CELL_BITS: usize = 1023;
const MAX_CELL_REFERENCES: usize = 4;

pub struct CellBuilder {
    bit_writer: BitWriter<Vec<u8>, BigEndian>,
    bits_to_write: usize,
    references: Vec<ArcCell>,
    is_cell_exotic: bool,
}

impl CellBuilder {
    pub fn new() -> CellBuilder {
        let bit_writer = BitWriter::endian(Vec::new(), BigEndian);
        CellBuilder {
            bit_writer,
            bits_to_write: 0,
            references: Vec::new(),
            is_cell_exotic: false,
        }
    }

    pub fn set_cell_is_exotic(&mut self, val: bool) {
        self.is_cell_exotic = val;
    }

    pub fn store_bit(&mut self, val: bool) -> Result<&mut Self, TonCellError> {
        self.bit_writer.write_bit(val).map_cell_builder_error()?;
        self.bits_to_write += 1;
        Ok(self)
    }

    pub fn store_u8(&mut self, bit_len: usize, val: u8) -> Result<&mut Self, TonCellError> {
        self.bit_writer
            .write(bit_len as u32, val)
            .map_cell_builder_error()?;
        self.bits_to_write += bit_len;
        Ok(self)
    }

    pub fn store_i8(&mut self, bit_len: usize, val: i8) -> Result<&mut Self, TonCellError> {
        self.bit_writer
            .write(bit_len as u32, val)
            .map_cell_builder_error()?;
        self.bits_to_write += bit_len;
        Ok(self)
    }

    pub fn store_u32(&mut self, bit_len: usize, val: u32) -> Result<&mut Self, TonCellError> {
        self.bit_writer
            .write(bit_len as u32, val)
            .map_cell_builder_error()?;
        self.bits_to_write += bit_len;
        Ok(self)
    }

    pub fn store_i32(&mut self, bit_len: usize, val: i32) -> Result<&mut Self, TonCellError> {
        self.bit_writer
            .write(bit_len as u32, val)
            .map_cell_builder_error()?;
        self.bits_to_write += bit_len;
        Ok(self)
    }

    pub fn store_u64(&mut self, bit_len: usize, val: u64) -> Result<&mut Self, TonCellError> {
        self.bit_writer
            .write(bit_len as u32, val)
            .map_cell_builder_error()?;
        self.bits_to_write += bit_len;
        Ok(self)
    }

    pub fn store_i64(&mut self, bit_len: usize, val: i64) -> Result<&mut Self, TonCellError> {
        self.bit_writer
            .write(bit_len as u32, val)
            .map_cell_builder_error()?;
        self.bits_to_write += bit_len;
        Ok(self)
    }

    pub fn store_uint(&mut self, bit_len: usize, val: &BigUint) -> Result<&mut Self, TonCellError> {
        let val_bits = if val.is_zero() {
            0
        } else {
            val.bits() as usize
        };

        if val_bits > bit_len {
            return Err(TonCellError::cell_builder_error(format!(
                "Value {} doesn't fit in {} bits (takes {} bits)",
                val, bit_len, val_bits
            )));
        }
        // example: bit_len=13, val=5. 5 = 00000101, we must store 0000000000101
        // leading_zeros_bits = 10
        // leading_zeros_bytes = 10 / 8 = 1
        let leading_zero_bits = bit_len - val_bits;
        let leading_zeros_bytes = leading_zero_bits / 8;
        for _ in 0..leading_zeros_bytes {
            self.store_byte(0)?;
        }
        // we must align high byte of val to specified bit_len, 00101 in our case
        let extra_zeros = leading_zero_bits % 8;
        for _ in 0..extra_zeros {
            self.store_bit(false)?;
        }
        // and then store val's high byte in minimum number of bits
        let val_bytes = val.to_bytes_be();
        let high_bits_cnt = {
            let cnt = val_bits % 8;
            if cnt == 0 {
                if val.is_zero() {
                    0
                } else {
                    8
                }
            } else {
                cnt
            }
        };
        let high_byte = val_bytes[0];
        for i in 0..high_bits_cnt {
            self.store_bit(high_byte & (1 << (high_bits_cnt - i - 1)) != 0)?;
        }
        // store the rest of val
        for byte in val_bytes.iter().skip(1) {
            self.store_byte(*byte)?;
        }
        Ok(self)
    }

    pub fn store_int(&mut self, bit_len: usize, val: &BigInt) -> Result<&mut Self, TonCellError> {
        let (sign, mag) = val.clone().into_parts();

        let mag_bits = if mag.is_zero() {
            bit_len as u64
        } else {
            mag.bits()
        };
        let bit_len = bit_len - 1; // reserve 1 bit for sign
        if bit_len < mag_bits as usize {
            return Err(TonCellError::cell_builder_error(format!(
                "Value {} doesn't fit in {} bits (takes {} bits)",
                val, bit_len, mag_bits
            )));
        }
        if sign == Sign::Minus {
            self.store_byte(1)?;
            self.store_uint(bit_len, &extend_and_invert_bits(bit_len, &mag)?)?;
        } else {
            self.store_byte(0)?;
            self.store_uint(bit_len, &mag)?;
        };
        Ok(self)
    }

    pub fn store_byte(&mut self, val: u8) -> Result<&mut Self, TonCellError> {
        self.store_u8(8, val)
    }

    pub fn store_slice(&mut self, slice: &[u8]) -> Result<&mut Self, TonCellError> {
        for val in slice {
            self.store_byte(*val)?;
        }
        Ok(self)
    }

    pub fn store_bits(&mut self, bit_len: usize, slice: &[u8]) -> Result<&mut Self, TonCellError> {
        let full_bytes = bit_len / 8;
        self.store_slice(&slice[0..full_bytes])?;
        let last_byte_len = bit_len % 8;
        if last_byte_len != 0 {
            let last_byte = slice[full_bytes] >> (8 - last_byte_len);
            self.store_u8(last_byte_len, last_byte)?;
        }
        Ok(self)
    }

    pub fn store_string(&mut self, val: &str) -> Result<&mut Self, TonCellError> {
        self.store_slice(val.as_bytes())
    }

    pub fn store_coins(&mut self, val: &BigUint) -> Result<&mut Self, TonCellError> {
        if val.is_zero() {
            self.store_u8(4, 0)
        } else {
            let num_bytes = (val.bits() as usize + 7) / 8;
            self.store_u8(4, num_bytes as u8)?;
            self.store_uint(num_bytes * 8, val)
        }
    }

    /// Stores address without optimizing hole address
    pub fn store_raw_address(&mut self, val: &TonAddress) -> Result<&mut Self, TonCellError> {
        self.store_u8(2, 0b10u8)?;
        self.store_bit(false)?;
        let wc = (val.workchain & 0xff) as u8;
        self.store_u8(8, wc)?;
        self.store_slice(&val.hash_part)?;
        Ok(self)
    }

    /// Stores address optimizing hole address two to bits
    pub fn store_address(&mut self, val: &TonAddress) -> Result<&mut Self, TonCellError> {
        if val == &TonAddress::NULL {
            self.store_u8(2, 0)?;
        } else {
            self.store_raw_address(val)?;
        }
        Ok(self)
    }

    /// Adds reference to an existing `Cell`.
    ///
    /// The reference is passed as `ArcCell` so it might be references from other cells.
    pub fn store_reference(&mut self, cell: &ArcCell) -> Result<&mut Self, TonCellError> {
        let ref_count = self.references.len() + 1;
        if ref_count > 4 {
            return Err(TonCellError::cell_builder_error(format!(
                "Cell must contain at most 4 references, got {}",
                ref_count
            )));
        }
        self.references.push(cell.clone());
        Ok(self)
    }

    pub fn store_references(&mut self, refs: &[ArcCell]) -> Result<&mut Self, TonCellError> {
        for r in refs {
            self.store_reference(r)?;
        }
        Ok(self)
    }

    /// Adds a reference to a newly constructed `Cell`.
    ///
    /// The cell is wrapped it the `Arc`.
    pub fn store_child(&mut self, cell: Cell) -> Result<&mut Self, TonCellError> {
        self.store_reference(&Arc::new(cell))
    }

    pub fn store_remaining_bits(
        &mut self,
        parser: &mut CellParser,
    ) -> Result<&mut Self, TonCellError> {
        let num_full_bytes = parser.remaining_bits() / 8;
        let bytes = parser.load_bytes(num_full_bytes)?;
        self.store_slice(bytes.as_slice())?;
        let num_bits = parser.remaining_bits() % 8;
        let tail = parser.load_u8(num_bits)?;
        self.store_u8(num_bits, tail)?;
        Ok(self)
    }

    pub fn store_cell_data(&mut self, cell: &Cell) -> Result<&mut Self, TonCellError> {
        let mut parser = cell.parser();
        self.store_remaining_bits(&mut parser)?;
        Ok(self)
    }

    pub fn store_cell(&mut self, cell: &Cell) -> Result<&mut Self, TonCellError> {
        self.store_cell_data(cell)?;
        self.store_references(cell.references.as_slice())?;
        Ok(self)
    }

    // https://docs.ton.org/develop/data-formats/tl-b-types#either
    pub fn store_either_cell_or_cell_ref(
        &mut self,
        cell: &ArcCell,
    ) -> Result<&mut Self, TonCellError> {
        if cell.bit_len() < self.remaining_bits() {
            self.store_bit(false)?;
            self.store_cell(cell)?;
        } else {
            self.store_bit(true)?;
            self.store_reference(cell)?;
        }

        Ok(self)
    }

    // https://docs.ton.org/develop/data-formats/tl-b-types#maybe
    pub fn store_maybe_cell_ref(
        &mut self,
        maybe_cell: &Option<ArcCell>,
    ) -> Result<&mut Self, TonCellError> {
        if let Some(cell) = maybe_cell {
            self.store_bit(true)?;
            self.store_reference(cell)?;
        } else {
            self.store_bit(false)?;
        }

        Ok(self)
    }

    pub fn remaining_bits(&self) -> usize {
        MAX_CELL_BITS - self.bits_to_write
    }

    pub fn build(&mut self) -> Result<Cell, TonCellError> {
        let mut trailing_zeros = 0;
        while !self.bit_writer.byte_aligned() {
            self.bit_writer.write_bit(false).map_cell_builder_error()?;
            trailing_zeros += 1;
        }

        if let Some(vec) = self.bit_writer.writer() {
            let bit_len = vec.len() * 8 - trailing_zeros;
            if bit_len > MAX_CELL_BITS {
                return Err(TonCellError::cell_builder_error(format!(
                    "Cell must contain at most {} bits, got {}",
                    MAX_CELL_BITS, bit_len
                )));
            }
            let ref_count = self.references.len();
            if ref_count > MAX_CELL_REFERENCES {
                return Err(TonCellError::cell_builder_error(format!(
                    "Cell must contain at most 4 references, got {}",
                    ref_count
                )));
            }

            Cell::new(
                vec.clone(),
                bit_len,
                self.references.clone(),
                self.is_cell_exotic,
            )
        } else {
            Err(TonCellError::CellBuilderError(
                "Stream is not byte-aligned".to_string(),
            ))
        }
    }
}

fn extend_and_invert_bits(bits_cnt: usize, src: &BigUint) -> Result<BigUint, TonCellError> {
    if bits_cnt < src.bits() as usize {
        return Err(TonCellError::cell_builder_error(format!(
            "Can't invert bits: value {} doesn't fit in {} bits",
            src, bits_cnt
        )));
    }

    let src_bytes = src.to_bytes_be();
    let inverted_bytes_cnt = (bits_cnt + 7) / 8;
    let mut inverted = vec![0xffu8; inverted_bytes_cnt];
    // can be optimized
    for (pos, byte) in src_bytes.iter().rev().enumerate() {
        let inverted_pos = inverted.len() - 1 - pos;
        inverted[inverted_pos] ^= byte;
    }
    let mut inverted_val_bytes = BigUint::from_bytes_be(&inverted)
        .add(BigUint::one())
        .to_bytes_be();
    let leading_zeros = inverted_bytes_cnt * 8 - bits_cnt;
    inverted_val_bytes[0] &= 0xffu8 >> leading_zeros;
    Ok(BigUint::from_bytes_be(&inverted_val_bytes))
}

impl Default for CellBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use num_bigint::{BigInt, BigUint, Sign};
    use num_traits::Zero;

    use crate::address::TonAddress;
    use crate::cell::builder::extend_and_invert_bits;
    use crate::cell::{CellBuilder, TonCellError};

    #[test]
    fn test_extend_and_invert_bits() -> Result<(), TonCellError> {
        let a = BigUint::from(1u8);
        let b = extend_and_invert_bits(8, &a)?;
        println!("a: {:0x}", a);
        println!("b: {:0x}", b);
        assert_eq!(b, BigUint::from(0xffu8));

        let b = extend_and_invert_bits(16, &a)?;
        assert_eq!(b, BigUint::from_slice(&[0xffffu32]));

        let b = extend_and_invert_bits(20, &a)?;
        assert_eq!(b, BigUint::from_slice(&[0xfffffu32]));

        let b = extend_and_invert_bits(8, &a)?;
        assert_eq!(b, BigUint::from_slice(&[0xffu32]));

        let b = extend_and_invert_bits(9, &a)?;
        assert_eq!(b, BigUint::from_slice(&[0x1ffu32]));

        assert!(extend_and_invert_bits(3, &BigUint::from(10u32)).is_err());
        Ok(())
    }

    #[test]
    fn write_bit() -> Result<(), TonCellError> {
        let mut writer = CellBuilder::new();
        let cell = writer.store_bit(true)?.build()?;
        assert_eq!(cell.data, [0b1000_0000]);
        assert_eq!(cell.bit_len, 1);
        let mut reader = cell.parser();
        let result = reader.load_bit()?;
        assert!(result);
        Ok(())
    }

    #[test]
    fn write_u8() -> Result<(), TonCellError> {
        let value = 234u8;
        let mut writer = CellBuilder::new();
        let cell = writer.store_u8(8, value)?.build()?;
        assert_eq!(cell.data, [0b1110_1010]);
        assert_eq!(cell.bit_len, 8);
        let mut reader = cell.parser();
        let result = reader.load_u8(8)?;
        assert_eq!(result, value);
        Ok(())
    }

    #[test]
    fn write_u32() -> Result<(), TonCellError> {
        let value = 0xFAD45AADu32;
        let mut writer = CellBuilder::new();
        let cell = writer.store_u32(32, value)?.build()?;
        assert_eq!(cell.data, [0xFA, 0xD4, 0x5A, 0xAD]);
        assert_eq!(cell.bit_len, 32);
        let mut reader = cell.parser();
        let result = reader.load_u32(32)?;
        assert_eq!(result, value);
        Ok(())
    }

    #[test]
    fn write_u64() -> Result<(), TonCellError> {
        let value = 0xFAD45AADAA12FF45;
        let mut writer = CellBuilder::new();
        let cell = writer.store_u64(64, value)?.build()?;
        assert_eq!(cell.data, [0xFA, 0xD4, 0x5A, 0xAD, 0xAA, 0x12, 0xFF, 0x45]);
        assert_eq!(cell.bit_len, 64);
        let mut reader = cell.parser();
        let result = reader.load_u64(64)?;
        assert_eq!(result, value);
        Ok(())
    }

    #[test]
    fn write_slice() -> Result<(), TonCellError> {
        let value = [0xFA, 0xD4, 0x5A, 0xAD, 0xAA, 0x12, 0xFF, 0x45];
        let mut writer = CellBuilder::new();
        let cell = writer.store_slice(&value)?.build()?;
        assert_eq!(cell.data, value);
        assert_eq!(cell.bit_len, 64);
        let mut reader = cell.parser();
        let bytes = reader.load_bytes(8)?;
        assert_eq!(bytes, value);
        Ok(())
    }

    #[test]
    fn write_str() -> Result<(), TonCellError> {
        let texts = ["hello", "Русский текст", "中华人民共和国", "\u{263A}😃"];
        for text in texts {
            let mut writer = CellBuilder::new();
            let cell = writer.store_string(text)?.build()?;
            let text_bytes = text.as_bytes();
            assert_eq!(cell.data, text_bytes);
            assert_eq!(cell.bit_len, text_bytes.len() * 8);
            let mut reader = cell.parser();
            let remaining_bytes = reader.remaining_bytes();
            let result = reader.load_utf8(remaining_bytes)?;
            assert_eq!(result, text);
        }
        Ok(())
    }

    #[test]
    fn write_address() -> Result<(), TonCellError> {
        let addr = TonAddress::from_base64_url("EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR")
            .unwrap();

        let mut writer = CellBuilder::new();
        let cell = writer.store_address(&addr)?.build()?;
        assert_eq!(
            cell.data,
            [
                128, 28, 155, 42, 157, 243, 233, 194, 74, 20, 77, 107, 119, 90, 237, 67, 155, 162,
                249, 250, 17, 117, 117, 173, 233, 132, 124, 110, 68, 225, 93, 237, 238, 192
            ]
        );
        assert_eq!(cell.bit_len, 2 + 1 + 8 + 32 * 8);
        let mut reader = cell.parser();
        let result = reader.load_address()?;
        assert_eq!(result, addr);
        Ok(())
    }

    #[test]
    fn write_big_int() -> Result<(), TonCellError> {
        let value = BigInt::from_str("3").unwrap();
        let mut writer = CellBuilder::new();
        writer.store_int(33, &value)?;
        let cell = writer.build()?;
        println!("cell: {:?}", cell);
        let written = BigInt::from_bytes_be(Sign::Plus, &cell.data);
        assert_eq!(written, value);

        // 256 bits (+ sign)
        let value = BigInt::from_str(
            "97887266651548624282413032824435501549503168134499591480902563623927645013201",
        )
        .unwrap();
        let mut writer = CellBuilder::new();
        writer.store_int(257, &value)?;
        let cell = writer.build()?;
        println!("cell: {:?}", cell);
        let written = BigInt::from_bytes_be(Sign::Plus, &cell.data);
        assert_eq!(written, value);

        let value = BigInt::from_str("-5").unwrap();
        let mut writer = CellBuilder::new();
        writer.store_int(5, &value)?;
        let cell = writer.build()?;
        println!("cell: {:?}", cell);
        let written = BigInt::from_bytes_be(Sign::Plus, &cell.data[1..]);
        let expected = BigInt::from_bytes_be(Sign::Plus, &[0xB0u8]);
        assert_eq!(written, expected);
        Ok(())
    }

    #[test]
    fn write_load_big_uint() -> Result<(), TonCellError> {
        let value = BigUint::from_str("3").unwrap();
        let mut writer = CellBuilder::new();
        assert!(writer.store_uint(1, &value).is_err());
        let bits_for_tests = [256, 128, 64, 8];

        for bits_num in bits_for_tests.iter() {
            writer.store_uint(*bits_num, &value)?;
        }
        let cell = writer.build()?;
        println!("cell: {:?}", cell);
        let mut cell_parser = cell.parser();
        for bits_num in bits_for_tests.iter() {
            let written_value = cell_parser.load_uint(*bits_num)?;
            assert_eq!(written_value, value);
        }

        // 256 bit
        let value = BigUint::from_str(
            "97887266651548624282413032824435501549503168134499591480902563623927645013201",
        )
        .unwrap();
        let mut writer = CellBuilder::new();
        assert!(writer.store_uint(255, &value).is_err());
        let bits_for_tests = [496, 264, 256];
        for bits_num in bits_for_tests.iter() {
            writer.store_uint(*bits_num, &value)?;
        }
        let cell = writer.build()?;
        let mut cell_parser = cell.parser();
        println!("cell: {:?}", cell);
        for bits_num in bits_for_tests.iter() {
            let written_value = cell_parser.load_uint(*bits_num)?;
            println!("RES: {:?},{:?}", written_value, value);

            assert_eq!(written_value, value);
        }

        Ok(())
    }

    #[test]
    fn test_padding() -> Result<(), TonCellError> {
        let mut writer = CellBuilder::new();
        writer.store_uint(32, &BigUint::zero())?; //deadline

        let cell = writer.build()?;

        println!("{:?}", cell);
        assert_eq!(cell.data.len(), 4);
        assert_eq!(cell.bit_len, 32);
        Ok(())
    }

    #[test]
    fn test_padding_r() -> Result<(), TonCellError> {
        let mut writer = CellBuilder::new();
        writer.store_uint(32, &BigUint::zero())?; //deadline
        writer.store_address(&TonAddress::null())?; //recipientAddress
        writer.store_address(&TonAddress::null())?; //referralAddress
        writer.store_bit(false)?; //fulfillPayload
        writer.store_bit(false)?; //rejectPayload
        let cell = writer.build()?;
        println!("{:?}", cell);

        assert_eq!(cell.data.len(), 5);
        assert_eq!(cell.bit_len, 38);
        Ok(())
    }
}
