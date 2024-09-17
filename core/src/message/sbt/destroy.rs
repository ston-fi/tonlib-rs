use super::SBT_DESTROY;
use crate::cell::{Cell, CellBuilder};
use crate::message::{HasOpcode, TonMessage, TonMessageError};

/// Creates a body for nft get_static_data according to TL-B schema:
///
/// ```raw
/// destroy#1f04537a
///   query_id:uint64
/// = InternalMsgBody;
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct SbtDestroyMessage {
    /// arbitrary request number.
    pub query_id: u64,
}

#[allow(clippy::new_without_default)]
impl SbtDestroyMessage {
    pub fn new() -> Self {
        SbtDestroyMessage { query_id: 0 }
    }
}

impl TonMessage for SbtDestroyMessage {
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

        let result = SbtDestroyMessage { query_id };
        result.verify_opcode(opcode)?;
        Ok(result)
    }
}

impl HasOpcode for SbtDestroyMessage {
    fn set_query_id(&mut self, query_id: u64) {
        self.query_id = query_id;
    }

    fn query_id(&self) -> u64 {
        self.query_id
    }

    fn opcode() -> u32 {
        SBT_DESTROY
    }
}

#[cfg(test)]
mod tests {
    use super::SbtDestroyMessage;
    use crate::message::{HasOpcode, TonMessage};

    #[test]
    fn sbt_destroy_msg_test() {
        let query_id = 1234567890;
        let expected = SbtDestroyMessage { query_id };

        let build_result = SbtDestroyMessage::new().with_query_id(query_id).build();
        assert!(build_result.is_ok());

        let cell = build_result.unwrap();
        let parse_result = SbtDestroyMessage::parse(&cell);
        assert!(parse_result.is_ok());

        let parsed_msg = parse_result.unwrap();
        assert_eq!(expected, parsed_msg);
    }
}
