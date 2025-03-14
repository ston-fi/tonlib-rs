use nacl::sign::signature;

use crate::cell::{ArcCell, Cell, TonCellError};
use crate::message::{TonMessageError, ZERO_COINS};
use crate::tlb_types::block::coins::Grams;
use crate::tlb_types::block::message::{CommonMsgInfo, ExtInMsgInfo, Message};
use crate::tlb_types::block::state_init::StateInit;
use crate::tlb_types::traits::TLBObject;
use crate::types::TonAddress;
use crate::wallet::mnemonic::KeyPair;
use crate::wallet::version_helper::VersionHelper;
use crate::wallet::versioned::{DEFAULT_WALLET_ID, DEFAULT_WALLET_ID_V5R1};
use crate::wallet::wallet_version::WalletVersion;

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct TonWallet {
    pub version: WalletVersion,
    pub key_pair: KeyPair,
    pub address: TonAddress,
    pub wallet_id: i32,
}

impl TonWallet {
    pub fn new(version: WalletVersion, key_pair: KeyPair) -> Result<TonWallet, TonCellError> {
        let wallet_id = match version {
            WalletVersion::V5R1 => DEFAULT_WALLET_ID_V5R1,
            _ => DEFAULT_WALLET_ID,
        };
        Self::new_with_params(version, key_pair, 0, wallet_id)
    }

    pub fn new_with_params(
        version: WalletVersion,
        key_pair: KeyPair,
        workchain: i32,
        wallet_id: i32,
    ) -> Result<TonWallet, TonCellError> {
        let data = VersionHelper::get_data(version, &key_pair, wallet_id)?.to_arc();
        let code = VersionHelper::get_code(version)?.clone();
        let address = TonAddress::derive(workchain, code, data)?;

        Ok(TonWallet {
            key_pair,
            version,
            address,
            wallet_id,
        })
    }

    pub fn create_external_msg<T: AsRef<[ArcCell]>>(
        &self,
        expire_at: u32,
        seqno: u32,
        add_state_init: bool,
        internal_messages: T,
    ) -> Result<Cell, TonMessageError> {
        let body = self.create_external_body(expire_at, seqno, internal_messages)?;
        let signed = self.sign_external_body(&body)?;
        let external = self.wrap_signed_body(signed, add_state_init)?;
        Ok(external)
    }

    pub fn create_external_body<T: AsRef<[ArcCell]>>(
        &self,
        expire_at: u32,
        seqno: u32,
        internal_msgs: T,
    ) -> Result<Cell, TonCellError> {
        VersionHelper::build_ext_msg(
            self.version,
            expire_at,
            seqno,
            self.wallet_id,
            internal_msgs,
        )
    }

    pub fn sign_external_body(&self, external_body: &Cell) -> Result<Cell, TonCellError> {
        let message_hash = external_body.cell_hash();
        let sign = signature(message_hash.as_slice(), self.key_pair.secret_key.as_slice())
            .map_err(|err| TonCellError::InternalError(err.message))?;
        VersionHelper::sign_msg(self.version, external_body, &sign)
    }

    pub fn wrap_signed_body(
        &self,
        signed_body: Cell,
        add_state_init: bool,
    ) -> Result<Cell, TonMessageError> {
        let msg_info = CommonMsgInfo::ExtIn(ExtInMsgInfo {
            src: TonAddress::NULL.to_msg_address(),
            dest: self.address.to_msg_address(),
            import_fee: Grams::new(ZERO_COINS.clone()),
        });

        let mut message = Message::new(msg_info, signed_body.to_arc());
        if add_state_init {
            let code = VersionHelper::get_code(self.version)?.clone();
            let data = VersionHelper::get_data(self.version, &self.key_pair, self.wallet_id)?;
            let state_init = StateInit::new(code, data.to_arc());
            message.with_state_init(state_init);
        }
        Ok(message.to_cell()?)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::cell::{Cell, CellBuilder};
    use crate::tlb_types::traits::TLBObject;
    use crate::types::TonAddress;

    const MNEMONIC_STR: &str = "fancy carpet hello mandate penalty trial consider property top vicious exit rebuild tragic profit urban major total month holiday sudden rib gather media vicious";
    const MNEMONIC_STR_V5: &str = "section garden tomato dinner season dice renew length useful spin trade intact use universe what post spike keen mandate behind concert egg doll rug";

    fn make_keypair(mnemonic_str: &str) -> KeyPair {
        let mnemonic = Mnemonic::from_str(mnemonic_str, &None).unwrap();
        mnemonic.to_key_pair().unwrap()
    }

    #[test]
    fn test_ton_wallet_create() -> anyhow::Result<()> {
        let key_pair = make_keypair(MNEMONIC_STR);

        let wallet_v3 = TonWallet::new(WalletVersion::V3R1, key_pair.clone())?;
        let expected_v3 = TonAddress::from_str("EQBiMfDMivebQb052Z6yR3jHrmwNhw1kQ5bcAUOBYsK_VPuK")?;
        assert_eq!(wallet_v3.address, expected_v3);

        let wallet_v3r2 = TonWallet::new(WalletVersion::V3R2, key_pair.clone())?;
        let expected_v3r2 =
            TonAddress::from_str("EQA-RswW9QONn88ziVm4UKnwXDEot5km7GEEXsfie_0TFOCO")?;
        assert_eq!(wallet_v3r2.address, expected_v3r2);

        let wallet_v4r2 = TonWallet::new(WalletVersion::V4R2, key_pair.clone())?;
        let expected_v4r2 =
            TonAddress::from_str("EQCDM_QGggZ3qMa_f3lRPk4_qLDnLTqdi6OkMAV2NB9r5TG3")?;
        assert_eq!(wallet_v4r2.address, expected_v4r2);

        let key_pair_v5 = make_keypair(MNEMONIC_STR_V5);
        let wallet_v5 = TonWallet::new(WalletVersion::V5R1, key_pair_v5.clone())?;
        let expected_v5 = TonAddress::from_str("UQDv2YSmlrlLH3hLNOVxC8FcQf4F9eGNs4vb2zKma4txo6i3")?;
        assert_eq!(wallet_v5.address, expected_v5);
        Ok(())
    }

    use crate::wallet::mnemonic::{KeyPair, Mnemonic};
    use crate::wallet::ton_wallet::{TonWallet, WalletVersion};
    use crate::wallet::versioned::v3::WalletExtMsgBodyV3;
    use crate::wallet::versioned::v4::WalletExtMsgBodyV4;
    use crate::wallet::versioned::v5::WalletExtMsgBodyV5;
    use crate::wallet::versioned::{DEFAULT_WALLET_ID, DEFAULT_WALLET_ID_V5R1};

    #[test]
    fn test_ton_wallet_debug() -> anyhow::Result<()> {
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

    #[test]
    fn test_ton_wallet_create_external_msg_v3() -> anyhow::Result<()> {
        let key_pair = make_keypair(MNEMONIC_STR);
        let wallet = TonWallet::new(WalletVersion::V3R1, key_pair)?;

        let int_msg = CellBuilder::new().build()?.to_arc();

        let ext_body_cell = wallet.create_external_body(13, 7, &[int_msg.clone()])?;
        let body = WalletExtMsgBodyV3::from_cell(&ext_body_cell)?;
        let expected = WalletExtMsgBodyV3 {
            subwallet_id: DEFAULT_WALLET_ID,
            msg_seqno: 7,
            valid_until: 13,
            msgs_modes: vec![3],
            msgs: vec![int_msg],
        };
        assert_eq!(body, expected);
        Ok(())
    }

    #[test]
    fn test_ton_wallet_create_external_msg_v4() -> anyhow::Result<()> {
        let key_pair = make_keypair(MNEMONIC_STR);
        let wallet = TonWallet::new(WalletVersion::V4R1, key_pair)?;

        let int_msg = CellBuilder::new().build()?.to_arc();

        let ext_body_cell = wallet.create_external_body(13, 7, &[int_msg.clone()])?;
        let body = WalletExtMsgBodyV4::from_cell(&ext_body_cell)?;
        let expected = WalletExtMsgBodyV4 {
            subwallet_id: DEFAULT_WALLET_ID,
            msg_seqno: 7,
            opcode: 0,
            valid_until: 13,
            msgs_modes: vec![3],
            msgs: vec![int_msg],
        };
        assert_eq!(body, expected);
        Ok(())
    }

    #[test]
    fn test_ton_wallet_create_external_msg_v5() -> anyhow::Result<()> {
        let key_pair = make_keypair(MNEMONIC_STR_V5);
        let wallet = TonWallet::new(WalletVersion::V5R1, key_pair)?;

        let msgs_cnt = 10usize;
        let mut int_msgs = vec![];
        for i in 0..msgs_cnt {
            let int_msg = CellBuilder::new()
                .store_u32(32, i as u32)?
                .build()?
                .to_arc();
            int_msgs.push(int_msg);
        }
        CellBuilder::new().build()?.to_arc();

        let ext_body_cell = wallet.create_external_body(13, 7, &int_msgs)?;
        let body = WalletExtMsgBodyV5::from_cell(&ext_body_cell)?;
        let expected = WalletExtMsgBodyV5 {
            wallet_id: DEFAULT_WALLET_ID_V5R1,
            msg_seqno: 7,
            valid_until: 13,
            msgs_modes: vec![3; msgs_cnt],
            msgs: int_msgs,
        };
        assert_eq!(body, expected);
        Ok(())
    }

    #[test]
    fn test_ton_wallet_create_external_msg_signed() -> anyhow::Result<()> {
        let key_pair_v3 = make_keypair(MNEMONIC_STR);
        let wallet_v3 = TonWallet::new(WalletVersion::V3R1, key_pair_v3)?;

        let key_pair_v5 = make_keypair(MNEMONIC_STR_V5);
        let wallet_v5 = TonWallet::new(WalletVersion::V5R1, key_pair_v5)?;

        let msg = CellBuilder::new().store_u32(32, 100)?.build()?.to_arc();

        for wallet in [wallet_v3, wallet_v5] {
            let body = wallet.create_external_body(1, 3, &[msg.clone()])?;
            let signed_msg = wallet.sign_external_body(&body)?;

            let mut parser = signed_msg.parser();
            match wallet.version {
                WalletVersion::V5R1 => {
                    // sign in last 512 bits
                    let data_size_bits = signed_msg.bit_len() - 512;
                    let mut builder = CellBuilder::new();
                    builder.store_bits(data_size_bits, &parser.load_bits(data_size_bits)?)?;
                    for ref_cell in parser.cell.references() {
                        builder.store_reference(ref_cell)?;
                    }

                    assert_eq!(body, builder.build()?)
                }
                _ => {
                    // sign in first 512 bits
                    parser.load_bits(512)?;
                    assert_eq!(body, Cell::read(&mut parser)?);
                }
            }
        }
        Ok(())
    }
}
