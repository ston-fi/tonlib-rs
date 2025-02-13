use crate::cell::{ArcCell, CellBuilder, CellParser, TonCellError};
use crate::tlb_types::traits::TLBObject;
use crate::types::TonHash;
use crate::wallet::versioned::utils::write_up_to_4_msgs;

/// Is not covered by tests and it generally means unsupported
/// WalletVersion::V1R1 | WalletVersion::V1R2 | WalletVersion::V1R3 | WalletVersion::V2R1 | WalletVersion::V2R2
#[derive(Debug, PartialEq, Clone)]
pub struct WalletDataV1V2 {
    pub seqno: u32,
    pub public_key: TonHash,
}

/// https://docs.ton.org/participate/wallets/contracts#wallet-v2
#[derive(Debug, PartialEq, Clone)]
pub struct WalletExtMsgBodyV2 {
    pub msg_seqno: u32,
    pub valid_until: u32,
    pub msgs_modes: Vec<u8>,
    pub msgs: Vec<ArcCell>,
}

impl WalletDataV1V2 {
    pub fn new(public_key: TonHash) -> Self {
        Self {
            seqno: 0,
            public_key,
        }
    }
}

impl TLBObject for WalletDataV1V2 {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        Ok(Self {
            seqno: parser.load_u32(32)?,
            public_key: parser.load_tonhash()?,
        })
    }

    fn write_to(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        dst.store_u32(32, self.seqno)?;
        dst.store_tonhash(&self.public_key)?;
        Ok(())
    }
}

impl TLBObject for WalletExtMsgBodyV2 {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        let _signature = parser.load_bytes(64)?;
        let msg_seqno = parser.load_u32(32)?;
        let valid_until = parser.load_u32(32)?;
        let msgs_cnt = parser.cell.references().len();
        let mut msgs_modes = Vec::with_capacity(msgs_cnt);
        let mut msgs = Vec::with_capacity(msgs_cnt);
        for _ in 0..msgs_cnt {
            msgs_modes.push(parser.load_u8(8)?);
            msgs.push(parser.next_reference()?);
        }
        Ok(Self {
            msg_seqno,
            valid_until,
            msgs_modes,
            msgs,
        })
    }

    fn write_to(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        dst.store_u32(32, self.msg_seqno)?;
        dst.store_u32(32, self.valid_until)?;
        write_up_to_4_msgs(dst, &self.msgs, &self.msgs_modes)?;
        Ok(())
    }
}
