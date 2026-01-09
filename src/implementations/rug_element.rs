use crate::{FieldElement, Share, MERSENNE_EXP, SHARE_BYTE_SIZE};
use rand::RngCore;
use rand_chacha::ChaCha20Rng;
use rug::integer::Order;
use rug::{Complete, Integer};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use std::sync::LazyLock;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct RugElement(Integer);

static PRIME_MODULO: LazyLock<Integer> =
    LazyLock::new(|| (Integer::from(1) << MERSENNE_EXP) - Integer::from(1));

impl RugElement {
    pub fn new_for_tests(raw: Integer) -> Self {
        Self(raw)
    }

    #[inline(always)]
    pub fn reduce_in_place(&mut self) {
        loop {
            let hi = (&self.0 >> MERSENNE_EXP).complete();
            if hi == 0 {
                break;
            }
            self.0 &= &*PRIME_MODULO;
            self.0 += &hi;
        }

        if self.0 >= *PRIME_MODULO {
            self.0 -= &*PRIME_MODULO;
        }
    }
}

impl FieldElement for RugElement {
    fn zero() -> Self {
        Self(Integer::from(0))
    }

    fn one() -> Self {
        Self(Integer::from(1))
    }

    fn gen_random(rng: &mut ChaCha20Rng) -> Self {
        let mut bytes = vec![0u8; SHARE_BYTE_SIZE];
        rng.fill_bytes(&mut bytes);
        Self::from(bytes)
    }

    fn from_slice(bytes: &[u8]) -> Self {
        let mut element = Self(Integer::from_digits(bytes, Order::Msf));
        element.reduce_in_place();
        element
    }
}

impl AddAssign<&Self> for RugElement {
    #[inline(always)]
    fn add_assign(&mut self, rhs: &Self) {
        self.0 += &rhs.0;
        self.reduce_in_place();
    }
}

impl Add<&Self> for RugElement {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: &Self) -> Self::Output {
        let mut value = Self(self.0 + &rhs.0);
        value.reduce_in_place();
        value
    }
}

impl SubAssign<&Self> for RugElement {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: &Self) {
        self.0 -= &rhs.0;
        if self.0 < 0 {
            self.0 += &*PRIME_MODULO;
        }
    }
}

impl Sub<&Self> for RugElement {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: &Self) -> Self::Output {
        let mut value = Self(self.0 - &rhs.0);
        if value.0 < 0 {
            value.0 += &*PRIME_MODULO;
        }
        value
    }
}

impl MulAssign<&Self> for RugElement {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: &Self) {
        self.0 *= &rhs.0;
        self.reduce_in_place();
    }
}

impl Mul<&Self> for RugElement {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: &Self) -> Self::Output {
        let mut value = Self(self.0 * &rhs.0);
        value.reduce_in_place();
        value
    }
}

impl DivAssign<&Self> for RugElement {
    #[inline(always)]
    fn div_assign(&mut self, rhs: &Self) {
        let inv = rhs.0.clone().invert(&PRIME_MODULO).unwrap();
        self.0.mul_assign(inv);
        self.reduce_in_place();
    }
}

impl Div<&Self> for RugElement {
    type Output = Self;

    fn div(self, rhs: &Self) -> Self::Output {
        assert_ne!(rhs, &Self::zero());

        let inv = rhs.0.clone().invert(&PRIME_MODULO).unwrap();
        let mut value = Self(self.0 * inv);
        value.reduce_in_place();
        value
    }
}

impl From<Vec<u8>> for RugElement {
    fn from(value: Vec<u8>) -> Self {
        let mut element = Self(Integer::from_digits(&value, Order::Msf));
        element.reduce_in_place();
        element
    }
}

impl From<RugElement> for Vec<u8> {
    fn from(value: RugElement) -> Self {
        let mut bytes = value.0.to_digits(Order::Msf);
        while bytes.first() == Some(&0) {
            bytes.remove(0);
        }
        bytes
    }
}

impl From<&Share> for RugElement {
    fn from(value: &Share) -> Self {
        let mut element = Self(Integer::from_digits(value, Order::Msf));
        element.reduce_in_place();
        element
    }
}

impl From<RugElement> for Share {
    fn from(value: RugElement) -> Self {
        let mut share = [0u8; SHARE_BYTE_SIZE];
        let bytes = value.0.to_digits(Order::Msf);
        share[SHARE_BYTE_SIZE - bytes.len()..].copy_from_slice(&bytes);
        share
    }
}

impl From<usize> for RugElement {
    fn from(value: usize) -> Self {
        let mut element = Self(Integer::from(value));
        element.reduce_in_place();
        element
    }
}

impl From<i32> for RugElement {
    fn from(value: i32) -> Self {
        let mut element = Self(Integer::from(value));
        element.reduce_in_place();
        element
    }
}

impl Neg for RugElement {
    type Output = Self;

    #[inline(always)]
    fn neg(self) -> Self::Output {
        if self.0 == 0 {
            self
        } else {
            Self(&*PRIME_MODULO - self.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RugElement;
    use crate::implementations::test_field_element;

    #[test]
    fn test_arithmetics() {
        test_field_element::<RugElement>();
    }
}
