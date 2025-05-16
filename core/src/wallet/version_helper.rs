use super::wallet_code::WALLET_VERSION_BY_CODE;
use crate::cell::{ArcCell, Cell, CellBuilder, TonCellError};
use crate::tlb_types::tlb::TLB;
use crate::wallet::mnemonic::KeyPair;
use crate::wallet::versioned::highload_v2::WalletDataHighloadV2R2;
use crate::wallet::versioned::v1_v2::{WalletDataV1V2, WalletExtMsgBodyV2};
use crate::wallet::versioned::v3::{WalletDataV3, WalletExtMsgBodyV3};
use crate::wallet::versioned::v4::{WalletDataV4, WalletExtMsgBodyV4};
use crate::wallet::versioned::v5::{WalletDataV5, WalletExtMsgBodyV5};
use crate::wallet::wallet_code::WALLET_CODE_BY_VERSION;
use crate::wallet::wallet_version::WalletVersion;
use crate::TonHash;

pub struct VersionHelper;

impl VersionHelper {
    pub fn get_data(
        version: WalletVersion,
        key_pair: &KeyPair,
        wallet_id: i32,
    ) -> Result<Cell, TonCellError> {
        let public_key = TonHash::try_from(key_pair.public_key.as_slice())?;
        let data_cell = match version {
            WalletVersion::V1R1
            | WalletVersion::V1R2
            | WalletVersion::V1R3
            | WalletVersion::V2R1
            | WalletVersion::V2R2 => WalletDataV1V2::new(public_key).to_cell()?,
            WalletVersion::V3R1 | WalletVersion::V3R2 => {
                WalletDataV3::new(wallet_id, public_key).to_cell()?
            }
            WalletVersion::V4R1 | WalletVersion::V4R2 => {
                WalletDataV4::new(wallet_id, public_key).to_cell()?
            }
            WalletVersion::V5R1 => WalletDataV5::new(wallet_id, public_key).to_cell()?,
            WalletVersion::HighloadV2R2 => {
                WalletDataHighloadV2R2::new(wallet_id, public_key).to_cell()?
            }
            WalletVersion::HighloadV1R1
            | WalletVersion::HighloadV1R2
            | WalletVersion::HighloadV2
            | WalletVersion::HighloadV2R1 => {
                let err_str = format!("initial_data for {version:?} is unsupported");
                return Err(TonCellError::InternalError(err_str));
            }
        };
        Ok(data_cell)
    }

    pub fn get_code(version: WalletVersion) -> Result<&'static ArcCell, TonCellError> {
        WALLET_CODE_BY_VERSION.get(&version).ok_or_else(|| {
            let err_str = format!("No code found for {version:?}");
            TonCellError::InternalError(err_str)
        })
    }

    pub fn get_version(code_hash: &TonHash) -> Result<&WalletVersion, TonCellError> {
        WALLET_VERSION_BY_CODE
            .get(code_hash)
            .ok_or_else(|| TonCellError::InternalError("No wallet version found".to_string()))
    }

    pub fn build_ext_msg<T: AsRef<[ArcCell]>>(
        version: WalletVersion,
        valid_until: u32,
        msg_seqno: u32,
        wallet_id: i32,
        msgs_refs: T,
    ) -> Result<Cell, TonCellError> {
        let msgs: Vec<ArcCell> = msgs_refs.as_ref().to_vec();

        match version {
            WalletVersion::V2R1 | WalletVersion::V2R2 => WalletExtMsgBodyV2 {
                msg_seqno,
                valid_until,
                msgs_modes: vec![3u8; msgs.len()],
                msgs,
            }
            .to_cell(),
            WalletVersion::V3R1 | WalletVersion::V3R2 => WalletExtMsgBodyV3 {
                subwallet_id: wallet_id,
                msg_seqno,
                valid_until,
                msgs_modes: vec![3u8; msgs.len()],
                msgs,
            }
            .to_cell(),
            WalletVersion::V4R1 | WalletVersion::V4R2 => WalletExtMsgBodyV4 {
                subwallet_id: wallet_id,
                valid_until,
                msg_seqno,
                opcode: 0,
                msgs_modes: vec![3u8; msgs.len()],
                msgs,
            }
            .to_cell(),
            WalletVersion::V5R1 => WalletExtMsgBodyV5 {
                wallet_id,
                valid_until,
                msg_seqno,
                msgs_modes: vec![3u8; msgs.len()],
                msgs,
            }
            .to_cell(),
            _ => {
                let err_str = format!("build_ext_msg for {version:?} is unsupported");
                Err(TonCellError::InternalError(err_str))
            }
        }
    }

    pub fn sign_msg(
        version: WalletVersion,
        msg_cell: &Cell,
        sign: &[u8],
    ) -> Result<Cell, TonCellError> {
        let signed_cell = match version {
            // different order
            WalletVersion::V5R1 => {
                let mut builder = CellBuilder::new();
                builder.store_cell(msg_cell)?;
                builder.store_slice(sign)?;
                builder.build()?
            }
            _ => {
                let mut builder = CellBuilder::new();
                builder.store_slice(sign)?;
                builder.store_cell(msg_cell)?;
                builder.build()?
            }
        };
        Ok(signed_cell)
    }
}
