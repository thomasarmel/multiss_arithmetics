use crate::{FieldElement, Share, MERSENNE_EXP, SHARE_BYTE_SIZE};
use num_bigint::{BigInt, Sign};
use rand::RngCore;
use rand_chacha::ChaCha20Rng;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use std::sync::LazyLock;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct BigIntElement(BigInt);

static PRIME_MODULO: LazyLock<BigInt> =
    LazyLock::new(|| (BigInt::from(1u8) << MERSENNE_EXP) - BigInt::from(1u8));

impl BigIntElement {
    pub fn new_for_tests(raw: BigInt) -> Self {
        Self(raw)
    }

    #[inline(always)]
    pub fn reduce_in_place(&mut self) {
        loop {
            let hi = &self.0 >> MERSENNE_EXP;
            if hi == BigInt::ZERO {
                break;
            }
            self.0 &= &*PRIME_MODULO;
            self.0 += hi;
        }

        if self.0 >= *PRIME_MODULO {
            self.0 -= &*PRIME_MODULO;
        }
    }
}

impl FieldElement for BigIntElement {
    fn zero() -> Self {
        Self(BigInt::from(0))
    }

    fn one() -> Self {
        Self(BigInt::from(1))
    }

    fn gen_random(rng: &mut ChaCha20Rng) -> Self {
        let mut bytes = vec![0u8; MERSENNE_EXP.div_ceil(8)];
        rng.fill_bytes(&mut bytes);
        Self::from(bytes)
    }

    fn from_slice(bytes: &[u8]) -> Self {
        let mut element = Self(BigInt::from_bytes_be(Sign::Plus, bytes));
        element.reduce_in_place();
        element
    }
}

impl AddAssign<&Self> for BigIntElement {
    #[inline(always)]
    fn add_assign(&mut self, rhs: &Self) {
        self.0 += &rhs.0;
        self.reduce_in_place();
    }
}

impl Add<&Self> for BigIntElement {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: &Self) -> Self::Output {
        let mut value = Self(self.0 + &rhs.0);
        value.reduce_in_place();
        value
    }
}

impl SubAssign<&Self> for BigIntElement {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: &Self) {
        self.0 -= &rhs.0;
        if self.0.sign() == Sign::Minus {
            self.0 += &*PRIME_MODULO;
        }
    }
}

impl Sub<&Self> for BigIntElement {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: &Self) -> Self::Output {
        let mut value = Self(self.0 - &rhs.0);
        if value.0.sign() == Sign::Minus {
            value.0 += &*PRIME_MODULO;
        }
        value
    }
}

impl MulAssign<&Self> for BigIntElement {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: &Self) {
        self.0 *= &rhs.0;
        self.reduce_in_place();
    }
}

impl Mul<&Self> for BigIntElement {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: &Self) -> Self::Output {
        let mut value = Self(self.0 * &rhs.0);
        value.reduce_in_place();
        value
    }
}

impl DivAssign<&Self> for BigIntElement {
    #[inline(always)]
    fn div_assign(&mut self, rhs: &Self) {
        let inv = rhs.0.modinv(&PRIME_MODULO).unwrap();
        self.0.mul_assign(inv);
        self.reduce_in_place();
    }
}

impl Div<&Self> for BigIntElement {
    type Output = Self;

    fn div(self, rhs: &Self) -> Self::Output {
        assert_ne!(rhs, &Self::zero());

        let inv = rhs.0.modinv(&PRIME_MODULO).unwrap();
        let mut value = Self(self.0 * inv);
        value.reduce_in_place();
        value
    }
}

impl From<Vec<u8>> for BigIntElement {
    fn from(value: Vec<u8>) -> Self {
        let mut element = Self(BigInt::from_bytes_be(Sign::Plus, &value));
        element.reduce_in_place();
        element
    }
}

impl From<BigIntElement> for Vec<u8> {
    fn from(value: BigIntElement) -> Self {
        let (_, mut bytes) = value.0.to_bytes_be();
        while bytes.first() == Some(&0) {
            bytes.remove(0);
        }
        bytes
    }
}

impl From<&Share> for BigIntElement {
    fn from(value: &Share) -> Self {
        let mut element = Self(BigInt::from_bytes_be(Sign::Plus, value));
        element.reduce_in_place();
        element
    }
}

impl From<BigIntElement> for Share {
    fn from(value: BigIntElement) -> Self {
        let mut share = [0u8; SHARE_BYTE_SIZE];
        let (_, bytes) = value.0.to_bytes_be();
        share[SHARE_BYTE_SIZE - bytes.len()..].copy_from_slice(&bytes);
        share
    }
}

impl From<usize> for BigIntElement {
    fn from(value: usize) -> Self {
        let mut element = Self(BigInt::from(value));
        element.reduce_in_place();
        element
    }
}

impl From<i32> for BigIntElement {
    fn from(value: i32) -> Self {
        let mut element = Self(BigInt::from(value));
        element.reduce_in_place();
        element
    }
}

impl Neg for BigIntElement {
    type Output = Self;

    #[inline(always)]
    fn neg(self) -> Self::Output {
        if self.0 == BigInt::ZERO {
            self
        } else {
            Self(&*PRIME_MODULO - self.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BigIntElement;
    use crate::implementations::test_field_element;

    #[test]
    fn test_arithmetics() {
        test_field_element::<BigIntElement>();
    }
}
