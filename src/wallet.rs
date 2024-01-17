use lazy_static::lazy_static;
use nacl::sign::signature;

use crate::address::TonAddress;
use crate::cell::{BagOfCells, Cell, CellBuilder, StateInit, TonCellError};
use crate::message::{TonMessageError, ZERO_COINS};
use crate::mnemonic::KeyPair;

lazy_static! {
    pub static ref WALLET_V3_CODE: BagOfCells = {
        let code = include_str!("../resources/wallet/wallet_v3_code.hex");
        BagOfCells::parse_hex(code).unwrap()
    };
    pub static ref WALLET_V3R2_CODE: BagOfCells = {
        let code = include_str!("../resources/wallet/wallet_v3r2_code.hex");
        BagOfCells::parse_hex(code).unwrap()
    };
    pub static ref WALLET_V4R2_CODE: BagOfCells = {
        let code = include_str!("../resources/wallet/wallet_v4r2_code.hex");
        BagOfCells::parse_hex(code).unwrap()
    };
}

#[derive(PartialEq, Eq, Clone, Hash)]
pub enum WalletVersion {
    V3,
    V3R2,
    V4R2,
}

impl WalletVersion {
    pub fn code(&self) -> &'static BagOfCells {
        let code: &BagOfCells = match self {
            WalletVersion::V3 => &WALLET_V3_CODE,
            WalletVersion::V3R2 => &WALLET_V3R2_CODE,
            WalletVersion::V4R2 => &WALLET_V4R2_CODE,
        };
        code
    }

    pub fn initial_data(
        &self,
        workchain: i32,
        key_pair: &KeyPair,
    ) -> Result<BagOfCells, TonCellError> {
        let mut data_builder = CellBuilder::new();
        data_builder
            .store_u32(32, 0)?
            // seqno
            .store_u32(32, 698983191 + workchain as u32)?
            //wallet_id
            .store_slice(key_pair.public_key.as_slice())?; // public key
        if *self == WalletVersion::V4R2 {
            data_builder.store_bit(false)?;
            // empty plugin dict
        }
        let data_cell = data_builder.build()?;
        Ok(BagOfCells::from_root(data_cell))
    }

    pub fn wallet_id(&self) -> u32 {
        0x29a9a317 // Same for all wallet versions
    }

    pub fn has_op(&self) -> bool {
        match self {
            WalletVersion::V4R2 => true,
            _ => false,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Hash)]
pub struct TonWallet {
    pub key_pair: KeyPair,
    pub version: WalletVersion,
    pub address: TonAddress,
}

impl TonWallet {
    pub fn derive(
        workchain: i32,
        version: WalletVersion,
        key_pair: &KeyPair,
    ) -> Result<TonWallet, TonCellError> {
        let data = version.initial_data(workchain, key_pair)?;
        let code = version.code();
        let state_init_hash =
            StateInit::create_account_id(code.single_root()?, data.single_root()?)?;
        let hash_part = match state_init_hash.as_slice().try_into() {
            Ok(hash_part) => hash_part,
            Err(_) => {
                return Err(TonCellError::InternalError(
                    "StateInit returned hash pof wrong size".to_string(),
                ))
            }
        };
        let addr = TonAddress::new(workchain, &hash_part);
        Ok(TonWallet {
            key_pair: key_pair.clone(),
            version,
            address: addr,
        })
    }

    pub fn create_external_message<T>(
        &self,
        expire_at: u32,
        seqno: u32,
        internal_message: T,
    ) -> Result<Cell, TonMessageError>
    where
        T: Into<Vec<Cell>>,
    {
        let body = self.create_external_body(expire_at, seqno, internal_message)?;
        let signed = self.sign_external_body(&body)?;
        let wrapped = self.wrap_signed_body(signed)?;
        Ok(wrapped)
    }

    pub fn create_external_body<T>(
        &self,
        expire_at: u32,
        seqno: u32,
        internal_message: T,
    ) -> Result<Cell, TonCellError>
    where
        T: Into<Vec<Cell>>,
    {
        let mut builder = CellBuilder::new();
        builder
            .store_u32(32, self.version.wallet_id())?
            .store_u32(32, expire_at)?
            .store_u32(32, seqno)?;
        if self.version.has_op() {
            builder.store_u8(8, 0)?;
        }
        for internal_message in internal_message.into() {
            builder.store_u8(8, 3)?; // send_mode
            builder.store_child(internal_message)?;
        }
        builder.build()
    }

    pub fn sign_external_body(&self, external_body: &Cell) -> Result<Cell, TonMessageError> {
        let message_hash = external_body.cell_hash()?;
        let sig = signature(message_hash.as_slice(), self.key_pair.secret_key.as_slice())
            .map_err(|e| TonMessageError::NaclCryptographicError(e.message))?;
        let mut body_builder = CellBuilder::new();
        body_builder.store_slice(sig.as_slice())?;
        body_builder.store_cell(&external_body)?;
        Ok(body_builder.build()?)
    }

    pub fn wrap_signed_body(&self, signed_body: Cell) -> Result<Cell, TonCellError> {
        let mut wrap_builder = CellBuilder::new();
        wrap_builder
            .store_u8(2, 2)?
            // No idea
            .store_address(&TonAddress::NULL)?
            // src
            .store_address(&self.address)?
            // dest
            .store_coins(&ZERO_COINS)?
            // import fee
            .store_bit(false)?
            // TODO: add state_init support
            .store_bit(true)?
            // signed_body is always defined
            .store_child(signed_body)?;
        wrap_builder.build()
    }
}

#[cfg(test)]
mod tests {
    use crate::address::TonAddress;
    use crate::mnemonic::Mnemonic;
    use crate::wallet::{TonWallet, WalletVersion};

    #[test]
    fn derive_wallet_works() -> anyhow::Result<()> {
        let mnemonic_str = "fancy carpet hello mandate penalty trial consider \
        property top vicious exit rebuild tragic profit urban major total month holiday \
        sudden rib gather media vicious";
        let mnemonic = Mnemonic::from_str(&mnemonic_str, &None)?;
        let key_pair = mnemonic.to_key_pair()?;
        let wallet_v3 = TonWallet::derive(0, WalletVersion::V3, &key_pair)?;
        let expected_v3: TonAddress = "EQBiMfDMivebQb052Z6yR3jHrmwNhw1kQ5bcAUOBYsK_VPuK".parse()?;
        assert_eq!(wallet_v3.address, expected_v3);
        let wallet_v3r2 = TonWallet::derive(0, WalletVersion::V3R2, &key_pair)?;
        let expected_v3r2: TonAddress =
            "EQA-RswW9QONn88ziVm4UKnwXDEot5km7GEEXsfie_0TFOCO".parse()?;
        assert_eq!(wallet_v3r2.address, expected_v3r2);
        let wallet_v4r2 = TonWallet::derive(0, WalletVersion::V4R2, &key_pair)?;
        let expected_v4r2: TonAddress =
            "EQCDM_QGggZ3qMa_f3lRPk4_qLDnLTqdi6OkMAV2NB9r5TG3".parse()?;
        assert_eq!(wallet_v4r2.address, expected_v4r2);
        Ok(())
    }
}
