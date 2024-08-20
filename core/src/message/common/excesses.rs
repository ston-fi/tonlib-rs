use super::EXCESSES;
use crate::cell::{Cell, CellBuilder};
use crate::message::{HasOpcode, TonMessage, TonMessageError};

/// Creates a body nft excesses according to TL-B schema:
///
/// ```raw
/// excesses
///   query_id:uint64
/// = InternalMsgBody;
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct NftExcessesMessage {
    /// arbitrary request number.
    pub query_id: u64,
}

#[allow(clippy::new_without_default)]
impl NftExcessesMessage {
    pub fn new() -> Self {
        NftExcessesMessage { query_id: 0 }
    }
}

impl TonMessage for NftExcessesMessage {
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

        let result = NftExcessesMessage { query_id };
        result.verify_opcode(opcode)?;
        Ok(result)
    }
}

impl HasOpcode for NftExcessesMessage {
    fn set_query_id(&mut self, query_id: u64) {
        self.query_id = query_id;
    }

    fn query_id(&self) -> u64 {
        self.query_id
    }

    fn opcode() -> u32 {
        EXCESSES
    }
}
