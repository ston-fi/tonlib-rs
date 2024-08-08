use num_bigint::BigUint;

use super::NFT_REPORT_STATIC_DATA;
use crate::cell::{Cell, CellBuilder};
use crate::message::{HasOpcode, TonMessage, TonMessageError};
use crate::TonAddress;

/// Creates a body for nft report_static_data according to TL-B schema:
///
/// ```raw
/// report_static_data query_id:uint64 index:uint256 collection:MsgAddress = InternalMsgBody
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct NftReportStaticDataMessage {
    /// arbitrary request number.
    pub query_id: u64,
    /// numerical index of this NFT in the collection, usually serial number of deployment.
    pub index: BigUint,
    /// address of the smart contract of the collection to which this NFT belongs.
    pub collection: TonAddress,
}

impl NftReportStaticDataMessage {
    pub fn new(index: BigUint, collection: TonAddress) -> Self {
        NftReportStaticDataMessage {
            query_id: 0,
            index,
            collection,
        }
    }
}

impl TonMessage for NftReportStaticDataMessage {
    fn build(&self) -> Result<Cell, TonMessageError> {
        let mut builder = CellBuilder::new();
        builder.store_u32(32, Self::opcode())?;
        builder.store_u64(64, self.query_id)?;

        builder.store_uint(256, &self.index)?;
        builder.store_address(&self.collection)?;

        Ok(builder.build()?)
    }

    fn parse(cell: &Cell) -> Result<Self, TonMessageError> {
        let mut parser = cell.parser();

        let opcode = parser.load_u32(32)?;
        let query_id = parser.load_u64(64)?;

        let index = parser.load_uint(256)?;
        let collection = parser.load_address()?;
        parser.ensure_empty()?;

        let result = NftReportStaticDataMessage {
            query_id,
            index,
            collection,
        };
        result.verify_opcode(opcode)?;
        Ok(result)
    }
}

impl HasOpcode for NftReportStaticDataMessage {
    fn set_query_id(&mut self, query_id: u64) {
        self.query_id = query_id;
    }

    fn query_id(&self) -> u64 {
        self.query_id
    }

    fn opcode() -> u32 {
        NFT_REPORT_STATIC_DATA
    }
}
