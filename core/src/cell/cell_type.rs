use std::cmp::PartialEq;
use std::io;
use std::io::Cursor;

use bitstream_io::{BigEndian, ByteRead, ByteReader};

use crate::cell::level_mask::LevelMask;
use crate::cell::{ArcCell, Cell, MapTonCellError, TonCellError, DEPTH_BYTES, MAX_LEVEL};
use crate::types::{TON_HASH_BYTES, ZERO_HASH};
use crate::TonHash;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum CellType {
    #[default]
    Ordinary,
    PrunedBranch,
    Library,
    MerkleProof,
    MerkleUpdate,
}

#[derive(Debug, Clone)]
struct Pruned {
    hash: TonHash,
    depth: u16,
}

impl CellType {
    pub(crate) fn determine_exotic_cell_type(data: &[u8]) -> Result<Self, TonCellError> {
        let Some(type_byte) = data.first() else {
            return Err(TonCellError::InvalidExoticCellData(
                "Not enough data for an exotic cell".to_owned(),
            ));
        };

        let cell_type = match type_byte {
            1 => CellType::PrunedBranch,
            2 => CellType::Library,
            3 => CellType::MerkleProof,
            4 => CellType::MerkleUpdate,
            cell_type => {
                return Err(TonCellError::InvalidExoticCellData(format!(
                    "Invalid first byte in exotic cell data: {}",
                    cell_type
                )))
            }
        };
        Ok(cell_type)
    }

    pub(crate) fn validate(
        &self,
        data: &[u8],
        bit_len: usize,
        references: impl AsRef<[ArcCell]>,
    ) -> Result<(), TonCellError> {
        match self {
            CellType::Ordinary => Ok(()),
            CellType::PrunedBranch => self.validate_exotic_pruned(data, bit_len, references),
            CellType::Library => self.validate_library(bit_len),
            CellType::MerkleProof => self.validate_merkle_proof(data, bit_len, references),
            CellType::MerkleUpdate => self.validate_merkle_update(data, bit_len, references),
        }
    }

    pub(crate) fn level_mask(
        &self,
        cell_data: &[u8],
        cell_data_bit_len: usize,
        references: &[ArcCell],
    ) -> Result<LevelMask, TonCellError> {
        let result = match self {
            CellType::Ordinary => references
                .iter()
                .fold(LevelMask::new(0), |level_mask, reference| {
                    level_mask.apply_or(reference.level_mask)
                }),
            CellType::PrunedBranch => self.pruned_level_mask(cell_data, cell_data_bit_len)?,
            CellType::Library => LevelMask::new(0),
            CellType::MerkleProof => references[0].level_mask.shift_right(),
            CellType::MerkleUpdate => references[0]
                .level_mask
                .apply_or(references[1].level_mask)
                .shift_right(),
        };

        Ok(result)
    }

    pub(crate) fn child_depth(&self, child: &Cell, level: u8) -> u16 {
        if matches!(self, CellType::MerkleProof | CellType::MerkleUpdate) {
            child.get_depth(level + 1)
        } else {
            child.get_depth(level)
        }
    }

    pub(crate) fn resolve_hashes_and_depths(
        &self,
        hashes: Vec<TonHash>,
        depths: Vec<u16>,
        data: &[u8],
        bit_len: usize,
        level_mask: LevelMask,
    ) -> Result<([TonHash; 4], [u16; 4]), TonCellError> {
        let mut resolved_hashes = [ZERO_HASH; 4];
        let mut resolved_depths = [0; 4];

        for i in 0..4 {
            let hash_index = level_mask.apply(i).hash_index();

            let (hash, depth) = if self == &CellType::PrunedBranch {
                let this_hash_index = level_mask.hash_index();
                if hash_index != this_hash_index {
                    let pruned = self
                        .pruned(data, bit_len, level_mask)
                        .map_cell_builder_error()?;
                    (pruned[hash_index].hash, pruned[hash_index].depth)
                } else {
                    (hashes[0], depths[0])
                }
            } else {
                (hashes[hash_index], depths[hash_index])
            };

            resolved_hashes[i as usize] = hash;
            resolved_depths[i as usize] = depth;
        }

        Ok((resolved_hashes, resolved_depths))
    }

    fn validate_exotic_pruned(
        &self,
        data: &[u8],
        bit_len: usize,
        references: impl AsRef<[ArcCell]>,
    ) -> Result<(), TonCellError> {
        if !references.as_ref().is_empty() {
            return Err(TonCellError::InvalidExoticCellData(format!(
                "Pruned Branch cell can't have refs, got {}",
                references.as_ref().len()
            )));
        }

        if bit_len < 16 {
            return Err(TonCellError::InvalidExoticCellData(
                "Not enough data for a PrunnedBranch special cell".to_owned(),
            ));
        }

        if !self.is_config_proof(bit_len) {
            let level_mask = self.pruned_level_mask(data, bit_len)?;
            let level = level_mask.level();

            if level == 0 || level > MAX_LEVEL {
                return Err(TonCellError::InvalidExoticCellData(format!(
                    "Pruned Branch cell level must be >= 1 and <= 3, got {}/{}",
                    level_mask.level(),
                    level_mask.mask()
                )));
            }

            let expected_size: usize =
                (2 + level_mask.apply(level - 1).hash_count() * (TON_HASH_BYTES + DEPTH_BYTES)) * 8;

            if bit_len != expected_size {
                return Err(TonCellError::InvalidExoticCellData(format!(
                    "Pruned branch cell must have exactly {expected_size} bits, got {bit_len}"
                )));
            }
        }

        Ok(())
    }

    fn validate_library(&self, bit_len: usize) -> Result<(), TonCellError> {
        const SIZE: usize = (1 + TON_HASH_BYTES) * 8;

        if bit_len != SIZE {
            return Err(TonCellError::InvalidExoticCellData(format!(
                "Pruned branch cell must have exactly {SIZE} bits, got {bit_len}"
            )));
        }

        Ok(())
    }

    fn validate_merkle_proof(
        &self,
        data: &[u8],
        bit_len: usize,
        references: impl AsRef<[ArcCell]>,
    ) -> Result<(), TonCellError> {
        let references = references.as_ref();
        // type + hash + depth
        const SIZE: usize = (1 + TON_HASH_BYTES + DEPTH_BYTES) * 8;

        if bit_len != SIZE {
            return Err(TonCellError::InvalidExoticCellData(format!(
                "Merkle Proof cell must have exactly (8 + 256 + 16) bits, got {bit_len}"
            )));
        }

        if references.as_ref().len() != 1 {
            return Err(TonCellError::InvalidExoticCellData(format!(
                "Merkle Proof cell must have exactly 1 ref, got {}",
                references.as_ref().len()
            )));
        }

        let proof_hash: [u8; TON_HASH_BYTES] =
            data[1..(1 + TON_HASH_BYTES)].try_into().map_err(|err| {
                TonCellError::InvalidExoticCellData(format!(
                    "Can't get proof hash bytes from cell data, {}",
                    err
                ))
            })?;
        let proof_depth_bytes = data[(1 + TON_HASH_BYTES)..(1 + TON_HASH_BYTES + 2)]
            .try_into()
            .map_err(|err| {
                TonCellError::InvalidExoticCellData(format!(
                    "Can't get proof depth bytes from cell data, {}",
                    err
                ))
            })?;
        let proof_depth = u16::from_be_bytes(proof_depth_bytes);
        let ref_hash = references[0].get_hash(0);
        let ref_depth = references[0].get_depth(0);

        if proof_depth != ref_depth {
            return Err(TonCellError::InvalidExoticCellData(format!(
                "Merkle Proof cell ref depth must be exactly {proof_depth}, got {ref_depth}"
            )));
        }

        if proof_hash != ref_hash {
            return Err(TonCellError::InvalidExoticCellData(format!(
                "Merkle Proof cell ref hash must be exactly {proof_hash:?}, got {ref_hash:?}"
            )));
        }

        Ok(())
    }

    fn validate_merkle_update(
        &self,
        data: &[u8],
        bit_len: usize,
        references: impl AsRef<[ArcCell]>,
    ) -> Result<(), TonCellError> {
        let references = references.as_ref();
        // type + hash + hash + depth + depth
        const SIZE: usize = 8 + (2 * (256 + 16));

        if bit_len != SIZE {
            return Err(TonCellError::InvalidExoticCellData(format!(
                "Merkle Update cell must have exactly (8 + 256 + 16) bits, got {bit_len}"
            )));
        }

        if references.len() != 2 {
            return Err(TonCellError::InvalidExoticCellData(format!(
                "Merkle Update cell must have exactly 2 refs, got {}",
                references.len()
            )));
        }

        let proof_hash1: TonHash = data[1..33].try_into().map_err(|err| {
            TonCellError::InvalidExoticCellData(format!(
                "Can't get proof hash bytes 1 from cell data, {}",
                err
            ))
        })?;
        let proof_hash2: TonHash = data[33..65].try_into().map_err(|err| {
            TonCellError::InvalidExoticCellData(format!(
                "Can't get proof hash bytes 2 from cell data, {}",
                err
            ))
        })?;
        let proof_depth_bytes1 = data[65..67].try_into().map_err(|err| {
            TonCellError::InvalidExoticCellData(format!(
                "Can't get proof depth bytes 1 from cell data, {}",
                err
            ))
        })?;
        let proof_depth_bytes2 = data[67..69].try_into().map_err(|err| {
            TonCellError::InvalidExoticCellData(format!(
                "Can't get proof depth bytes 2 from cell data, {}",
                err
            ))
        })?;
        let proof_depth1 = u16::from_be_bytes(proof_depth_bytes1);
        let proof_depth2 = u16::from_be_bytes(proof_depth_bytes2);

        let ref_hash1 = references[0].get_hash(0);
        let ref_depth1 = references[0].get_depth(0);
        let ref_hash2 = references[1].get_hash(0);
        let ref_depth2 = references[1].get_depth(0);

        if proof_depth1 != ref_depth1 {
            return Err(TonCellError::InvalidExoticCellData(format!(
                "Merkle Proof cell ref depth 1 must be exactly {proof_depth1}, got {ref_depth1}"
            )));
        }

        if proof_hash1 != ref_hash1 {
            return Err(TonCellError::InvalidExoticCellData(format!(
                "Merkle Proof cell ref hash 1 must be exactly {proof_hash1:?}, got {ref_hash1:?}"
            )));
        }

        if proof_depth2 != ref_depth2 {
            return Err(TonCellError::InvalidExoticCellData(format!(
                "Merkle Proof cell ref depth 2 must be exactly {proof_depth2}, got {ref_depth2}"
            )));
        }

        if proof_hash2 != ref_hash2 {
            return Err(TonCellError::InvalidExoticCellData(format!(
                "Merkle Proof cell ref hash 2 must be exactly {proof_hash2:?}, got {ref_hash2:?}"
            )));
        }

        Ok(())
    }

    fn pruned_level_mask(&self, data: &[u8], bit_len: usize) -> Result<LevelMask, TonCellError> {
        if data.len() < 5 {
            return Err(TonCellError::InvalidExoticCellData(format!(
                "Pruned Branch cell date can't be shorter than 5 bytes, got {}",
                data.len()
            )));
        }

        let level_mask = if self.is_config_proof(bit_len) {
            LevelMask::new(1)
        } else {
            let mask_byte = data[1];
            LevelMask::new(mask_byte as u32)
        };

        Ok(level_mask)
    }

    fn pruned(
        &self,
        data: &[u8],
        bit_len: usize,
        level_mask: LevelMask,
    ) -> Result<Vec<Pruned>, io::Error> {
        let current_index = if self.is_config_proof(bit_len) { 1 } else { 2 };

        let cursor = Cursor::new(&data[current_index..]);
        let mut reader = ByteReader::endian(cursor, BigEndian);

        let level = level_mask.level() as usize;
        let hashes = (0..level)
            .map(|_| reader.read::<TonHash>())
            .collect::<Result<Vec<_>, _>>()?;
        let depths = (0..level)
            .map(|_| reader.read::<u16>())
            .collect::<Result<Vec<_>, _>>()?;

        let result = hashes
            .into_iter()
            .zip(depths)
            .map(|(hash, depth)| Pruned { depth, hash })
            .collect();

        Ok(result)
    }

    /// Special case for config proof
    /// This test proof is generated in the moment of voting for a slashing
    /// it seems that tools generate it incorrectly and therefore doesn't have mask in it
    /// so we need to hardcode it equal to 1 in this case
    fn is_config_proof(&self, bit_len: usize) -> bool {
        self == &CellType::PrunedBranch && bit_len == 280
    }
}
