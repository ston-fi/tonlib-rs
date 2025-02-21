use std::ops::Deref;

use crate::cell::{CellBuilder, CellParser, TonCellError};
use crate::tlb_types::traits::TLBObject;

// https://github.com/ton-blockchain/ton/blob/2a68c8610bf28b43b2019a479a70d0606c2a0aa1/crypto/block/block.tlb#L11
#[derive(Clone, Debug, PartialEq)]
pub enum Either<L, R> {
    Left(L),
    Right(R),
}

// Either X ^X
#[derive(Clone, Debug, PartialEq)]
pub struct EitherRef<T> {
    pub value: T,
    pub layout: EitherRefLayout,
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum EitherRefLayout {
    ToCell,
    ToRef,
    Native,
}

impl<L: TLBObject, R: TLBObject> TLBObject for Either<L, R> {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        match parser.load_bit()? {
            false => Ok(Either::Left(L::read(parser)?)),
            true => Ok(Either::Right(R::read(parser)?)),
        }
    }

    fn write_to(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        match self {
            Either::Left(left) => {
                dst.store_bit(false)?;
                left.write_to(dst)?;
            }
            Either::Right(right) => {
                dst.store_bit(true)?;
                right.write_to(dst)?;
            }
        };
        Ok(())
    }
}

impl<T> EitherRef<T> {
    pub fn new(value: T) -> Self {
        EitherRef {
            value,
            layout: EitherRefLayout::Native,
        }
    }
}

impl<T: TLBObject> TLBObject for EitherRef<T> {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        match parser.load_bit()? {
            false => Ok(EitherRef {
                value: TLBObject::read(parser)?,
                layout: EitherRefLayout::ToCell,
            }),
            true => {
                let child = parser.next_reference()?;
                Ok(EitherRef {
                    value: TLBObject::from_cell(child.deref())?,
                    layout: EitherRefLayout::ToRef,
                })
            }
        }
    }

    fn write_to(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        let cell = self.value.to_cell()?;
        let serial_layout = match self.layout {
            EitherRefLayout::ToCell => EitherRefLayout::ToCell,
            EitherRefLayout::ToRef => EitherRefLayout::ToRef,
            EitherRefLayout::Native => {
                if cell.bit_len() < dst.remaining_bits() {
                    EitherRefLayout::ToCell
                } else {
                    EitherRefLayout::ToRef
                }
            }
        };
        match serial_layout {
            EitherRefLayout::ToCell => dst.store_bit(false)?.store_cell(&cell)?,
            EitherRefLayout::ToRef => dst.store_bit(true)?.store_child(cell)?,
            _ => unreachable!("Invalid EitherRefLayout value"),
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tokio_test::assert_ok;

    use super::*;
    use crate::cell::CellBuilder;
    use crate::tlb_types::primitives::test_types::{TestType1, TestType2};

    #[test]
    fn test_either_ref() -> anyhow::Result<()> {
        let obj1 = EitherRef {
            value: TestType1 { value: 1 },
            layout: EitherRefLayout::ToCell,
        };

        let obj2 = EitherRef {
            value: TestType2 { value: 2 },
            layout: EitherRefLayout::ToRef,
        };

        let obj3 = EitherRef {
            value: TestType1 { value: 3 },
            layout: EitherRefLayout::Native,
        };

        let cell = CellBuilder::new()
            .store_tlb(&obj1)?
            .store_tlb(&obj2)?
            .store_tlb(&obj3)?
            .build()?;
        let mut parser = cell.parser();
        let parsed_obj1: EitherRef<TestType1> = parser.load_tlb()?;
        let parsed_obj2: EitherRef<TestType2> = parser.load_tlb()?;
        let parsed_obj3: EitherRef<TestType1> = parser.load_tlb()?;
        assert_eq!(obj1, parsed_obj1);
        assert_eq!(obj2, parsed_obj2);
        assert_eq!(obj3.value, parsed_obj3.value);
        assert_eq!(parsed_obj3.layout, EitherRefLayout::ToCell);

        // check layout
        let mut parser = cell.parser();
        assert!(!parser.load_bit()?); // to_cell
        assert_ok!(parser.load_bits(32)); // skipping
        assert!(parser.load_bit()?); // to_ref
        assert_eq!(parser.cell.references().len(), 1);
        assert!(!parser.load_bit()?); // to_cell
        assert_ok!(parser.load_bits(32)); // skipping
        Ok(())
    }

    #[test]
    fn test_either() -> anyhow::Result<()> {
        let obj1: Either<TestType1, TestType2> = Either::Left(TestType1 { value: 1 });
        let obj2: Either<TestType1, TestType2> = Either::Right(TestType2 { value: 2 });
        let cell = CellBuilder::new()
            .store_tlb(&obj1)?
            .store_tlb(&obj2)?
            .build()?;
        let mut parser = cell.parser();
        let parsed_obj1 = parser.load_tlb()?;
        let parsed_obj2 = parser.load_tlb()?;
        assert_eq!(obj1, parsed_obj1);
        assert_eq!(obj2, parsed_obj2);

        // check raw data
        let mut parser = cell.parser();
        assert!(!parser.load_bit()?);
        assert_ok!(parser.load_bits(32)); // skipping
        assert!(parser.load_bit()?);
        Ok(())
    }
}
