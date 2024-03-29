use std::sync::Arc;

use bitstream_io::{BigEndian, BitWrite, BitWriter};
use num_bigint::{BigInt, BigUint};
use num_traits::Zero;

use crate::address::TonAddress;
use crate::cell::error::{MapTonCellError, TonCellError};
use crate::cell::{ArcCell, Cell, CellParser};

const MAX_CELL_BITS: usize = 1023;
const MAX_CELL_REFERENCES: usize = 4;

pub struct CellBuilder {
    bit_writer: BitWriter<Vec<u8>, BigEndian>,
    references: Vec<ArcCell>,
}

impl CellBuilder {
    pub fn new() -> CellBuilder {
        let bit_writer = BitWriter::endian(Vec::new(), BigEndian);
        CellBuilder {
            bit_writer,
            references: Vec::new(),
        }
    }

    pub fn store_bit(&mut self, val: bool) -> Result<&mut Self, TonCellError> {
        self.bit_writer.write_bit(val).map_cell_builder_error()?;
        Ok(self)
    }

    pub fn store_u8(&mut self, bit_len: usize, val: u8) -> Result<&mut Self, TonCellError> {
        self.bit_writer
            .write(bit_len as u32, val)
            .map_cell_builder_error()?;
        Ok(self)
    }

    pub fn store_i8(&mut self, bit_len: usize, val: i8) -> Result<&mut Self, TonCellError> {
        self.bit_writer
            .write(bit_len as u32, val)
            .map_cell_builder_error()?;
        Ok(self)
    }

    pub fn store_u32(&mut self, bit_len: usize, val: u32) -> Result<&mut Self, TonCellError> {
        self.bit_writer
            .write(bit_len as u32, val)
            .map_cell_builder_error()?;
        Ok(self)
    }

    pub fn store_i32(&mut self, bit_len: usize, val: i32) -> Result<&mut Self, TonCellError> {
        self.bit_writer
            .write(bit_len as u32, val)
            .map_cell_builder_error()?;
        Ok(self)
    }

    pub fn store_u64(&mut self, bit_len: usize, val: u64) -> Result<&mut Self, TonCellError> {
        self.bit_writer
            .write(bit_len as u32, val)
            .map_cell_builder_error()?;
        Ok(self)
    }

    pub fn store_i64(&mut self, bit_len: usize, val: i64) -> Result<&mut Self, TonCellError> {
        self.bit_writer
            .write(bit_len as u32, val)
            .map_cell_builder_error()?;
        Ok(self)
    }

    pub fn store_uint(&mut self, bit_len: usize, val: &BigUint) -> Result<&mut Self, TonCellError> {
        if val.bits() as usize > bit_len {
            return Err(TonCellError::cell_builder_error(format!(
                "Value {} doesn't fit in {} bits",
                val, bit_len
            )));
        }
        let bytes = val.to_bytes_be();
        let num_full_bytes = bit_len / 8;
        let num_bits_in_high_byte = bit_len % 8;
        if bytes.len() > num_full_bytes + 1 {
            return Err(TonCellError::cell_builder_error(format!(
                "Internal error: can't fit {} into {} bits ",
                val, bit_len
            )));
        }
        if num_bits_in_high_byte > 0 {
            let high_byte: u8 = if bytes.len() == num_full_bytes + 1 {
                bytes[0]
            } else {
                0
            };
            self.store_u8(num_bits_in_high_byte, high_byte)?;
        }
        let num_empty_bytes = num_full_bytes - bytes.len();
        for _ in 0..num_empty_bytes {
            self.store_byte(0)?;
        }
        for b in bytes {
            self.store_byte(b)?;
        }
        Ok(self)
    }

    pub fn store_int(&mut self, bit_len: usize, val: &BigInt) -> Result<&mut Self, TonCellError> {
        if val.bits() as usize > bit_len {
            return Err(TonCellError::cell_builder_error(format!(
                "Value {} doesn't fit in {} bits",
                val, bit_len
            )));
        }
        let bytes = val.to_signed_bytes_be();
        let num_full_bytes = bit_len / 8;
        let num_bits_in_high_byte = bit_len % 8;
        if bytes.len() > num_full_bytes + 1 {
            return Err(TonCellError::cell_builder_error(format!(
                "Internal error: can't fit {} into {} bits ",
                val, bit_len
            )));
        }
        if num_bits_in_high_byte > 0 {
            let high_byte: u8 = if bytes.len() == num_full_bytes + 1 {
                bytes[0]
            } else {
                0
            };
            self.store_u8(num_bits_in_high_byte, high_byte)?;
        }
        let num_empty_bytes = num_full_bytes - bytes.len();
        for _ in 0..num_empty_bytes {
            self.store_byte(0)?;
        }
        for b in bytes {
            self.store_byte(b)?;
        }
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
            Ok(Cell {
                data: vec.clone(),
                bit_len,
                references: self.references.clone(),
            })
        } else {
            Err(TonCellError::CellBuilderError(
                "Stream is not byte-aligned".to_string(),
            ))
        }
    }
}

impl Default for CellBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::address::TonAddress;
    use crate::cell::CellBuilder;

    #[test]
    fn write_bit() -> anyhow::Result<()> {
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
    fn write_u8() -> anyhow::Result<()> {
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
    fn write_u32() -> anyhow::Result<()> {
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
    fn write_u64() -> anyhow::Result<()> {
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
    fn write_slice() -> anyhow::Result<()> {
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
    fn write_str() -> anyhow::Result<()> {
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
    fn write_address() -> anyhow::Result<()> {
        let addr = TonAddress::from_base64_url("EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR")?;

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
}
