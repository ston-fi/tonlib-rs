use std::ops::{Add, ShlAssign};

use num_bigint::BigUint;
use num_traits::Zero;

#[derive(Clone)]
pub(crate) struct BitString {
    value: BigUint,
    bit_len: usize,
}

impl BitString {
    pub fn new() -> Self {
        BitString {
            value: BigUint::zero(),
            bit_len: 0,
        }
    }

    pub fn shl_assign_and_add(&mut self, rhs: usize, val: BigUint) {
        self.value.shl_assign(rhs);
        self.value += val;
        self.bit_len += rhs;
    }

    pub fn shl_assign_and_fill(&mut self, rhs: usize) {
        let val = create_biguint_with_ones(rhs);
        self.shl_assign_and_add(rhs, val)
    }

    pub fn shl_assign(&mut self, rhs: usize) {
        self.value.shl_assign(rhs);
        self.bit_len += rhs;
    }

    pub fn bit_len(&self) -> usize {
        self.bit_len
    }

    pub fn get_value_as_bytes(&self) -> Vec<u8> {
        self.value.to_bytes_be()
    }
}

impl Add<BigUint> for BitString {
    type Output = BitString;
    fn add(mut self, other: BigUint) -> BitString {
        self.value += other;
        self
    }
}
fn create_biguint_with_ones(n: usize) -> BigUint {
    let mut msb = vec![(1u8 << (n % 8)) - 1];
    let lsb = vec![0xffu8; n / 8];
    msb.extend(lsb);
    BigUint::from_bytes_be(&msb)
}

#[cfg(test)]
mod tests {
    use num_bigint::BigUint;
    use num_traits::ToPrimitive;

    use crate::cell::bit_string::create_biguint_with_ones;

    #[test]
    fn test_create_biguint_with_ones() -> anyhow::Result<()> {
        let r = create_biguint_with_ones(17);
        println!("{:08x}", r.to_u32().unwrap());
        assert_eq!(r, BigUint::from(0x1ffffu32));
        let r = create_biguint_with_ones(16);
        println!("{:08x}", r.to_u32().unwrap());
        assert_eq!(r, BigUint::from(0xffffu32));
        let r = create_biguint_with_ones(15);
        println!("{:08x}", r.to_u32().unwrap());
        assert_eq!(r, BigUint::from(0x7fffu32));
        let r = create_biguint_with_ones(13);
        println!("{:08x}", r.to_u32().unwrap());
        assert_eq!(r, BigUint::from(0x1fffu32));
        let r = create_biguint_with_ones(11);
        println!("{:08x}", r.to_u32().unwrap());
        assert_eq!(r, BigUint::from(0x07ffu32));
        let r = create_biguint_with_ones(9);
        println!("{:08x}", r.to_u32().unwrap());
        assert_eq!(r, BigUint::from(0x01ffu32));
        let r = create_biguint_with_ones(8);
        println!("{:08x}", r.to_u32().unwrap());
        assert_eq!(r, BigUint::from(0x0ffu32));
        let r = create_biguint_with_ones(7);
        println!("{:08x}", r.to_u32().unwrap());
        assert_eq!(r, BigUint::from(0x07fu32));
        let r = create_biguint_with_ones(1);
        println!("{:08x}", r.to_u32().unwrap());
        assert_eq!(r, BigUint::from(0x01u32));
        let r = create_biguint_with_ones(0);
        println!("{:08x}", r.to_u32().unwrap());
        assert_eq!(r, BigUint::from(0x00u32));
        Ok(())
    }
}
