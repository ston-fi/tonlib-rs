use crate::cell::{CellBuilder, CellParser, TonCellError};
use crate::tlb_types::traits::TLBObject;

#[derive(Debug, PartialEq)]
pub(super) struct TestType1 {
    pub(super) value: i32,
}

#[derive(Debug, PartialEq)]
pub(super) struct TestType2 {
    pub(super) value: i64,
}

impl TLBObject for TestType1 {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        Ok(TestType1 {
            value: parser.load_i32(32)?,
        })
    }

    fn write_to(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        dst.store_i32(32, self.value)?;
        Ok(())
    }
}

impl TLBObject for TestType2 {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        Ok(TestType2 {
            value: parser.load_i64(64)?,
        })
    }

    fn write_to(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        dst.store_i64(64, self.value)?;
        Ok(())
    }
}
