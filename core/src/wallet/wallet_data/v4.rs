use crate::cell::{ArcCell, CellBuilder, CellParser, TonCellError};
use crate::tlb_types::primitives::option::OptionRef;
use crate::tlb_types::traits::TLBObject;
use crate::types::TonHash;

/// WalletVersion::V4R1 | WalletVersion::V4R2
pub struct WalletDataV4 {
    pub seqno: u32,
    pub wallet_id: i32,
    pub public_key: TonHash,
    pub plugins: OptionRef<ArcCell>,
}

impl WalletDataV4 {
    pub fn new(wallet_id: i32, public_key: TonHash) -> Self {
        Self {
            seqno: 0,
            wallet_id,
            public_key,
            plugins: OptionRef::NONE,
        }
    }
}

impl TLBObject for WalletDataV4 {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        Ok(Self {
            seqno: parser.load_u32(32)?,
            wallet_id: parser.load_i32(32)?,
            public_key: parser.load_tonhash()?,
            plugins: TLBObject::read(parser)?,
        })
    }

    fn write_to(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        dst.store_u32(32, self.seqno)?;
        dst.store_i32(32, self.wallet_id)?;
        dst.store_tonhash(&self.public_key)?;
        self.plugins.write_to(dst)?;
        Ok(())
    }
}
