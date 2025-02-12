use crate::cell::{CellBuilder, CellParser, TonCellError};
use crate::tlb_types::traits::TLBObject;
use crate::types::TonHash;

/// WalletVersion::V3R1 | WalletVersion::V3R2
pub struct WalletDataV3 {
    pub seqno: u32,
    pub wallet_id: i32,
    pub public_key: TonHash,
}

impl WalletDataV3 {
    pub fn new(wallet_id: i32, public_key: TonHash) -> Self {
        Self {
            seqno: 0,
            wallet_id,
            public_key,
        }
    }
}
impl TLBObject for WalletDataV3 {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        Ok(Self {
            seqno: parser.load_u32(32)?,
            wallet_id: parser.load_i32(32)?,
            public_key: parser.load_tonhash()?,
        })
    }

    fn write_to(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        dst.store_u32(32, self.seqno)?;
        dst.store_i32(32, self.wallet_id)?;
        dst.store_tonhash(&self.public_key)?;
        Ok(())
    }
}
