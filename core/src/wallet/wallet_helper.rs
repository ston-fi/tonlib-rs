use crate::cell::{ArcCell, Cell, TonCellError};
use crate::tlb_types::traits::TLBObject;
use crate::wallet::mnemonic::KeyPair;
use crate::wallet::wallet_code::WALLET_CODE_BY_VERSION;
use crate::wallet::wallet_data::highload_v2::WalletDataHighloadV2R2;
use crate::wallet::wallet_data::v1_v2::WalletDataV1V2;
use crate::wallet::wallet_data::v3::WalletDataV3;
use crate::wallet::wallet_data::v4::WalletDataV4;
use crate::wallet::wallet_data::v5::WalletDataV5;
use crate::wallet::wallet_version::WalletVersion;
use crate::TonHash;

pub struct TonWalletHelper;

impl TonWalletHelper {
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

    pub fn has_opcode(version: WalletVersion) -> bool {
        matches!(version, WalletVersion::V4R2)
    }
}
