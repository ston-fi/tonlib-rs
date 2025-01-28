use num_bigint::BigUint;
use num_traits::{One, Zero};

/// All functions except `add_leading_bit` expect 1 extra leading bit in `val` set to 1
pub(super) fn all_bits_same(val: &BigUint) -> bool {
    if val.is_zero() {
        return true;
    }
    let origin_bits = val.bits();
    let all_zero = (val - 1u32).bits() != origin_bits;
    let all_ones = (val + 1u32).bits() != origin_bits;
    all_zero || all_ones
}

pub(super) fn common_prefix_len(a: &BigUint, b: &BigUint) -> usize {
    let xor = a ^ b;
    (a.bits() - xor.bits() - 1) as usize // don't forget leading zero
}

pub(super) fn remove_leading_bit(val: &BigUint) -> BigUint {
    let bits = val.bits();
    let mask = BigUint::one() << (bits - 1);
    val ^ mask
}

pub(super) fn add_leading_bit(val: &BigUint, val_bit_len: usize) -> BigUint {
    let leading_bit = BigUint::one() << val_bit_len;
    leading_bit | val
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_bits_same() {
        for val in [
            BigUint::from(0u32),
            BigUint::from(0b1111u32),
            BigUint::from(0b1000u32),
        ] {
            assert!(all_bits_same(&val));
        }

        let val = BigUint::from(0b1011u32);
        assert!(!all_bits_same(&val));
    }

    #[test]
    fn test_common_prefix_len() {
        let a = BigUint::from(0b1011u32);
        let b = BigUint::from(0b1010u32);
        assert_eq!(common_prefix_len(&a, &b), 2);

        let a = BigUint::from(0b1011u32);
        let b = BigUint::from(0b1011u32);
        assert_eq!(common_prefix_len(&a, &b), 3);
    }

    #[test]
    fn test_remove_leading_bit() {
        let val = BigUint::from(0b1011u32);
        assert_eq!(remove_leading_bit(&val), BigUint::from(0b011u32));

        let val = BigUint::from(0b1111u32);
        assert_eq!(remove_leading_bit(&val), BigUint::from(0b111u32));

        let val = BigUint::from(0b1u32);
        assert_eq!(remove_leading_bit(&val), BigUint::from(0u32));
    }

    #[test]
    fn test_add_leading_bit() {
        let val = BigUint::from(0b1011u32);
        assert_eq!(add_leading_bit(&val, 4), BigUint::from(0b11011u32));

        let val = BigUint::from(0b1111u32);
        assert_eq!(add_leading_bit(&val, 4), BigUint::from(0b11111u32));

        let val = BigUint::from(0u32);
        assert_eq!(add_leading_bit(&val, 1), BigUint::from(0b10u32));
    }
}
