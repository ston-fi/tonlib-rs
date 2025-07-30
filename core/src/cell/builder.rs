use std::cmp::min;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

use bitstream_io::{BigEndian, BitWrite, BitWriter};
use num_bigint::{BigInt, BigUint};
use num_traits::Zero;

use super::TonCellNum;
use crate::cell::dict::{DictBuilder, ValWriter};
use crate::cell::error::{MapTonCellError, TonCellError};
use crate::cell::{ArcCell, Cell, CellParser};
use crate::tlb_types::block::msg_address::MsgAddress;
use crate::tlb_types::tlb::TLB;
use crate::{TonAddress, TonHash};

pub(crate) const MAX_CELL_BITS: usize = 1023;
pub(crate) const MAX_CELL_REFERENCES: usize = 4;
pub(crate) const MAX_LEVEL_MASK: u32 = 3;

pub struct CellBuilder {
    bit_writer: BitWriter<Vec<u8>, BigEndian>,
    bits_to_write: usize,
    references: Vec<ArcCell>,
    is_cell_exotic: bool,
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum EitherCellLayout {
    Native,
    ToRef,
    ToCell,
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

    pub fn store_number_optional<N: TonCellNum>(
        &mut self,
        bit_len: usize,
        maybe_val: Option<N>,
    ) -> Result<&mut Self, TonCellError> {
        if let Some(val) = maybe_val {
            self.store_bit(true)?;
            self.store_number(bit_len, &val)?;
        } else {
            self.store_bit(false)?;
        }
        Ok(self)
    }

    pub fn store_u8(&mut self, bit_len: usize, val: u8) -> Result<&mut Self, TonCellError> {
        self.store_number(bit_len, &val)
    }

    pub fn store_i8(&mut self, bit_len: usize, val: i8) -> Result<&mut Self, TonCellError> {
        self.store_number(bit_len, &val)
    }

    pub fn store_u16(&mut self, bit_len: usize, val: u16) -> Result<&mut Self, TonCellError> {
        self.store_number(bit_len, &val)
    }

    pub fn store_u32(&mut self, bit_len: usize, val: u32) -> Result<&mut Self, TonCellError> {
        self.store_number(bit_len, &val)
    }

    pub fn store_i32(&mut self, bit_len: usize, val: i32) -> Result<&mut Self, TonCellError> {
        self.store_number(bit_len, &val)
    }

    pub fn store_u64(&mut self, bit_len: usize, val: u64) -> Result<&mut Self, TonCellError> {
        self.store_number(bit_len, &val)
    }

    pub fn store_i64(&mut self, bit_len: usize, val: i64) -> Result<&mut Self, TonCellError> {
        self.store_number(bit_len, &val)
    }

    pub fn store_uint(&mut self, bit_len: usize, val: &BigUint) -> Result<&mut Self, TonCellError> {
        self.store_number(bit_len, val)
    }

    pub fn store_int(&mut self, bit_len: usize, val: &BigInt) -> Result<&mut Self, TonCellError> {
        self.store_number(bit_len, val)
    }

    pub fn store_byte(&mut self, val: u8) -> Result<&mut Self, TonCellError> {
        self.store_number(8, &val)
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
            let num_bytes = (val.bits() as usize).div_ceil(8);
            self.store_u8(4, num_bytes as u8)?;
            self.store_uint(num_bytes * 8, val)
        }
    }

    /// Stores address without optimizing hole address
    pub fn store_raw_address(&mut self, val: &TonAddress) -> Result<&mut Self, TonCellError> {
        self.store_u8(2, 0b10u8)?; //store as MsgAddressInt
        self.store_bit(false)?; // always no anycast
        let wc = (val.workchain & 0xff) as u8;
        self.store_u8(8, wc)?;
        self.store_slice(val.hash_part.as_slice())?;
        Ok(self)
    }

    /// Stores address optimizing hole address two to bits
    pub fn store_address(&mut self, val: &TonAddress) -> Result<&mut Self, TonCellError> {
        val.to_msg_address().write(self)?;
        Ok(self)
    }

    pub fn store_msg_address(&mut self, val: &MsgAddress) -> Result<&mut Self, TonCellError> {
        val.write(self)?;
        Ok(self)
    }

    /// Adds reference to an existing `Cell`.
    ///
    /// The reference is passed as `ArcCell` so it might be references from other cells.
    pub fn store_reference(&mut self, cell: &ArcCell) -> Result<&mut Self, TonCellError> {
        if self.references.len() == 4 {
            return Err(TonCellError::cell_builder_error("Cell already has 4 refs"));
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

    /// Adds a newly constructed `Cell` as a reference.
    ///
    /// The cell is wrapped it the `Arc`.
    pub fn store_child(&mut self, cell: Cell) -> Result<&mut Self, TonCellError> {
        self.store_reference(&cell.to_arc())
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
        layout: EitherCellLayout,
    ) -> Result<&mut Self, TonCellError> {
        match layout {
            EitherCellLayout::Native => {
                if cell.bit_len() < self.remaining_bits() {
                    self.store_bit(false)?;
                    self.store_cell(cell)?;
                } else {
                    self.store_bit(true)?;
                    self.store_reference(cell)?;
                }
            }
            EitherCellLayout::ToRef => {
                self.store_bit(true)?;
                self.store_reference(cell)?;
            }
            EitherCellLayout::ToCell => {
                self.store_bit(false)?;
                self.store_cell(cell)?;
            }
        }

        Ok(self)
    }

    // https://docs.ton.org/develop/data-formats/tl-b-types#maybe
    pub fn store_ref_cell_optional(
        &mut self,
        maybe_cell: Option<&ArcCell>,
    ) -> Result<&mut Self, TonCellError> {
        if let Some(cell) = maybe_cell {
            self.store_bit(true)?;
            self.store_reference(cell)?;
        } else {
            self.store_bit(false)?;
        }
        Ok(self)
    }

    pub fn store_dict_data<K, V>(
        &mut self,
        key_len_bits: usize,
        value_writer: ValWriter<V>,
        data: HashMap<K, V>,
    ) -> Result<&mut Self, TonCellError>
    where
        BigUint: From<K>,
    {
        if data.is_empty() {
            return Err(TonCellError::CellBuilderError(
                "can't save empty dict as dict_data".to_string(),
            ));
        }
        let dict_builder = DictBuilder::new(key_len_bits, value_writer, data)?;
        let dict_cell = dict_builder.build()?;
        self.store_cell(&dict_cell)
    }

    pub fn store_dict<K, V>(
        &mut self,
        key_len_bits: usize,
        value_writer: ValWriter<V>,
        data: HashMap<K, V>,
    ) -> Result<&mut Self, TonCellError>
    where
        BigUint: From<K>,
    {
        if data.is_empty() {
            self.store_bit(false)
        } else {
            self.store_bit(true)?;

            let dict_data = Arc::new(
                CellBuilder::new()
                    .store_dict_data(key_len_bits, value_writer, data)?
                    .build()?,
            );
            self.store_reference(&dict_data)
        }
    }

    pub fn store_tonhash(&mut self, ton_hash: &TonHash) -> Result<&mut Self, TonCellError> {
        self.store_slice(ton_hash.as_slice())
    }

    pub fn remaining_bits(&self) -> usize {
        MAX_CELL_BITS - self.bits_to_write
    }

    pub fn store_number<N, B>(&mut self, bit_len: usize, data: B) -> Result<&mut Self, TonCellError>
    where
        N: TonCellNum,
        B: Deref<Target = N>,
    {
        let value = data.deref();

        // data is zero
        if bit_len == 0 {
            if value.tcn_is_zero() {
                Ok(self)
            } else {
                Err(TonCellError::CellBuilderError(format!(
                    "Cannot write non-zero number {value} as 0 bits"
                )))
            }
            //data is unsigned primitive
        } else if let Some(unsigned) = value.tcn_to_unsigned_primitive() {
            self.bit_writer.write_var(bit_len as u32, unsigned)?;
            self.bits_to_write += bit_len;
            Ok(self)
            //data is signed or BigInt or BigUint
        } else {
            let min_bits = value.tcn_min_bits_len();
            if min_bits > bit_len {
                Err(TonCellError::CellBuilderError(format!(
                    "Cannot write number {value} in {bit_len} bits (requires at least {min_bits} bits)"
                )))
            } else {
                let bytes = value.tcn_to_bytes();
                let padding_bits = bit_len - min_bits;

                let first_padding_byte = if N::SIGNED && bytes[0] & 0x80 != 0 {
                    0xFF
                } else {
                    0
                };

                if padding_bits > 0 {
                    let pad_bytes = vec![first_padding_byte; padding_bits.div_ceil(8)];
                    self.write_bits(pad_bytes, padding_bits)?;
                }

                let bit_offset = bytes.len() * 8 - min_bits;
                self.write_bits_with_offset(bytes, bit_len - padding_bits, bit_offset)?;
                Ok(self)
            }
        }
    }

    pub fn write_bits_with_offset<T: AsRef<[u8]>>(
        &mut self,
        data: T,
        bit_len: usize,
        bit_offset: usize,
    ) -> Result<&mut Self, TonCellError> {
        self.bits_to_write += bit_len;
        let mut value = data.as_ref();

        if (bit_len + bit_offset).div_ceil(8) > value.len() {
            Err(TonCellError::CellBuilderError(format!(
                "Can't extract {} bits from {} bytes",
                bit_len + bit_offset,
                value.len()
            )))
        } else if bit_len == 0 {
            Ok(self)
        } else {
            // skip bytes_offset, adjust borders
            value = &value[bit_offset / 8..];
            let aligned_bit_offset = bit_offset % 8;

            let first_byte_bits_len = min(bit_len, 8 - aligned_bit_offset);
            let mut first_byte_val = value[0] << aligned_bit_offset >> aligned_bit_offset;
            if first_byte_bits_len == bit_len {
                first_byte_val >>= 8 - aligned_bit_offset - bit_len
            }
            self.bit_writer
                .write_var(first_byte_bits_len as u32, first_byte_val)?;

            value = &value[1..];
            let aligned_bit_len = bit_len - first_byte_bits_len;

            let full_bytes = aligned_bit_len / 8;
            self.bit_writer.write_bytes(&value[0..full_bytes])?;
            let rest_bits_len = aligned_bit_len % 8;
            if rest_bits_len != 0 {
                self.bit_writer.write_var(
                    rest_bits_len as u32,
                    value[full_bytes] >> (8 - rest_bits_len),
                )?;
            }

            Ok(self)
        }
    }

    pub fn write_bits<T: AsRef<[u8]>>(
        &mut self,
        data: T,
        bit_len: usize,
    ) -> Result<&mut Self, TonCellError> {
        self.write_bits_with_offset(data, bit_len, 0)
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
                    "Cell must contain at most {MAX_CELL_BITS} bits, got {bit_len}"
                )));
            }
            let ref_count = self.references.len();
            if ref_count > MAX_CELL_REFERENCES {
                return Err(TonCellError::cell_builder_error(format!(
                    "Cell must contain at most 4 references, got {ref_count}"
                )));
            }

            Cell::new(
                vec.clone(),
                self.bits_to_write,
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

impl Default for CellBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::ops::ShrAssign;
    use std::str::FromStr;

    use num_bigint::{BigInt, BigUint, Sign};
    use num_traits::{FromPrimitive, Num, Zero};

    use crate::cell::dict::predefined_readers::{key_reader_u8, val_reader_uint};
    use crate::cell::{CellBuilder, TonCellError};
    use crate::types::TonAddress;
    use crate::TonHash;

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
        println!("cell: {cell:?}");
        let mut written = BigInt::from_bytes_be(Sign::Plus, &cell.data);
        written.shr_assign(8 - cell.bit_len % 8); // should shift bigint here as cell builder writes unalinged bits

        assert_eq!(written, value);

        // 256 bits (+ sign)
        let value = BigInt::from_str_radix(
            "123456789ABCDEFAA55AA55AA55AA55AA55AA55AA55AA55AA55AA55AA55",
            16,
        )
        .unwrap();
        let mut writer = CellBuilder::new();
        writer.store_int(257, &value)?;
        let cell = writer.build()?;
        println!("cell: {cell:?}");
        let mut written = BigInt::from_bytes_be(Sign::Plus, &cell.data);
        written.shr_assign(8 - cell.bit_len % 8);
        assert_eq!(written, value);

        let value = BigInt::from_str("-5").unwrap();
        let mut writer = CellBuilder::new();
        writer.store_int(5, &value)?;
        let cell = writer.build()?;
        println!("cell: {cell:?}");
        assert_eq!(5, cell.bit_len);
        assert_eq!(0b1101_1000, cell.data[0]);

        let value = BigInt::from_str("-5").unwrap();
        let mut writer = CellBuilder::new();
        writer.store_int(7, &value)?;
        let cell = writer.build()?;
        println!("cell: {cell:?}");
        assert_eq!(7, cell.bit_len);
        assert_eq!(0b1111_0110, cell.data[0]);

        assert!(CellBuilder::new()
            .store_int(32, &BigInt::from(2401234567u32))
            .is_err());
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
        println!("cell: {cell:?}");
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
        println!("cell: {cell:?}");
        for bits_num in bits_for_tests.iter() {
            let written_value = cell_parser.load_uint(*bits_num)?;
            assert_eq!(written_value, value);
        }

        Ok(())
    }

    #[test]
    fn test_padding() -> Result<(), TonCellError> {
        let mut writer = CellBuilder::new();

        let n = BigUint::from(0x55a5f0f0u32);

        writer.store_uint(32, &BigUint::zero())?;
        writer.store_uint(32, &n)?;
        writer.store_uint(31, &BigUint::zero())?;
        writer.store_uint(31, &n)?;
        writer.store_uint(35, &BigUint::zero())?;
        writer.store_uint(35, &n)?;
        let cell = writer.build()?;

        println!("{cell:?}");
        assert_eq!(cell.data.len(), 25);
        assert_eq!(cell.bit_len, 196);

        let mut parser = cell.parser();
        let result_zero = parser.load_uint(32)?;
        let result_test_num = parser.load_uint(32)?;

        assert_eq!(result_zero, BigUint::zero());
        assert_eq!(result_test_num, n);
        let result_zero = parser.load_uint(31)?;
        let result_test_num = parser.load_uint(31)?;

        assert_eq!(result_zero, BigUint::zero());
        assert_eq!(result_test_num, n);
        let result_zero = parser.load_uint(35)?;
        let result_test_num = parser.load_uint(35)?;

        assert_eq!(result_zero, BigUint::zero());

        assert_eq!(result_test_num, n);
        parser.ensure_empty()?;

        Ok(())
    }

    #[test]
    fn test_zero_alone() -> Result<(), TonCellError> {
        let bitlens_to_test = [
            1, 7, 8, 9, 30, 31, 32, 33, 127, 128, 129, 255, 256, 257, 300,
        ];
        for bitlen in bitlens_to_test {
            let mut writer = CellBuilder::new();
            writer.store_uint(bitlen, &BigUint::zero())?;

            let cell = writer.build()?;

            println!("{cell:?}");
            let taeget_bytelen = bitlen.div_ceil(8);
            assert_eq!(cell.data.len(), taeget_bytelen);

            assert_eq!(cell.bit_len, bitlen);

            let mut parser = cell.parser();
            let result_zero = parser.load_uint(bitlen)?;

            assert_eq!(result_zero, BigUint::zero());
            parser.ensure_empty()?;
        }
        Ok(())
    }

    #[test]
    fn test_store_dict() -> Result<(), TonCellError> {
        let mut builder = CellBuilder::new();
        let mut data = HashMap::new();
        data.insert(1u8, BigUint::from(2u8));
        data.insert(3u8, BigUint::from(4u8));

        let value_writer = |writer: &mut CellBuilder, value: BigUint| {
            writer.store_uint(8, &value)?;
            Ok(())
        };
        builder.store_dict(8, value_writer, data.clone())?;
        let cell = builder.build()?;
        let mut parser = cell.parser();
        let parsed = parser.load_dict(8, key_reader_u8, val_reader_uint)?;
        assert_eq!(data, parsed);
        Ok(())
    }

    #[test]
    fn test_store_dict_data_empty() -> Result<(), TonCellError> {
        let mut builder = CellBuilder::new();
        let data: HashMap<BigUint, BigUint> = HashMap::new();
        let value_writer = |writer: &mut CellBuilder, value: BigUint| {
            writer.store_uint(8, &value)?;
            Ok(())
        };
        assert!(builder.store_dict_data(8, value_writer, data).is_err());
        Ok(())
    }

    #[test]
    fn test_store_tonhash() -> Result<(), TonCellError> {
        let mut writer = CellBuilder::new();
        let ton_hash =
            TonHash::from_hex("9f31f4f413a3accb706c88962ac69d59103b013a0addcfaeed5dd73c18fa98a8")?;

        writer.store_tonhash(&ton_hash)?;
        let cell = writer.build()?;
        let mut parser = cell.parser();
        let parsed = parser.load_tonhash()?;
        assert_eq!(ton_hash, parsed);
        parser.ensure_empty()?;
        Ok(())
    }

    #[test]
    fn test_store_load_signed_unaligned() -> Result<(), TonCellError> {
        let mut builder = CellBuilder::new();
        builder.store_bit(false)?;
        builder.store_i8(8, -4)?;
        builder.store_i32(32, -5)?;
        builder.store_i64(64, -6)?;
        builder.store_u32(9, 256)?;
        let cell = builder.build()?;
        let mut parser = cell.parser();
        assert!(!parser.load_bit()?);
        assert_eq!(parser.load_i8(8)?, -4);
        assert_eq!(parser.load_i32(32)?, -5);
        assert_eq!(parser.load_i64(64)?, -6);
        assert_eq!(parser.load_u32(9)?, 256);
        Ok(())
    }

    #[test]
    fn test_store_load_117146891372() -> Result<(), TonCellError> {
        let mut test = CellBuilder::new();
        test.store_number(257, &BigUint::from_u64(117146891372).unwrap())
            .unwrap();
        Ok(())
    }
}
