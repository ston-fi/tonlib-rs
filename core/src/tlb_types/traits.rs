use crate::cell::{CellBuilder, CellParser, TonCellError};

pub trait TLBObject: Sized {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError>;

    fn write(&self, builder: &mut CellBuilder) -> Result<(), TonCellError>;
}
