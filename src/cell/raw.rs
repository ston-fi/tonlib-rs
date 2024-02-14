use std::io::Cursor;

use bitstream_io::{BigEndian, BitWrite, BitWriter, ByteRead, ByteReader};
use crc::Crc;
use lazy_static::lazy_static;

use crate::cell::{MapTonCellError, TonCellError};

lazy_static! {
    pub static ref CRC_32_ISCSI: Crc<u32> = Crc::<u32>::new(&crc::CRC_32_ISCSI);
}

/// Raw representation of Cell.
///
/// References are stored as indices in BagOfCells.
#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub(crate) struct RawCell {
    pub(crate) data: Vec<u8>,
    pub(crate) bit_len: usize,
    pub(crate) references: Vec<usize>,
    pub(crate) max_level: u8,
}

/// Raw representation of BagOfCells.
///
/// `cells` must be topologically sorted.
#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub(crate) struct RawBagOfCells {
    pub(crate) cells: Vec<RawCell>,
    pub(crate) roots: Vec<usize>,
}

const GENERIC_BOC_MAGIC: u32 = 0xb5ee9c72;
const _INDEXED_BOC_MAGIC: u32 = 0x68ff65f3;
const _INDEXED_CRC32_MAGIC: u32 = 0xacc3a728;

impl RawBagOfCells {
    pub(crate) fn parse(serial: &[u8]) -> Result<RawBagOfCells, TonCellError> {
        let cursor = Cursor::new(serial);

        let mut reader: ByteReader<Cursor<&[u8]>, BigEndian> =
            ByteReader::endian(cursor, BigEndian);
        // serialized_boc#b5ee9c72
        let magic = reader.read::<u32>().map_boc_deserialization_error()?;

        let (has_idx, has_crc32c, _has_cache_bits, size) = match magic {
            GENERIC_BOC_MAGIC => {
                // has_idx:(## 1) has_crc32c:(## 1) has_cache_bits:(## 1) flags:(## 2) { flags = 0 }
                let header = reader.read::<u8>().map_boc_deserialization_error()?;
                let has_idx = (header >> 7) & 1 == 1;
                let has_crc32c = (header >> 6) & 1 == 1;
                let has_cache_bits = (header >> 5) & 1 == 1;
                // size:(## 3) { size <= 4 }
                let size = header & 0b0000_0111;

                (has_idx, has_crc32c, has_cache_bits, size)
            }
            magic => {
                return Err(TonCellError::boc_deserialization_error(format!(
                    "Unsupported cell magic number: {:#}",
                    magic
                )));
            }
        };
        //   off_bytes:(## 8) { off_bytes <= 8 }
        let off_bytes = reader.read::<u8>().map_boc_deserialization_error()?;
        //cells:(##(size * 8))
        let cells = read_var_size(&mut reader, size)?;
        //   roots:(##(size * 8)) { roots >= 1 }
        let roots = read_var_size(&mut reader, size)?;
        //   absent:(##(size * 8)) { roots + absent <= cells }
        let _absent = read_var_size(&mut reader, size)?;
        //   tot_cells_size:(##(off_bytes * 8))
        let _tot_cells_size = read_var_size(&mut reader, off_bytes)?;
        //   root_list:(roots * ##(size * 8))
        let mut root_list = vec![];
        for _ in 0..roots {
            root_list.push(read_var_size(&mut reader, size)?)
        }
        //   index:has_idx?(cells * ##(off_bytes * 8))
        let mut index = vec![];
        if has_idx {
            for _ in 0..cells {
                index.push(read_var_size(&mut reader, off_bytes)?)
            }
        }
        //   cell_data:(tot_cells_size * [ uint8 ])
        let mut cell_vec = Vec::with_capacity(cells);

        for _ in 0..cells {
            let cell = read_cell(&mut reader, size)?;
            cell_vec.push(cell);
        }
        //   crc32c:has_crc32c?uint32
        let _crc32c = if has_crc32c {
            reader.read::<u32>().map_boc_deserialization_error()?
        } else {
            0
        };
        // TODO: Check crc32

        Ok(RawBagOfCells {
            cells: cell_vec,
            roots: root_list,
        })
    }

    pub(crate) fn serialize(&self, has_crc32: bool) -> Result<Vec<u8>, TonCellError> {
        //Based on https://github.com/toncenter/tonweb/blob/c2d5d0fc23d2aec55a0412940ce6e580344a288c/src/boc/Cell.js#L198

        let root_count = self.roots.len();
        if root_count > 1 {
            return Err(TonCellError::boc_serialization_error(format!(
                "Single root expected, got {}",
                root_count
            )));
        }

        let num_ref_bits = 32 - (self.cells.len() as u32).leading_zeros();
        let num_ref_bytes = (num_ref_bits + 7) / 8;

        let mut full_size = 0u32;
        let mut index = Vec::<u32>::with_capacity(self.cells.len());
        for cell in &self.cells {
            index.push(full_size);
            full_size += raw_cell_size(cell, num_ref_bytes);
        }

        let num_offset_bits = 32 - full_size.leading_zeros();
        let num_offset_bytes = (num_offset_bits + 7) / 8;

        let mut writer = BitWriter::endian(Vec::new(), BigEndian);

        writer
            .write(32, GENERIC_BOC_MAGIC)
            .map_boc_serialization_error()?;

        //write flags byte
        let has_idx = false;
        let has_cache_bits = false;
        let flags: u8 = 0;
        writer.write_bit(has_idx).map_boc_serialization_error()?;
        writer.write_bit(has_crc32).map_boc_serialization_error()?;
        writer
            .write_bit(has_cache_bits)
            .map_boc_serialization_error()?;
        writer.write(2, flags).map_boc_serialization_error()?;
        writer
            .write(3, num_ref_bytes)
            .map_boc_serialization_error()?;
        writer
            .write(8, num_offset_bytes)
            .map_boc_serialization_error()?;
        writer
            .write(8 * num_ref_bytes, self.cells.len() as u32)
            .map_boc_serialization_error()?;
        writer
            .write(8 * num_ref_bytes, 1)
            .map_boc_serialization_error()?; // One root for now
        writer
            .write(8 * num_ref_bytes, 0)
            .map_boc_serialization_error()?; // Complete BOCs only
        writer
            .write(8 * num_offset_bytes, full_size)
            .map_boc_serialization_error()?;
        writer
            .write(8 * num_ref_bytes, 0)
            .map_boc_serialization_error()?; // Root should have index 0

        for cell in &self.cells {
            write_raw_cell(&mut writer, cell, num_ref_bytes)?;
        }

        if has_crc32 {
            let bytes = writer.writer().ok_or_else(|| {
                TonCellError::boc_serialization_error("Stream is not byte-aligned")
            })?;
            let cs = CRC_32_ISCSI.checksum(bytes.as_slice());
            writer
                .write_bytes(cs.to_le_bytes().as_slice())
                .map_boc_serialization_error()?;
        }
        writer.byte_align().map_boc_serialization_error()?;
        let res = writer
            .writer()
            .ok_or_else(|| TonCellError::boc_serialization_error("Stream is not byte-aligned"))?;
        Ok(res.clone())
    }
}

fn read_cell(
    reader: &mut ByteReader<Cursor<&[u8]>, BigEndian>,
    size: u8,
) -> Result<RawCell, TonCellError> {
    let d1 = reader.read::<u8>().map_boc_deserialization_error()?;
    let d2 = reader.read::<u8>().map_boc_deserialization_error()?;

    let max_level = d1 >> 5;
    let _is_exotic = (d1 & 8) != 0;
    let ref_num = d1 & 0x07;
    let data_size = ((d2 >> 1) + (d2 & 1)).into();
    let full_bytes = (d2 & 0x01) == 0;

    let mut data = reader
        .read_to_vec(data_size)
        .map_boc_deserialization_error()?;

    let data_len = data.len();
    let padding_len = if data_len > 0 && !full_bytes {
        // Fix last byte,
        // see https://github.com/toncenter/tonweb/blob/c2d5d0fc23d2aec55a0412940ce6e580344a288c/src/boc/BitString.js#L302
        let num_zeros = data[data_len - 1].trailing_zeros();
        if num_zeros >= 8 {
            return Err(TonCellError::boc_deserialization_error(
                "Last byte of binary must not be zero if full_byte flag is not set",
            ));
        }
        data[data_len - 1] &= !(1 << num_zeros);
        num_zeros + 1
    } else {
        0
    };
    let bit_len = data.len() * 8 - padding_len as usize;
    let mut references: Vec<usize> = Vec::new();
    for _ in 0..ref_num {
        references.push(read_var_size(reader, size)?);
    }
    let cell = RawCell {
        data,
        bit_len,
        references,
        max_level,
    };
    Ok(cell)
}

fn raw_cell_size(cell: &RawCell, ref_size_bytes: u32) -> u32 {
    let data_len = (cell.bit_len + 7) / 8;
    2 + data_len as u32 + cell.references.len() as u32 * ref_size_bytes
}

fn write_raw_cell(
    writer: &mut BitWriter<Vec<u8>, BigEndian>,
    cell: &RawCell,
    ref_size_bytes: u32,
) -> Result<(), TonCellError> {
    let level = 0u32; // TODO: Support
    let is_exotic = 0u32; // TODO: Support
    let num_refs = cell.references.len() as u32;
    let d1 = num_refs + is_exotic * 8 + level * 32;

    let padding_bits = cell.bit_len % 8;
    let full_bytes = padding_bits == 0;
    let data = cell.data.as_slice();
    let data_len = (cell.bit_len + 7) / 8;
    let d2 = data_len as u8 * 2 - if full_bytes { 0 } else { 1 }; //subtract 1 if the last byte is not full

    writer.write(8, d1).map_boc_serialization_error()?;
    writer.write(8, d2).map_boc_serialization_error()?;
    if !full_bytes {
        writer
            .write_bytes(&data[..data_len - 1])
            .map_boc_serialization_error()?;
        let last_byte = data[data_len - 1];
        let l = last_byte | 1 << (8 - padding_bits - 1);
        writer.write(8, l).map_boc_serialization_error()?;
    } else {
        writer.write_bytes(data).map_boc_serialization_error()?;
    }

    for r in cell.references.as_slice() {
        writer
            .write(8 * ref_size_bytes, *r as u32)
            .map_boc_serialization_error()?;
        // One root for now
    }

    Ok(())
}

fn read_var_size(
    reader: &mut ByteReader<Cursor<&[u8]>, BigEndian>,
    n: u8,
) -> Result<usize, TonCellError> {
    let bytes = reader
        .read_to_vec(n.into())
        .map_boc_deserialization_error()?;

    let mut result = 0;
    for &byte in &bytes {
        result <<= 8;
        result |= usize::from(byte);
    }
    Ok(result)
}
