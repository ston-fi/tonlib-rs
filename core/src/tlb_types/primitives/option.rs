use std::ops::{Deref, DerefMut};

use crate::cell::{CellBuilder, CellParser, TonCellError};
use crate::tlb_types::traits::TLBObject;

// Maybe ^X
#[derive(Clone, Debug, PartialEq, Copy)]
pub struct OptionRef<T>(pub Option<T>);

impl<T> OptionRef<T> {
    pub const NONE: OptionRef<T> = OptionRef(None);

    pub const fn new(value: T) -> Self {
        OptionRef(Some(value))
    }
}

impl<T> Deref for OptionRef<T> {
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for OptionRef<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: TLBObject> TLBObject for OptionRef<T> {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        match parser.load_bit()? {
            false => Ok(OptionRef(None)),
            true => {
                let child = parser.next_reference()?;
                let mut child_parser = child.parser();
                Ok(OptionRef(Some(TLBObject::read(&mut child_parser)?)))
            }
        }
    }

    fn write_to(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        match &self.0 {
            Some(value) => {
                dst.store_bit(true)?;
                dst.store_child(value.to_cell()?)?;
            }
            None => {
                dst.store_bit(false)?;
            }
        }
        Ok(())
    }
}

// Maybe X
impl<T: TLBObject> TLBObject for Option<T> {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        match parser.load_bit()? {
            false => Ok(None),
            true => Ok(Some(parser.load_tlb()?)),
        }
    }

    fn write_to(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        match self {
            None => dst.store_bit(false)?,
            Some(value) => dst.store_bit(true)?.store_tlb(value)?,
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tokio_test::assert_ok;

    use super::*;
    use crate::cell::CellBuilder;
    use crate::tlb_types::primitives::test_types::TestType1;

    #[test]
    fn test_option() -> anyhow::Result<()> {
        let obj1 = Some(TestType1 { value: 1 });
        let obj2: Option<TestType1> = None;
        let cell = CellBuilder::new()
            .store_tlb(&obj1)?
            .store_tlb(&obj2)?
            .build()?;
        let mut parser = cell.parser();
        let parsed_obj1: Option<TestType1> = parser.load_tlb()?;
        let parsed_obj2: Option<TestType1> = parser.load_tlb()?;
        assert_eq!(obj1, parsed_obj1);
        assert_eq!(None, parsed_obj2);

        // check layout
        let mut parser = cell.parser();
        assert!(parser.load_bit()?); // Some
        assert_ok!(parser.load_bits(32)); // skipping
        assert!(!parser.load_bit()?); // None
        Ok(())
    }

    #[test]
    fn test_option_ref() -> anyhow::Result<()> {
        let obj1 = OptionRef(Some(TestType1 { value: 1 }));
        let obj2: OptionRef<TestType1> = OptionRef(None);
        let cell = CellBuilder::new()
            .store_tlb(&obj1)?
            .store_tlb(&obj2)?
            .build()?;
        let mut parser = cell.parser();
        let parsed_obj1: OptionRef<TestType1> = parser.load_tlb()?;
        let parsed_obj2: OptionRef<TestType1> = parser.load_tlb()?;
        assert_eq!(obj1, parsed_obj1);
        assert_eq!(obj2, parsed_obj2);

        // check layout
        let mut parser = cell.parser();
        assert!(parser.load_bit()?); // Some
        assert_eq!(parser.references.len(), 1);
        assert!(!parser.load_bit()?); // None
        Ok(())
    }
}
