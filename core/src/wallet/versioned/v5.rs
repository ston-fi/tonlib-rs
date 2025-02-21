use crate::cell::{ArcCell, CellBuilder, CellParser, TonCellError};
use crate::tlb_types::block::out_action::{OutAction, OutActionSendMsg, OutList};
use crate::tlb_types::primitives::reference::Ref;
use crate::tlb_types::traits::{TLBObject, TLBPrefix};
use crate::types::TonHash;
use crate::wallet::versioned::utils::validate_msgs_count;

/// WalletVersion::V5R1
/// https://github.com/ton-blockchain/wallet-contract-v5/blob/main/types.tlb#L29
#[derive(Debug, PartialEq, Clone)]
pub struct WalletDataV5 {
    pub signature_allowed: bool,
    pub seqno: u32,
    pub wallet_id: i32,
    pub public_key: TonHash,
    pub extensions: Option<Ref<ArcCell>>,
}

/// https://docs.ton.org/participate/wallets/contracts#wallet-v5
/// signature is not considered as part of msg body
/// https://github.com/ton-blockchain/wallet-contract-v5/blob/main/types.tlb
/// This implementation support only jetton transfer messages
#[derive(Debug, PartialEq, Clone)]
pub struct WalletExtMsgBodyV5 {
    pub wallet_id: i32,
    pub valid_until: u32,
    pub msg_seqno: u32,
    pub msgs_modes: Vec<u8>,
    pub msgs: Vec<ArcCell>,
}

impl WalletDataV5 {
    pub fn new(wallet_id: i32, public_key: TonHash) -> Self {
        Self {
            signature_allowed: true,
            seqno: 0,
            wallet_id,
            public_key,
            extensions: None,
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
            extensions: TLBObject::read(parser)?,
        })
    }

    fn write_to(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        dst.store_bit(self.signature_allowed)?;
        dst.store_u32(32, self.seqno)?;
        dst.store_i32(32, self.wallet_id)?;
        dst.store_tonhash(&self.public_key)?;
        self.extensions.write_to(dst)?;
        Ok(())
    }
}

impl TLBObject for WalletExtMsgBodyV5 {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        Self::verify_prefix(parser)?;
        let wallet_id = parser.load_i32(32)?;
        let valid_until = parser.load_u32(32)?;
        let msg_seqno = parser.load_u32(32)?;
        let inner_request = InnerRequest::read(parser)?;
        let (msgs, msgs_modes) = parse_inner_request(inner_request)?;
        Ok(Self {
            wallet_id,
            valid_until,
            msg_seqno,
            msgs_modes,
            msgs,
        })
    }

    fn write_to(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        Self::write_prefix(dst)?;
        dst.store_i32(32, self.wallet_id)?;
        dst.store_u32(32, self.valid_until)?;
        dst.store_u32(32, self.msg_seqno)?;
        let inner_req = build_inner_request(&self.msgs, &self.msgs_modes)?;
        inner_req.write_to(dst)?;
        Ok(())
    }

    fn prefix() -> &'static TLBPrefix {
        const PREFIX: TLBPrefix = TLBPrefix::new(32, 0x7369676e);
        &PREFIX
    }
}

// https://github.com/ton-blockchain/wallet-contract-v5/blob/88557ebc33047a95207f6e47ac8aadb102dff744/types.tlb#L26
#[derive(Debug, PartialEq, Clone)]
pub(super) struct InnerRequest {
    out_actions: Option<Ref<OutList>>, // tlb tells there is Option<OutList>, but it lies
                                       // other_actions: Option<()> unsupported
}

impl TLBObject for InnerRequest {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        let out_actions = TLBObject::read(parser)?;
        if parser.load_bit()? {
            return Err(TonCellError::InternalError(
                "other_actions parsing is unsupported".to_string(),
            ));
        }
        Ok(Self { out_actions })
    }

    fn write_to(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        self.out_actions.write_to(dst)?;
        dst.store_bit(false)?; // other_actions are not supported
        Ok(())
    }
}

fn parse_inner_request(request: InnerRequest) -> Result<(Vec<ArcCell>, Vec<u8>), TonCellError> {
    let mut out_list = match request.out_actions {
        Some(out_list) => out_list.0,
        None => return Ok((vec![], vec![])),
    };
    let mut msgs = vec![];
    let mut msgs_modes = vec![];
    while let OutList::Some(action) = out_list {
        if let OutAction::SendMsg(action_send_msg) = &action.action {
            msgs.push(action_send_msg.out_msg.clone());
            msgs_modes.push(action_send_msg.mode);
        } else {
            let err_str = format!("Unsupported OutAction: {action:?}");
            return Err(TonCellError::InvalidCellData(err_str));
        }
        out_list = TLBObject::from_cell(&action.prev.0)?;
    }

    Ok((msgs, msgs_modes))
}

fn build_inner_request(msgs: &[ArcCell], msgs_modes: &[u8]) -> Result<InnerRequest, TonCellError> {
    validate_msgs_count(msgs, msgs_modes, 255)?;
    // TODO suboptimal - can be done in 1 pass, but here we have 1 loop pass + recursion in OutList
    let mut actions = vec![];
    for (msg, mode) in msgs.iter().zip(msgs_modes.iter()) {
        let action = OutActionSendMsg {
            mode: *mode,
            out_msg: msg.clone(),
        };
        actions.push(OutAction::SendMsg(action));
    }

    let out_list = OutList::new(&actions)?;

    let req = InnerRequest {
        out_actions: Some(Ref::new(out_list)),
    };
    Ok(req)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::cell::Cell;
    use crate::tlb_types::traits::TLBObject;
    use crate::wallet::versioned::{DEFAULT_WALLET_ID_V5R1, DEFAULT_WALLET_ID_V5R1_TESTNET};

    #[test]
    fn test_wallet_data_v5() -> anyhow::Result<()> {
        // https://tonviewer.com/UQDwj2jGHWEbPpDf0I2qktDwqtv6tBCfBVNH9gJEnM-QmHDa
        let src_boc_hex = "b5ee9c7241010101002b00005180000000bfffff88e5f9bbe4db9b026385fb9a446ee75d0a7bb1dd77956387b468eb01950900a4fa20cbe13a2a";
        let wallet_data = WalletDataV5::from_boc_hex(src_boc_hex)?;
        assert_eq!(wallet_data.seqno, 1);
        assert_eq!(wallet_data.wallet_id, DEFAULT_WALLET_ID_V5R1);
        assert_eq!(
            wallet_data.public_key,
            TonHash::from_hex("cbf377c9b73604c70bf73488ddceba14f763baef2ac70f68d1d6032a120149f4")?
        );
        assert_eq!(wallet_data.extensions, None);

        let serial_boc_hex = wallet_data.to_boc_hex(true)?;
        assert_eq!(src_boc_hex, serial_boc_hex);
        let restored = WalletDataV5::from_boc_hex(&serial_boc_hex)?;
        assert_eq!(wallet_data, restored);
        Ok(())
    }

    #[test]
    fn test_wallet_data_v5_testnet() -> anyhow::Result<()> {
        let src_boc_hex = "b5ee9c7201010101002b000051800000013ffffffed2b31b23dbe5144a626b9d5d1d4208e36d97e4adb472d42c073bfff85b3107e4a0";
        let wallet_data = WalletDataV5::from_boc_hex(src_boc_hex)?;
        assert_eq!(wallet_data.seqno, 2);
        assert_eq!(wallet_data.wallet_id, DEFAULT_WALLET_ID_V5R1_TESTNET);
        Ok(())
    }

    #[test]
    fn test_wallet_ext_msg_body_v5() -> anyhow::Result<()> {
        // https://tonviewer.com/transaction/b4c5eddc52d0e23dafb2da6d022a5b6ae7eba52876fa75d32b2a95fa30c7e2f0
        let body_hex = "b5ee9c720101040100940001a17369676e7fffff11ffffffff00000000bc04889cb28b36a3a00810e363a413763ec34860bf0fce552c5d36e37289fafd442f1983d740f92378919d969dd530aec92d258a0779fb371d4659f10ca1b3826001020a0ec3c86d030302006642007847b4630eb08d9f486fe846d5496878556dfd5a084f82a9a3fb01224e67c84c187a1200000000000000000000000000000000";
        let body_cell = Cell::from_boc_hex(body_hex)?;
        let mut body_parser = body_cell.parser();
        let body = WalletExtMsgBodyV5::read(&mut body_parser)?;
        let sign = body_parser.load_bytes(64)?;

        assert_eq!(body.wallet_id, DEFAULT_WALLET_ID_V5R1);
        assert_eq!(body.valid_until, 4294967295);
        assert_eq!(body.msg_seqno, 0);
        assert_eq!(body.msgs_modes, vec![3]);
        assert_eq!(body.msgs.len(), 1);

        let serial_cell = body.to_cell()?;
        let signed_serial = CellBuilder::new()
            .store_cell(&serial_cell)?
            .store_slice(&sign)?
            .build()?;

        assert_eq!(body_cell, signed_serial);
        let parsed_back = WalletExtMsgBodyV5::from_cell(&signed_serial)?;
        assert_eq!(body, parsed_back);
        Ok(())
    }
}
