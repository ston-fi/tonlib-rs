use nacl::sign::signature;

use crate::cell::{ArcCell, Cell, CellBuilder, TonCellError};
use crate::message::{TonMessageError, ZERO_COINS};
use crate::tlb_types::block::coins::Grams;
use crate::tlb_types::block::message::{CommonMsgInfo, ExtInMsgInfo, Message};
use crate::tlb_types::block::state_init::StateInit;
use crate::tlb_types::traits::TLBObject;
use crate::types::TonAddress;
use crate::wallet::mnemonic::KeyPair;
use crate::wallet::wallet_data::{DEFAULT_WALLET_ID, DEFAULT_WALLET_ID_V5R1};
use crate::wallet::wallet_helper::TonWalletHelper;
use crate::wallet::wallet_version::WalletVersion;

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct TonWallet {
    pub version: WalletVersion,
    pub key_pair: KeyPair,
    pub address: TonAddress,
    pub wallet_id: i32,
}

impl TonWallet {
    pub fn new(
        version: WalletVersion,
        key_pair: &KeyPair,
        workchain: i32,
        wallet_id: i32,
    ) -> Result<TonWallet, TonCellError> {
        let data = TonWalletHelper::get_data(version, key_pair, wallet_id)?.to_arc();
        let code = TonWalletHelper::get_code(version)?.clone();
        let addr = TonAddress::derive(workchain, code, data)?;
        Ok(TonWallet {
            key_pair: key_pair.clone(),
            version,
            address: addr,
            wallet_id,
        })
    }

    pub fn new_default(
        version: WalletVersion,
        key_pair: &KeyPair,
    ) -> Result<TonWallet, TonCellError> {
        let wallet_id = match version {
            WalletVersion::V5R1 => DEFAULT_WALLET_ID_V5R1,
            _ => DEFAULT_WALLET_ID,
        };
        Self::new(version, key_pair, 0, wallet_id)
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

        if TonWalletHelper::has_opcode(self.version) {
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
        let msg_info = CommonMsgInfo::ExtIn(ExtInMsgInfo {
            src: TonAddress::NULL.to_tlb_msg_addr(),
            dest: self.address.to_tlb_msg_addr(),
            import_fee: Grams::new(ZERO_COINS.clone()),
        });

        let mut message = Message::new(msg_info, signed_body.to_arc());
        if state_init {
            let code = TonWalletHelper::get_code(self.version)?.clone();
            let data =
                TonWalletHelper::get_data(WalletVersion::V4R2, &self.key_pair, self.wallet_id)?
                    .to_arc();
            let state_init = StateInit::new(code, data);
            message.with_state_init(state_init);
        }
        Ok(message.to_cell()?)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::types::TonAddress;
    use crate::wallet::error::MnemonicError;

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

        let wallet_v3 = TonWallet::new_default(WalletVersion::V3R1, &key_pair).unwrap();
        let expected_v3: TonAddress = "EQBiMfDMivebQb052Z6yR3jHrmwNhw1kQ5bcAUOBYsK_VPuK"
            .parse()
            .unwrap();
        assert_eq!(wallet_v3.address, expected_v3);
        let wallet_v3r2 = TonWallet::new_default(WalletVersion::V3R2, &key_pair).unwrap();
        let expected_v3r2: TonAddress = "EQA-RswW9QONn88ziVm4UKnwXDEot5km7GEEXsfie_0TFOCO"
            .parse()
            .unwrap();
        assert_eq!(wallet_v3r2.address, expected_v3r2);
        let wallet_v4r2 = TonWallet::new_default(WalletVersion::V4R2, &key_pair).unwrap();
        let expected_v4r2: TonAddress = "EQCDM_QGggZ3qMa_f3lRPk4_qLDnLTqdi6OkMAV2NB9r5TG3"
            .parse()
            .unwrap();
        assert_eq!(wallet_v4r2.address, expected_v4r2);

        let wallet_v5 = TonWallet::new_default(WalletVersion::V5R1, &key_pair_v5).unwrap();
        let expected_v5: TonAddress = "UQDv2YSmlrlLH3hLNOVxC8FcQf4F9eGNs4vb2zKma4txo6i3"
            .parse()
            .unwrap();
        assert_eq!(wallet_v5.address, expected_v5);
        Ok(())
    }

    use crate::wallet::mnemonic::{KeyPair, Mnemonic};
    use crate::wallet::ton_wallet::{TonWallet, WalletVersion};

    #[test]
    fn test_debug_ton_wallet() -> anyhow::Result<()> {
        let key_pair = KeyPair {
            public_key: vec![1, 2, 3],
            secret_key: vec![4, 5, 6],
        };
        let wallet = TonWallet {
            key_pair,
            version: WalletVersion::V4R2,
            address: TonAddress::from_str("EQBiMfDMivebQb052Z6yR3jHrmwNhw1kQ5bcAUOBYsK_VPuK")?,
            wallet_id: 42,
        };

        let debug_output = format!("{:?}", wallet);
        let expected_output = "TonWallet { version: V4R2, key_pair: KeyPair { public_key: [1, 2, 3], secret_key: \"***REDACTED***\" }, address: EQBiMfDMivebQb052Z6yR3jHrmwNhw1kQ5bcAUOBYsK_VPuK, wallet_id: 42 }";
        assert_eq!(debug_output, expected_output);
        Ok(())
    }
}
