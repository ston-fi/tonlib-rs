use crate::cell::{ArcCell, CellBuilder, CellParser, TonCellError};
use crate::tlb_types::primitives::reference::Ref;
use crate::tlb_types::tlb::TLB;
use crate::types::TonHash;

/// WalletVersion::HighloadV2R2
#[derive(Clone, Debug)]
pub struct WalletDataHighloadV2R2 {
    pub wallet_id: i32,
    pub last_cleaned_time: u64,
    pub public_key: TonHash,
    pub queries: Option<Ref<ArcCell>>,
}

impl WalletDataHighloadV2R2 {
    pub fn new(wallet_id: i32, public_key: TonHash) -> Self {
        Self {
            wallet_id,
            last_cleaned_time: 0,
            public_key,
            queries: None,
        }
    }
}

impl TLB for WalletDataHighloadV2R2 {
    fn read_definition(parser: &mut CellParser) -> Result<Self, TonCellError> {
        Ok(Self {
            wallet_id: parser.load_i32(32)?,
            last_cleaned_time: parser.load_u64(64)?,
            public_key: parser.load_tonhash()?,
            queries: TLB::read(parser)?,
        })
    }

    fn write_definition(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        dst.store_i32(32, self.wallet_id)?;
        dst.store_u64(64, self.last_cleaned_time)?;
        dst.store_tonhash(&self.public_key)?;
        self.queries.write(dst)?;
        Ok(())
    }
}
