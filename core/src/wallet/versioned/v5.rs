use crate::cell::{ArcCell, CellBuilder, CellParser, TonCellError};
use crate::tlb_types::primitives::option::OptionRef;
use crate::tlb_types::traits::TLBObject;
use crate::types::TonHash;

/// WalletVersion::V5R1
#[derive(Debug, PartialEq, Clone)]
pub struct WalletDataV5 {
    pub signature_allowed: bool,
    pub seqno: u32,
    pub wallet_id: i32,
    pub public_key: TonHash,
    pub plugins: OptionRef<ArcCell>,
}

/// https://docs.ton.org/participate/wallets/contracts#wallet-v5
/// signature is not considered as part of msg body
/// https://github.com/ton-blockchain/wallet-contract-v5/blob/main/types.tlb
#[derive(Debug, PartialEq, Clone)]
pub struct WalletExtMsgBodyV5 {
    pub opcode: u32,
    pub wallet_id: i32,
    pub valid_until: u32,
    pub msg_seqno: u32,
    // pub msgs_modes: Vec<u8>,
    // pub msgs: Vec<ArcCell>,
}

impl WalletDataV5 {
    pub fn new(wallet_id: i32, public_key: TonHash) -> Self {
        Self {
            signature_allowed: true,
            seqno: 0,
            wallet_id,
            public_key,
            plugins: OptionRef::NONE,
        }
    }
}

impl TLBObject for WalletDataV5 {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        Ok(Self {
            signature_allowed: parser.load_bit()?,
            seqno: parser.load_u32(32)?,
            wallet_id: parser.load_i32(32)?,
            public_key: parser.load_tonhash()?,
            plugins: TLBObject::read(parser)?,
        })
    }

    fn write_to(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        dst.store_bit(self.signature_allowed)?;
        dst.store_u32(32, self.seqno)?;
        dst.store_i32(32, self.wallet_id)?;
        dst.store_tonhash(&self.public_key)?;
        self.plugins.write_to(dst)?;
        Ok(())
    }
}

impl TLBObject for WalletExtMsgBodyV5 {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        let opcode = parser.load_u32(32)?;
        let subwallet_id = parser.load_i32(32)?;
        let valid_until = parser.load_u32(32)?;
        let msg_seqno = parser.load_u32(32)?;
        // let msgs_cnt = parser.references.len();
        // let mut msgs_modes = Vec::with_capacity(msgs_cnt);
        // let mut msgs = Vec::with_capacity(msgs_cnt);
        // for _ in 0..msgs_cnt {
        //     msgs_modes.push(parser.load_u8(8)?);
        //     msgs.push(parser.next_reference()?);
        // }
        Ok(Self {
            opcode,
            wallet_id: subwallet_id,
            valid_until,
            msg_seqno,
            // msgs_modes,
            // msgs,
        })
    }

    fn write_to(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        dst.store_u32(32, self.opcode)?;
        dst.store_i32(32, self.wallet_id)?;
        dst.store_u32(32, self.valid_until)?;
        dst.store_u32(32, self.msg_seqno)?;
        // for (mode, msg) in self.msgs_modes.iter().zip(self.msgs.iter()) {
        //     dst.store_u8(8, *mode)?;
        //     dst.store_reference(msg)?;
        // }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::tlb_types::traits::TLBObject;
    use crate::wallet::versioned::DEFAULT_WALLET_ID_V5R1;

    #[test]
    fn test_wallet_data_v4() -> anyhow::Result<()> {
        // UQDwj2jGHWEbPpDf0I2qktDwqtv6tBCfBVNH9gJEnM-QmHDa
        let src_boc_hex = "b5ee9c7241010101002b00005180000000bfffff88e5f9bbe4db9b026385fb9a446ee75d0a7bb1dd77956387b468eb01950900a4fa20cbe13a2a";
        let wallet_data = WalletDataV5::from_boc_hex(src_boc_hex)?;
        assert_eq!(wallet_data.seqno, 1);
        assert_eq!(wallet_data.wallet_id, DEFAULT_WALLET_ID_V5R1);
        assert_eq!(
            wallet_data.public_key,
            TonHash::from_hex("cbf377c9b73604c70bf73488ddceba14f763baef2ac70f68d1d6032a120149f4")?
        );
        assert_eq!(wallet_data.plugins, OptionRef::NONE);

        let serial_boc_hex = wallet_data.to_boc_hex()?;
        let restored = WalletDataV5::from_boc_hex(&serial_boc_hex)?;
        assert_eq!(wallet_data, restored);
        Ok(())
    }

    #[test]
    fn test_wallet_ext_msg_body_v5() -> anyhow::Result<()> {
        // https://tonviewer.com/transaction/b4c5eddc52d0e23dafb2da6d022a5b6ae7eba52876fa75d32b2a95fa30c7e2f0
        let body = WalletExtMsgBodyV5::from_boc_hex("b5ee9c720101040100940001a17369676e7fffff11ffffffff00000000bc04889cb28b36a3a00810e363a413763ec34860bf0fce552c5d36e37289fafd442f1983d740f92378919d969dd530aec92d258a0779fb371d4659f10ca1b3826001020a0ec3c86d030302006642007847b4630eb08d9f486fe846d5496878556dfd5a084f82a9a3fb01224e67c84c187a1200000000000000000000000000000000")?;
        assert_eq!(body.opcode, 0x7369676e);
        assert_eq!(body.wallet_id, DEFAULT_WALLET_ID_V5R1);
        assert_eq!(body.valid_until, 4294967295);
        assert_eq!(body.msg_seqno, 0);
        // assert_eq!(body.msgs_modes, vec![3]);
        // assert_eq!(body.msgs.len(), 1);

        let serial_cell = body.to_cell()?;
        let parsed_back = WalletExtMsgBodyV5::from_cell(&serial_cell)?;
        assert_eq!(body, parsed_back);
        Ok(())
    }
}
