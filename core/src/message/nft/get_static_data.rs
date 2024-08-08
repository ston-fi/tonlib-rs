use super::NFT_GET_STATIC_DATA;
use crate::cell::{Cell, CellBuilder};
use crate::message::{HasOpcode, TonMessage, TonMessageError};

/// Creates a body for nft get_static_data according to TL-B schema:
///
/// ```raw
/// get_static_data#2fcb26a2
///   query_id:uint64
/// = InternalMsgBody;
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct NftGetStaticDataMessage {
    /// arbitrary request number.
    pub query_id: u64,
}

#[allow(clippy::new_without_default)]
impl NftGetStaticDataMessage {
    pub fn new() -> Self {
        NftGetStaticDataMessage { query_id: 0 }
    }
}

impl TonMessage for NftGetStaticDataMessage {
    fn build(&self) -> Result<Cell, TonMessageError> {
        let mut builder = CellBuilder::new();
        builder.store_u32(32, Self::opcode())?;
        builder.store_u64(64, self.query_id)?;

        Ok(builder.build()?)
    }

    fn parse(cell: &Cell) -> Result<Self, TonMessageError> {
        let mut parser = cell.parser();

        let opcode: u32 = parser.load_u32(32)?;
        let query_id = parser.load_u64(64)?;
        parser.ensure_empty()?;

        let result = NftGetStaticDataMessage { query_id };
        result.verify_opcode(opcode)?;
        Ok(result)
    }
}

impl HasOpcode for NftGetStaticDataMessage {
    fn set_query_id(&mut self, query_id: u64) {
        self.query_id = query_id;
    }

    fn query_id(&self) -> u64 {
        self.query_id
    }

    fn opcode() -> u32 {
        NFT_GET_STATIC_DATA
    }
}
