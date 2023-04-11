use crate::binary::reader::BinaryReader;
use anyhow::{anyhow, bail};
use bitstream_io::{BigEndian, BitWrite, BitWriter};
use crc::Crc;
use lazy_static::lazy_static;

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
    pub(crate) fn parse(serial: &[u8]) -> anyhow::Result<RawBagOfCells> {
        let mut reader: BinaryReader = BinaryReader::new(serial);
        let magic = reader.read_u32_be()?;
        let (has_idx, hash_crc32, _has_cache_bits, _flags, size_bytes) = match magic {
            GENERIC_BOC_MAGIC => {
                let flags_byte = reader.read_u8()?;
                let has_idx = flags_byte & 0x80 == 0x80;
                let hash_crc32 = flags_byte & 0x40 == 0x40;
                let has_cache_bits = flags_byte & 0x20 == 0x20;
                let flags = (flags_byte >> 3) & 0x03;
                let size_bytes = flags_byte & 0x07;
                (
                    has_idx,
                    hash_crc32,
                    has_cache_bits,
                    flags,
                    size_bytes as usize,
                )
            }
            _ => {
                return Err(anyhow!("Unsupported cell1 magic number: {:08x}", magic));
            }
        };
        let offset_bytes = reader.read_u8()? as usize;
        let num_cells = reader.read_var_size_be(size_bytes)?;
        let num_roots = reader.read_var_size_be(size_bytes)?;
        let _num_absent = reader.read_var_size_be(size_bytes)?;
        let total_cells_size = reader.read_var_size_be(offset_bytes)?;
        let roots: Vec<usize> = (0..num_roots)
            .map(|_| reader.read_var_size_be(size_bytes))
            .collect::<anyhow::Result<Vec<usize>>>()?;
        let _index: Vec<usize> = if has_idx {
            (0..num_cells)
                .map(|_| reader.read_var_size_be(size_bytes))
                .collect::<anyhow::Result<Vec<usize>>>()?
        } else {
            Vec::new()
        };
        let hash_len: usize = if hash_crc32 { 4 } else { 0 };
        let expected_len = reader.position() as usize + total_cells_size + hash_len;
        if serial.len() != expected_len {
            return Err(anyhow!(
                "Invalid len, expected {}, actual {}",
                expected_len,
                serial.len()
            ));
        }
        // TODO: Check crc32
        let mut cells: Vec<RawCell> = Vec::new();
        while (reader.position() as usize) < serial.len() - hash_len {
            let raw_cell = read_raw_cell(&mut reader, size_bytes)?;
            cells.push(raw_cell);
        }
        Ok(RawBagOfCells { cells, roots })
    }

    pub(crate) fn serialize(&self, has_crc32: bool) -> anyhow::Result<Vec<u8>> {
        //Based on https://github.com/toncenter/tonweb/blob/c2d5d0fc23d2aec55a0412940ce6e580344a288c/src/boc/Cell.js#L198

        if self.roots.len() > 1 {
            bail!("So far only a single root cells are supported");
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

        writer.write(32, GENERIC_BOC_MAGIC)?;

        //write flags byte
        let has_idx = false;
        let has_cache_bits = false;
        let flags: u8 = 0;
        writer.write_bit(has_idx)?;
        writer.write_bit(has_crc32)?;
        writer.write_bit(has_cache_bits)?;
        writer.write(2, flags)?;
        writer.write(3, num_ref_bytes)?;

        writer.write(8, num_offset_bytes)?;
        writer.write(8 * num_ref_bytes, self.cells.len() as u32)?;
        writer.write(8 * num_ref_bytes, 1)?; // One root for now
        writer.write(8 * num_ref_bytes, 0)?; // Complete BOCs only
        writer.write(8 * num_offset_bytes, full_size)?;
        writer.write(8 * num_ref_bytes, 0)?; // Root should have index 0

        for cell in &self.cells {
            write_raw_cell(&mut writer, cell, num_ref_bytes)?;
        }

        if has_crc32 {
            let bytes = writer
                .writer()
                .ok_or(anyhow!("Stream is not byte-aligned"))?;
            let cs = CRC_32_ISCSI.checksum(bytes.as_slice());
            writer.write_bytes(cs.to_le_bytes().as_slice())?;
        }
        writer.byte_align()?;
        let res = writer
            .writer()
            .ok_or(anyhow!("Stream is not byte-aligned"))?;
        Ok(res.clone())
    }
}

fn read_raw_cell(reader: &mut BinaryReader, size_bytes: usize) -> anyhow::Result<RawCell> {
    let d1 = reader.read_u8()?;
    let d2 = reader.read_u8()?;
    let max_level = d1 / 32;
    let _is_exotic = d1 & 8 == 8;
    let ref_num = d1 & 0x07;
    let data_size = (d2 + 1) / 2;
    let full_bytes = d2 & 0x01 == 0;
    let mut data = vec![0; data_size as usize];
    reader.read_bytes(data.as_mut_slice())?;
    let data_len = data.len();
    let padding_len = if data_len > 0 && !full_bytes {
        // Fix last byte,
        // see https://github.com/toncenter/tonweb/blob/c2d5d0fc23d2aec55a0412940ce6e580344a288c/src/boc/BitString.js#L302
        let num_zeros = data[data_len - 1].trailing_zeros();
        if num_zeros >= 8 {
            return Err(anyhow!(
                "Last byte of binary must not be zero if full_byte flag is not set"
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
        references.push(reader.read_var_size_be(size_bytes)?);
    }
    let res = RawCell {
        data,
        bit_len,
        references,
        max_level,
    };
    Ok(res)
}

fn raw_cell_size(cell: &RawCell, ref_size_bytes: u32) -> u32 {
    let data_len = (cell.bit_len + 7) / 8;
    2 + data_len as u32 + cell.references.len() as u32 * ref_size_bytes
}

fn write_raw_cell(
    writer: &mut BitWriter<Vec<u8>, BigEndian>,
    cell: &RawCell,
    ref_size_bytes: u32,
) -> anyhow::Result<()> {
    let level = 0u32; // TODO: Support
    let is_exotic = 0u32; // TODO: Support
    let num_refs = cell.references.len() as u32;
    let d1 = num_refs + is_exotic * 8 + level * 32;

    let padding_bits = cell.bit_len % 8;
    let full_bytes = padding_bits == 0;
    let data = cell.data.as_slice();
    let data_len = (cell.bit_len + 7) / 8;
    let d2 = data_len as u8 * 2 - if full_bytes { 0 } else { 1 }; //subtract 1 if the last byte is not full

    writer.write(8, d1)?;
    writer.write(8, d2)?;
    if !full_bytes {
        writer.write_bytes(&data[..data_len - 1])?;
        let last_byte = data[data_len - 1];
        let l = last_byte | (1 << 8 - padding_bits - 1);
        writer.write(8, l)?;
    } else {
        writer.write_bytes(data)?;
    }

    for r in cell.references.as_slice() {
        writer.write(8 * ref_size_bytes, *r as u32)?; // One root for now
    }

    Ok(())
}
