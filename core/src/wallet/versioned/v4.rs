use crate::cell::{ArcCell, CellBuilder, CellParser, TonCellError};
use crate::tlb_types::primitives::reference::Ref;
use crate::tlb_types::tlb::TLB;
use crate::types::TonHash;
use crate::wallet::versioned::utils::write_up_to_4_msgs;

/// WalletVersion::V4R1 | WalletVersion::V4R2
#[derive(Debug, PartialEq, Clone)]
pub struct WalletDataV4 {
    pub seqno: u32,
    pub wallet_id: i32,
    pub public_key: TonHash,
    pub plugins: Option<Ref<ArcCell>>,
}

/// https://docs.ton.org/participate/wallets/contracts#wallet-v4
/// signature is not considered as part of msg body
#[derive(Debug, PartialEq, Clone)]
pub struct WalletExtMsgBodyV4 {
    pub subwallet_id: i32,
    pub valid_until: u32,
    pub msg_seqno: u32,
    pub opcode: u32,
    pub msgs_modes: Vec<u8>,
    pub msgs: Vec<ArcCell>,
}

impl WalletDataV4 {
    pub fn new(wallet_id: i32, public_key: TonHash) -> Self {
        Self {
            seqno: 0,
            wallet_id,
            public_key,
            plugins: None,
        }
    }
}

impl TLB for WalletDataV4 {
    fn read_definition(parser: &mut CellParser) -> Result<Self, TonCellError> {
        Ok(Self {
            seqno: parser.load_u32(32)?,
            wallet_id: parser.load_i32(32)?,
            public_key: parser.load_tonhash()?,
            plugins: TLB::read(parser)?,
        })
    }

    fn write_definition(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        dst.store_u32(32, self.seqno)?;
        dst.store_i32(32, self.wallet_id)?;
        dst.store_tonhash(&self.public_key)?;
        self.plugins.write(dst)?;
        Ok(())
    }
}

impl TLB for WalletExtMsgBodyV4 {
    fn read_definition(parser: &mut CellParser) -> Result<Self, TonCellError> {
        let subwallet_id = parser.load_i32(32)?;
        let valid_until = parser.load_u32(32)?;
        let msg_seqno = parser.load_u32(32)?;
        let opcode = parser.load_u32(8)?;
        if opcode != 0 {
            let err_str = format!("Unsupported opcode: {opcode}");
            return Err(TonCellError::InternalError(err_str));
        }

        let msgs_cnt = parser.cell.references().len();
        let mut msgs_modes = Vec::with_capacity(msgs_cnt);
        let mut msgs = Vec::with_capacity(msgs_cnt);
        for _ in 0..msgs_cnt {
            msgs_modes.push(parser.load_u8(8)?);
            msgs.push(parser.next_reference()?);
        }
        Ok(Self {
            subwallet_id,
            valid_until,
            msg_seqno,
            opcode,
            msgs_modes,
            msgs,
        })
    }

    fn write_definition(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        if self.opcode != 0 {
            let err_str = format!("Unsupported opcode: {}", self.opcode);
            return Err(TonCellError::InternalError(err_str));
        }
        dst.store_i32(32, self.subwallet_id)?;
        dst.store_u32(32, self.valid_until)?;
        dst.store_u32(32, self.msg_seqno)?;
        dst.store_u32(8, self.opcode)?;
        write_up_to_4_msgs(dst, &self.msgs, &self.msgs_modes)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::cell::Cell;
    use crate::tlb_types::tlb::TLB;
    use crate::wallet::versioned::DEFAULT_WALLET_ID;

    #[test]
    fn test_wallet_data_v4() -> anyhow::Result<()> {
        // https://tonviewer.com/UQCS65EGyiApUTLOYXDs4jOLoQNCE0o8oNnkmfIcm0iX5FRT
        let src_boc_hex = "b5ee9c7241010101002b0000510000001429a9a317cbf377c9b73604c70bf73488ddceba14f763baef2ac70f68d1d6032a120149f440a6c9f37d";
        let wallet_data = WalletDataV4::from_boc_hex(src_boc_hex)?;
        assert_eq!(wallet_data.seqno, 20);
        assert_eq!(wallet_data.wallet_id, DEFAULT_WALLET_ID);
        assert_eq!(
            wallet_data.public_key,
            TonHash::from_hex("cbf377c9b73604c70bf73488ddceba14f763baef2ac70f68d1d6032a120149f4")?
        );
        assert_eq!(wallet_data.plugins, None);

        let serial_boc_hex = wallet_data.to_boc_hex(false)?;
        let restored = WalletDataV4::from_boc_hex(&serial_boc_hex)?;
        assert_eq!(wallet_data, restored);
        Ok(())
    }

    #[test]
    fn test_wallet_ext_msg_body_v4() -> anyhow::Result<()> {
        // https://tonviewer.com/transaction/891dbceffb986251768d4c33bb8dcf11d522408ff78b8e683d135304ca377b8b
        let body_signed_cell = Cell::from_boc_hex("b5ee9c7201010201008700019c9dcd3a68926ad6fb9d094c5b72901bfc359ada50f22b648c6c2223c767135d397c7489c121071e45a5316a94a533d80c41450049ebeed406c419fea99117f40629a9a31767ad328900000013000301006842007847b4630eb08d9f486fe846d5496878556dfd5a084f82a9a3fb01224e67c84c200989680000000000000000000000000000")?;
        let mut parser = body_signed_cell.parser();
        parser.load_bytes(64)?; // signature
        let body_cell = Cell::read(&mut parser)?;

        let body = WalletExtMsgBodyV4::from_cell(&body_cell)?;
        assert_eq!(body.subwallet_id, DEFAULT_WALLET_ID);
        assert_eq!(body.valid_until, 1739403913);
        assert_eq!(body.msg_seqno, 19);
        assert_eq!(body.opcode, 0);
        assert_eq!(body.msgs_modes, vec![3]);
        assert_eq!(body.msgs.len(), 1);

        let serial_cell = body.to_cell()?;
        assert_eq!(body_cell, serial_cell);
        Ok(())
    }
}
