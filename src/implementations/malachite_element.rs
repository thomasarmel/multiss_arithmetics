use crate::{FieldElement, Share, MERSENNE_EXP, SHARE_BYTE_SIZE};
use malachite::base::num::arithmetic::traits::{Mod, ModInverse, ModSub, ModSubAssign};
use malachite::base::num::basic::traits::Zero;
use malachite::Natural;
use rand::RngCore;
use rand_chacha::ChaCha20Rng;
use std::iter::repeat_n;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use std::sync::LazyLock;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct MalachiteElement(Natural);
static PRIME_MODULO: LazyLock<Natural> =
    LazyLock::new(|| (Natural::from(1u8) << MERSENNE_EXP) - Natural::from(1u8));
impl MalachiteElement {
    pub fn new_for_tests(raw: Natural) -> Self {
        Self(raw)
    }
    pub fn reduce_in_place(&mut self) {
        loop {
            let hi = (&self.0) >> MERSENNE_EXP;
            if hi == Natural::ZERO {
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
impl FieldElement for MalachiteElement {
    fn zero() -> Self {
        Self(Natural::from(0u8))
    }
    fn one() -> Self {
        Self(Natural::from(1u8))
    }
    fn gen_random(rng: &mut ChaCha20Rng) -> Self {
        let mut raw = [0u8; SHARE_BYTE_SIZE];
        rng.fill_bytes(&mut raw);
        Self::from(&raw)
    }

    fn from_slice(bytes: &[u8]) -> Self {
        let len = bytes.len();
        let rem = len % 8;

        // number of u64 limbs we’ll produce
        let mut u64s = Vec::with_capacity(len.div_ceil(8));

        // If length isn’t a multiple of 8, pad the *front* (most significant) chunk.
        if rem != 0 {
            let mut buf = [0u8; 8];
            buf[8 - rem..].copy_from_slice(&bytes[..rem]); // left-pad with zeros
            u64s.push(u64::from_be_bytes(buf));
        }

        // Process the remaining full 8-byte chunks as big-endian limbs.
        for chunk in bytes[rem..].chunks_exact(8) {
            let mut arr = [0u8; 8];
            arr.copy_from_slice(chunk);
            u64s.push(u64::from_be_bytes(arr));
        }

        let mut element = Self(Natural::from_owned_limbs_desc(u64s));
        element.reduce_in_place();
        element
    }
}

impl AddAssign<&Self> for MalachiteElement {
    fn add_assign(&mut self, rhs: &Self) {
        self.0.add_assign(&rhs.0);
        self.reduce_in_place();
    }
}

impl Add<&Self> for MalachiteElement {
    type Output = Self;

    fn add(self, rhs: &Self) -> Self::Output {
        let mut value = Self(self.0.add(&rhs.0));
        value.reduce_in_place();
        value
    }
}
impl SubAssign<&Self> for MalachiteElement {
    fn sub_assign(&mut self, rhs: &Self) {
        self.0.mod_sub_assign(&rhs.0, &*PRIME_MODULO)
    }
}

impl Sub<&Self> for MalachiteElement {
    type Output = Self;

    fn sub(self, rhs: &Self) -> Self::Output {
        Self(self.0.mod_sub(&rhs.0, &*PRIME_MODULO))
    }
}
impl MulAssign<&Self> for MalachiteElement {
    fn mul_assign(&mut self, rhs: &Self) {
        self.0.mul_assign(&rhs.0);
        self.reduce_in_place();
    }
}

impl Mul<&Self> for MalachiteElement {
    type Output = Self;

    fn mul(self, rhs: &Self) -> Self::Output {
        let mut value = Self(self.0.mul(&rhs.0));
        value.reduce_in_place();
        value
    }
}

impl DivAssign<&Self> for MalachiteElement {
    fn div_assign(&mut self, rhs: &Self) {
        let inv = (&rhs.0).mod_inverse(&*PRIME_MODULO).unwrap();
        self.0.mul_assign(&inv);
        self.reduce_in_place();
    }
}

impl Div<&Self> for MalachiteElement {
    type Output = Self;

    fn div(self, rhs: &Self) -> Self::Output {
        // unwrap is ok because modulo is prime
        assert_ne!(rhs, &Self::zero());

        let inv = (&rhs.0).mod_inverse(&*PRIME_MODULO).unwrap();

        let mut value = Self(self.0.mul(&inv));
        value.reduce_in_place();
        value
    }
}

impl From<Vec<u8>> for MalachiteElement {
    fn from(mut value: Vec<u8>) -> Self {
        let pad_len = 8 - value.len().rem_euclid(8);
        value.reserve_exact(pad_len);
        value.splice(0..0, repeat_n(0, pad_len));
        let mut u64s = Vec::with_capacity(value.len() / 8);

        value.chunks_exact(8).for_each(|chunk| {
            u64s.push(u64::from_be_bytes([
                chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
            ]))
        });

        let mut element = Self(Natural::from_owned_limbs_desc(u64s));
        element.reduce_in_place();
        element
    }
}

impl From<MalachiteElement> for Vec<u8> {
    fn from(value: MalachiteElement) -> Self {
        let limbs = value.0.into_limbs_desc();

        let mut bytes = Vec::with_capacity(limbs.len() * 8);
        limbs
            .into_iter()
            .for_each(|l| bytes.extend_from_slice(&l.to_be_bytes()));
        let first_nz = bytes.iter().position(|&b| b != 0).unwrap_or(bytes.len());
        bytes.drain(..first_nz);
        bytes
    }
}

impl From<&Share> for MalachiteElement {
    fn from(value: &Share) -> Self {
        // /8 always work because the lenght of a `Share` is a multiple of 8
        let mut u64s = Vec::with_capacity(value.len() / 8);

        value.chunks_exact(8).for_each(|chunk| {
            u64s.push(u64::from_be_bytes([
                chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
            ]))
        });

        let mut element = Self(Natural::from_owned_limbs_desc(u64s));
        element.reduce_in_place();
        element
    }
}
impl From<MalachiteElement> for Share {
    fn from(value: MalachiteElement) -> Self {
        let mut out = [0u8; SHARE_BYTE_SIZE];
        let limbs = value.0.into_limbs_desc();
        let mut start = SHARE_BYTE_SIZE - limbs.len() * 8;

        for limb in limbs {
            out[start..start + 8].copy_from_slice(&limb.to_be_bytes());
            start += 8;
        }

        out
    }
}

impl From<usize> for MalachiteElement {
    fn from(value: usize) -> Self {
        let raw = Natural::from(value).mod_op(&*PRIME_MODULO);
        Self(raw)
    }
}

impl From<i32> for MalachiteElement {
    fn from(value: i32) -> Self {
        let raw = Natural::from(value.unsigned_abs()).mod_op(&*PRIME_MODULO);
        if value < 0 {
            Self::zero() - &Self(raw)
        } else {
            Self(raw)
        }
    }
}

impl Neg for MalachiteElement {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::zero() - &self
    }
}

#[cfg(test)]
mod tests {

    use crate::implementations::test_field_element;

    use super::MalachiteElement;

    #[test]
    fn test_arithmetics() {
        test_field_element::<MalachiteElement>();
    }
}
