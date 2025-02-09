mod types;

use std::sync::Arc;

use lazy_static::lazy_static;
use nacl::sign::signature;
pub use types::*;

use crate::cell::{ArcCell, BagOfCells, Cell, CellBuilder, TonCellError};
use crate::message::{TonMessageError, ZERO_COINS};
use crate::mnemonic::KeyPair;
use crate::tlb_types::block::state_init::StateInit;
use crate::tlb_types::traits::TLBObject;
use crate::{TonAddress, TonHash};

pub const DEFAULT_WALLET_ID_V5R1: i32 = 0x7FFFFF11;
pub const DEFAULT_WALLET_ID: i32 = 0x29a9a317;

lazy_static! {
    pub static ref WALLET_V1R1_CODE: BagOfCells = {
        let code = include_str!("../resources/wallet/wallet_v1r1.code");
        BagOfCells::parse_base64(code).unwrap()
    };
    pub static ref WALLET_V1R2_CODE: BagOfCells = {
        let code = include_str!("../resources/wallet/wallet_v1r2.code");
        BagOfCells::parse_base64(code).unwrap()
    };
    pub static ref WALLET_V1R3_CODE: BagOfCells = {
        let code = include_str!("../resources/wallet/wallet_v1r3.code");
        BagOfCells::parse_base64(code).unwrap()
    };
    pub static ref WALLET_V2R1_CODE: BagOfCells = {
        let code = include_str!("../resources/wallet/wallet_v2r1.code");
        BagOfCells::parse_base64(code).unwrap()
    };
    pub static ref WALLET_V2R2_CODE: BagOfCells = {
        let code = include_str!("../resources/wallet/wallet_v2r2.code");
        BagOfCells::parse_base64(code).unwrap()
    };
    pub static ref WALLET_V3R1_CODE: BagOfCells = {
        let code = include_str!("../resources/wallet/wallet_v3r1.code");
        BagOfCells::parse_base64(code).unwrap()
    };
    pub static ref WALLET_V3R2_CODE: BagOfCells = {
        let code = include_str!("../resources/wallet/wallet_v3r2.code");
        BagOfCells::parse_base64(code).unwrap()
    };
    pub static ref WALLET_V4R1_CODE: BagOfCells = {
        let code = include_str!("../resources/wallet/wallet_v4r1.code");
        BagOfCells::parse_base64(code).unwrap()
    };
    pub static ref WALLET_V4R2_CODE: BagOfCells = {
        let code = include_str!("../resources/wallet/wallet_v4r2.code");
        BagOfCells::parse_base64(code).unwrap()
    };
    pub static ref WALLET_V5R1_CODE: BagOfCells = {
        let code = include_str!("../resources/wallet/wallet_v5.code");
        BagOfCells::parse_base64(code).unwrap()
    };
    pub static ref HIGHLOAD_V1R1_CODE: BagOfCells = {
        let code = include_str!("../resources/wallet/highload_v1r1.code");
        BagOfCells::parse_base64(code).unwrap()
    };
    pub static ref HIGHLOAD_V1R2_CODE: BagOfCells = {
        let code = include_str!("../resources/wallet/highload_v1r2.code");
        BagOfCells::parse_base64(code).unwrap()
    };
    pub static ref HIGHLOAD_V2_CODE: BagOfCells = {
        let code = include_str!("../resources/wallet/highload_v2.code");
        BagOfCells::parse_base64(code).unwrap()
    };
    pub static ref HIGHLOAD_V2R1_CODE: BagOfCells = {
        let code = include_str!("../resources/wallet/highload_v2r1.code");
        BagOfCells::parse_base64(code).unwrap()
    };
    pub static ref HIGHLOAD_V2R2_CODE: BagOfCells = {
        let code = include_str!("../resources/wallet/highload_v2r2.code");
        BagOfCells::parse_base64(code).unwrap()
    };
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum WalletVersion {
    V1R1,
    V1R2,
    V1R3,
    V2R1,
    V2R2,
    V3R1,
    V3R2,
    V4R1,
    V4R2,
    V5R1,
    HighloadV1R1,
    HighloadV1R2,
    HighloadV2,
    HighloadV2R1,
    HighloadV2R2,
}

impl WalletVersion {
    pub fn code(&self) -> Result<&ArcCell, TonCellError> {
        let code: &BagOfCells = match self {
            WalletVersion::V1R1 => &WALLET_V1R1_CODE,
            WalletVersion::V1R2 => &WALLET_V1R2_CODE,
            WalletVersion::V1R3 => &WALLET_V1R3_CODE,
            WalletVersion::V2R1 => &WALLET_V2R1_CODE,
            WalletVersion::V2R2 => &WALLET_V2R2_CODE,
            WalletVersion::V3R1 => &WALLET_V3R1_CODE,
            WalletVersion::V3R2 => &WALLET_V3R2_CODE,
            WalletVersion::V4R1 => &WALLET_V4R1_CODE,
            WalletVersion::V4R2 => &WALLET_V4R2_CODE,
            WalletVersion::V5R1 => &WALLET_V5R1_CODE,
            WalletVersion::HighloadV1R1 => &HIGHLOAD_V1R1_CODE,
            WalletVersion::HighloadV1R2 => &HIGHLOAD_V1R2_CODE,
            WalletVersion::HighloadV2 => &HIGHLOAD_V2_CODE,
            WalletVersion::HighloadV2R1 => &HIGHLOAD_V2R1_CODE,
            WalletVersion::HighloadV2R2 => &HIGHLOAD_V2R2_CODE,
        };
        code.single_root()
    }

    pub fn initial_data(
        &self,
        key_pair: &KeyPair,
        wallet_id: i32,
    ) -> Result<ArcCell, TonCellError> {
        let public_key = TonHash::try_from(key_pair.public_key.as_slice())?;
        let data_cell: Cell = match &self {
            WalletVersion::V1R1
            | WalletVersion::V1R2
            | WalletVersion::V1R3
            | WalletVersion::V2R1
            | WalletVersion::V2R2 => WalletDataV1V2 {
                seqno: 0,
                public_key,
            }
            .try_into()?,
            WalletVersion::V3R1 | WalletVersion::V3R2 => WalletDataV3 {
                seqno: 0,
                wallet_id,
                public_key,
            }
            .try_into()?,
            WalletVersion::V4R1 | WalletVersion::V4R2 => WalletDataV4 {
                seqno: 0,
                wallet_id,
                public_key,
            }
            .try_into()?,
            WalletVersion::V5R1 => WalletDataV5 {
                signature_allowed: true,
                seqno: 0,
                wallet_id,
                public_key,
            }
            .try_into()?,
            WalletVersion::HighloadV2R2 => WalletDataHighloadV2R2 {
                wallet_id,
                last_cleaned_time: 0,
                public_key,
            }
            .try_into()?,
            WalletVersion::HighloadV1R1
            | WalletVersion::HighloadV1R2
            | WalletVersion::HighloadV2
            | WalletVersion::HighloadV2R1 => {
                return Err(TonCellError::InternalError(
                    "No generation for this wallet version".to_string(),
                ));
            }
        };

        Ok(Arc::new(data_cell))
    }

    pub fn has_op(&self) -> bool {
        matches!(self, WalletVersion::V4R2)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct TonWallet {
    pub key_pair: KeyPair,
    pub version: WalletVersion,
    pub address: TonAddress,
    pub wallet_id: i32,
}

impl TonWallet {
    pub fn derive(
        workchain: i32,
        version: WalletVersion,
        key_pair: &KeyPair,
        wallet_id: i32,
    ) -> Result<TonWallet, TonCellError> {
        let data = version.initial_data(key_pair, wallet_id)?;
        let code = version.code()?;
        let addr = TonAddress::derive(workchain, code.clone(), data)?;
        Ok(TonWallet {
            key_pair: key_pair.clone(),
            version,
            address: addr,
            wallet_id,
        })
    }

    pub fn derive_default(
        version: WalletVersion,
        key_pair: &KeyPair,
    ) -> Result<TonWallet, TonCellError> {
        let wallet_id = match version {
            WalletVersion::V5R1 => DEFAULT_WALLET_ID_V5R1,
            _ => DEFAULT_WALLET_ID,
        };
        let data = version.initial_data(key_pair, wallet_id)?;
        let code = version.code()?;
        let addr = TonAddress::derive(0, code.clone(), data)?;
        Ok(TonWallet {
            key_pair: key_pair.clone(),
            version,
            address: addr,
            wallet_id,
        })
    }

    pub fn create_external_message<T: AsRef<[ArcCell]>>(
        &self,
        expire_at: u32,
        seqno: u32,
        internal_messages: T,
        state_init: bool,
    ) -> Result<Cell, TonMessageError> {
        let body = self.create_external_body(expire_at, seqno, internal_messages)?;
        let signed = self.sign_external_body(&body)?;
        let wrapped = self.wrap_signed_body(signed, state_init)?;
        Ok(wrapped)
    }

    pub fn create_external_body<T: AsRef<[ArcCell]>>(
        &self,
        expire_at: u32,
        seqno: u32,
        internal_messages: T,
    ) -> Result<Cell, TonCellError> {
        let mut builder = CellBuilder::new();
        builder
            .store_i32(32, self.wallet_id)?
            .store_u32(32, expire_at)?
            .store_u32(32, seqno)?;
        if self.version.has_op() {
            builder.store_u8(8, 0)?;
        }
        for internal_message in internal_messages.as_ref() {
            builder.store_u8(8, 3)?; // send_mode
            builder.store_reference(internal_message)?;
        }
        builder.build()
    }

    pub fn sign_external_body(&self, external_body: &Cell) -> Result<Cell, TonMessageError> {
        let message_hash = external_body.cell_hash();
        let sig = signature(message_hash.as_slice(), self.key_pair.secret_key.as_slice())
            .map_err(|e| TonMessageError::NaclCryptographicError(e.message))?;
        let mut body_builder = CellBuilder::new();
        body_builder.store_slice(sig.as_slice())?;
        body_builder.store_cell(external_body)?;
        Ok(body_builder.build()?)
    }

    pub fn wrap_signed_body(
        &self,
        signed_body: Cell,
        state_init: bool,
    ) -> Result<Cell, TonMessageError> {
        let mut wrap_builder = CellBuilder::new();
        wrap_builder
            .store_u8(2, 2)? // mark 2 first bits (1, 0) as external message
            .store_address(TonAddress::null())? // src
            .store_address(&self.address)? // dest
            .store_coins(&ZERO_COINS)?; // import fee
        if state_init {
            wrap_builder.store_bit(true)?; // state init present
            wrap_builder.store_bit(true)?; // state init in ref
            let initial_data = self.version.initial_data(&self.key_pair, self.wallet_id)?;
            let code = WALLET_V4R2_CODE.single_root()?.clone();
            let state_init = StateInit::new(code, initial_data).to_cell()?;
            wrap_builder.store_child(state_init)?;
        } else {
            wrap_builder.store_bit(false)?; // state init absent
        }
        wrap_builder.store_bit(true)?; // signed_body is always defined
        wrap_builder.store_child(signed_body)?;
        Ok(wrap_builder.build()?)
    }
}

#[cfg(test)]
mod tests {
    use crate::mnemonic::{Mnemonic, MnemonicError};
    use crate::wallet::{TonWallet, WalletVersion};
    use crate::TonAddress;

    #[test]
    fn derive_wallet_works() -> Result<(), MnemonicError> {
        let mnemonic_str = "fancy carpet hello mandate penalty trial consider \
        property top vicious exit rebuild tragic profit urban major total month holiday \
        sudden rib gather media vicious";

        let v5_mnemonic_str = "section garden tomato dinner season dice renew length useful spin trade intact use universe what post spike keen mandate behind concert egg doll rug";
        let mnemonic = Mnemonic::from_str(mnemonic_str, &None)?;
        let key_pair = mnemonic.to_key_pair()?;

        let mnemonic_v5 = Mnemonic::from_str(v5_mnemonic_str, &None)?;
        let key_pair_v5 = mnemonic_v5.to_key_pair()?;

        let wallet_v3 = TonWallet::derive_default(WalletVersion::V3R1, &key_pair).unwrap();
        let expected_v3: TonAddress = "EQBiMfDMivebQb052Z6yR3jHrmwNhw1kQ5bcAUOBYsK_VPuK"
            .parse()
            .unwrap();
        assert_eq!(wallet_v3.address, expected_v3);
        let wallet_v3r2 = TonWallet::derive_default(WalletVersion::V3R2, &key_pair).unwrap();
        let expected_v3r2: TonAddress = "EQA-RswW9QONn88ziVm4UKnwXDEot5km7GEEXsfie_0TFOCO"
            .parse()
            .unwrap();
        assert_eq!(wallet_v3r2.address, expected_v3r2);
        let wallet_v4r2 = TonWallet::derive_default(WalletVersion::V4R2, &key_pair).unwrap();
        let expected_v4r2: TonAddress = "EQCDM_QGggZ3qMa_f3lRPk4_qLDnLTqdi6OkMAV2NB9r5TG3"
            .parse()
            .unwrap();
        assert_eq!(wallet_v4r2.address, expected_v4r2);

        let wallet_v5 = TonWallet::derive_default(WalletVersion::V5R1, &key_pair_v5).unwrap();
        let expected_v5: TonAddress = "UQDv2YSmlrlLH3hLNOVxC8FcQf4F9eGNs4vb2zKma4txo6i3"
            .parse()
            .unwrap();
        assert_eq!(wallet_v5.address, expected_v5);
        Ok(())
    }

    use crate::wallet::KeyPair;
    #[test]
    fn test_debug_ton_wallet() {
        let key_pair = KeyPair {
            public_key: vec![1, 2, 3],
            secret_key: vec![4, 5, 6],
        };
        let wallet = TonWallet {
            key_pair,
            version: WalletVersion::V4R2,
            address: "EQBiMfDMivebQb052Z6yR3jHrmwNhw1kQ5bcAUOBYsK_VPuK"
                .parse()
                .unwrap(),
            wallet_id: 42,
        };

        let debug_output = format!("{:?}", wallet);
        let expected_output = "TonWallet { key_pair: KeyPair { public_key: [1, 2, 3], secret_key: \"***REDACTED***\" }, version: V4R2, address: EQBiMfDMivebQb052Z6yR3jHrmwNhw1kQ5bcAUOBYsK_VPuK, wallet_id: 42 }";
        assert_eq!(debug_output, expected_output);
    }
}
