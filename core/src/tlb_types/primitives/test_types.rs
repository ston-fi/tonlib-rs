use crate::cell::{CellBuilder, CellParser, TonCellError};
use crate::tlb_types::tlb::TLB;

#[derive(Debug, PartialEq, Clone)]
pub(super) struct TestType1 {
    pub(super) value: i32,
}

#[derive(Debug, PartialEq, Clone)]
pub(super) struct TestType2 {
    pub(super) value: i64,
}

impl TLB for TestType1 {
    fn read_definition(parser: &mut CellParser) -> Result<Self, TonCellError> {
        Ok(TestType1 {
            value: parser.load_i32(32)?,
        })
    }

    fn write_definition(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        dst.store_i32(32, self.value)?;
        Ok(())
    }
}

impl TLB for TestType2 {
    fn read_definition(parser: &mut CellParser) -> Result<Self, TonCellError> {
        Ok(TestType2 {
            value: parser.load_i64(64)?,
        })
    }

    fn write_definition(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        dst.store_i64(64, self.value)?;
        Ok(())
    }
}
