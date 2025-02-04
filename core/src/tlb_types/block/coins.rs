use num_bigint::BigUint;
use num_traits::Zero;

use crate::cell::{ArcCell, CellBuilder, CellParser, TonCellError};
use crate::tlb_types::traits::TLBObject;

// https://github.com/ton-blockchain/ton/blob/050a984163a53df16fb03f66cc445c34bfed48ed/crypto/block/block.tlb#L124
#[derive(Clone, Debug, PartialEq)]
pub struct CurrencyCollection {
    pub grams: Grams,
    // There is a dict is TLB, but we don't reed it - so keeping raw format
    pub other: Option<ArcCell>,
}

// https://github.com/ton-blockchain/ton/blob/050a984163a53df16fb03f66cc445c34bfed48ed/crypto/block/block.tlb#L116
#[derive(Clone, Debug, PartialEq)]
pub struct Grams {
    pub amount: BigUint,
}

impl CurrencyCollection {
    pub fn new(amount: BigUint) -> Self {
        Self {
            grams: Grams::new(amount),
            other: None,
        }
    }
}

impl Grams {
    pub fn new(amount: BigUint) -> Self {
        Self { amount }
    }
}

impl TLBObject for CurrencyCollection {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        Ok(CurrencyCollection {
            grams: TLBObject::read(parser)?,
            other: TLBObject::read(parser)?,
        })
    }

    fn write_to(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        self.grams.write_to(dst)?;
        self.other.write_to(dst)?;
        Ok(())
    }
}

impl TLBObject for Grams {
    fn read(parser: &mut CellParser) -> Result<Self, TonCellError> {
        let byte_len = parser.load_u8(4)?;
        let amount = if byte_len == 0 {
            BigUint::zero()
        } else {
            parser.load_uint(byte_len as usize * 8)?
        };
        Ok(Grams { amount })
    }

    fn write_to(&self, dst: &mut CellBuilder) -> Result<(), TonCellError> {
        let bit_len = self.amount.bits();
        if bit_len == 0 {
            dst.store_u64(4, bit_len)?;
            return Ok(());
        }
        let byte_len = (bit_len + 7) / 8;
        dst.store_u64(4, byte_len)?;
        dst.store_uint(byte_len as usize * 8, &self.amount)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::tlb_types::block::coins::CurrencyCollection;
    use crate::tlb_types::traits::TLBObject;

    #[test]
    fn test_currency_collection() -> anyhow::Result<()> {
        let parsed = CurrencyCollection::from_boc_hex("b5ee9c720101010100070000094c143b1d14")?;
        assert_eq!(parsed.grams.amount, 3242439121u32.into());

        let cell_serial = parsed.to_cell()?;
        let parsed_back = CurrencyCollection::from_cell(&cell_serial)?;
        assert_eq!(parsed, parsed_back);
        Ok(())
    }

    #[test]
    fn test_currency_collection_zero_grams() -> anyhow::Result<()> {
        let currency = CurrencyCollection::new(0u32.into());
        let cell = currency.to_cell()?;
        let mut parser = cell.parser();
        let parsed: CurrencyCollection = parser.load_tlb()?;
        assert_eq!(parsed.grams.amount, 0u32.into());

        let cell_serial = parsed.to_cell()?;
        assert_eq!(cell_serial, cell);
        Ok(())
    }
}
