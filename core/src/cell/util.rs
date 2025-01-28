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

// return false if preconditions are not met
pub fn rewrite_bits(
    src: &[u8],
    src_offset_bits: usize,
    dst: &mut [u8],
    dst_offset_bits: usize,
    len: usize,
) -> bool {
    // Calculate total bits available in source and destination
    let src_total_bits = src.len() * 8;
    let dst_total_bits = dst.len() * 8;

    // Check preconditions
    if src_offset_bits + len > src_total_bits || dst_offset_bits + len > dst_total_bits {
        return false;
    }

    for i in 0..len {
        // Calculate the source bit position and extract the bit
        let src_bit_pos = src_offset_bits + i;
        let src_byte_index = src_bit_pos / 8;
        let src_bit_offset = 7 - (src_bit_pos % 8); // MSB is bit 7
        let src_bit = (src[src_byte_index] >> src_bit_offset) & 1;

        // Calculate the destination bit position and write the bit
        let dst_bit_pos = dst_offset_bits + i;
        let dst_byte_index = dst_bit_pos / 8;
        let dst_bit_offset = 7 - (dst_bit_pos % 8); // MSB is bit 7

        // Clear the target bit and set it to the source bit value
        dst[dst_byte_index] &= !(1 << dst_bit_offset); // Clear the bit
        dst[dst_byte_index] |= src_bit << dst_bit_offset; // Set the bit
    }

    true
}

#[cfg(test)]
mod tests {
    use crate::cell::rewrite_bits;

    #[test]
    fn test_rewrite_bits() {
        let src = vec![0b11001100, 0b10101010]; // Source bits
        let mut dst = vec![0b00000000, 0b00000000]; // Destination bits
        assert!(rewrite_bits(&src, 4, &mut dst, 8, 8));
        assert_eq!(dst, vec![0b00000000, 0b11001010]);

        let src = vec![0b11001100, 0b10101010]; // Source bits
        let mut dst = vec![0b00000000, 0b00000000]; // Destination bits
        assert!(rewrite_bits(&src, 0, &mut dst, 0, 16));
        assert_eq!(dst, src);

        let src = vec![0b11001100, 0b10101010]; // Source bits
        let mut dst = vec![0b00000000, 0b00000000]; // Destination bits
        assert!(rewrite_bits(&src, 0, &mut dst, 0, 8));
        assert_eq!(dst[0], src[0]);
        assert_eq!(dst[1], 0b00000000);

        assert!(!rewrite_bits(&src, 14, &mut dst, 6, 10));
    }
}
