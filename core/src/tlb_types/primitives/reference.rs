use std::ops::{Deref, DerefMut};

use crate::cell::{CellBuilder, CellParser, TonCellError};
use crate::tlb_types::tlb::TLB;

#[derive(Debug, PartialEq, Clone)]
pub struct Ref<T>(pub T);

impl<T> Ref<T> {
    pub const fn new(value: T) -> Self {
        Ref(value)
    }
}

impl<T> Deref for Ref<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Ref<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: TLB> TLB for Ref<T> {
    fn read_definition(parser: &mut CellParser) -> Result<Ref<T>, TonCellError> {
        Ok(Ref(T::from_cell(parser.next_reference()?.as_ref())?))
    }

    fn write_definition(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        dst.store_child(self.0.to_cell()?)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::cell::CellBuilder;
    use crate::tlb_types::primitives::reference::Ref;
    use crate::tlb_types::primitives::test_types::TestType1;
    use crate::tlb_types::tlb::TLB;

    #[test]
    fn test_ref() -> anyhow::Result<()> {
        let obj = Ref::new(TestType1 { value: 1 });
        let mut builder = CellBuilder::new();
        obj.write(&mut builder)?;
        let cell = builder.build()?;
        assert_eq!(cell.references().len(), 1);
        let parsed_back = Ref::<TestType1>::from_cell(&cell)?;
        assert_eq!(obj, parsed_back);
        Ok(())
    }
}
