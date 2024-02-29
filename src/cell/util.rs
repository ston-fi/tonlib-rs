use std::io;

use bitstream_io::{BitRead, BitReader, Endianness};

use crate::cell::{MapTonCellError, TonCellError};

pub trait BitReadExt {
    fn read_bits(&mut self, num_bits: usize, slice: &mut [u8]) -> Result<(), TonCellError>;
}

impl<R: io::Read, E: Endianness> BitReadExt for BitReader<R, E> {
    fn read_bits(&mut self, num_bits: usize, slice: &mut [u8]) -> Result<(), TonCellError> {
        let total_bytes = (num_bits + 7) / 8;
        if total_bytes > slice.len() {
            let msg = format!(
                "Attempt to read {} bits into buffer {} bytes",
                num_bits,
                slice.len()
            );
            return Err(TonCellError::CellParserError(msg));
        }
        let full_bytes = (num_bits) / 8;
        self.read_bytes(&mut slice[0..full_bytes])
            .map_cell_parser_error()?;
        let last_byte_len = num_bits % 8;
        if last_byte_len != 0 {
            let last_byte = self
                .read::<u8>(last_byte_len as u32)
                .map_cell_parser_error()?;
            slice[full_bytes] = last_byte << (8 - last_byte_len);
        }
        Ok(())
    }
}
