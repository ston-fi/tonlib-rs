use crate::cell::{CellBuilder, CellParser, TonCellError};
use crate::tlb_types::traits::TLBObject;
use crate::types::TonHash;

/// WalletVersion::V1R1 | WalletVersion::V1R2 | WalletVersion::V1R3 | WalletVersion::V2R1 | WalletVersion::V2R2
pub struct WalletDataV1V2 {
    pub seqno: u32,
    pub public_key: TonHash,
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
