use std::fmt::Debug;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use rand_chacha::ChaCha20Rng;
pub mod birkhoff;
pub mod errors;
pub mod implementations;
pub mod secret_sharing;

pub const MODULO_BYTE_SIZE: usize = MERSENNE_EXP.div_ceil(8);
/// This value represents the maximum size of a unique share in bytes,
/// rounded to 8 for easier limbs manipulation
/// It is based on the prime modulo used in the calculations.
pub const SHARE_BYTE_SIZE: usize = MODULO_BYTE_SIZE.next_multiple_of(8);
pub type Share = [u8; SHARE_BYTE_SIZE];
pub const MIN_THRESHOLD: usize = 2;

/// This value defines the prime number used in the modular calculations.
/// This prime number is a Mersenne (2^M-1) to facilitate the reduction.
const MERSENNE_EXP: usize = 9689;
pub const ALLOWED: &[usize] = &[
    2,
    3,
    5,
    7,
    13,
    17,
    19,
    31,
    61,
    89,
    107,
    127,
    521,
    607,
    1279,
    2203,
    2281,
    3217,
    4253,
    4423,
    9689,
    9941,
    11213,
    19937,
    21701,
    23209,
    44497,
    86243,
    110_503,
    132_049,
    216_091,
    756_839,
    859_433,
    1_257_787,
    1_398_269,
    2_976_221,
    3_021_377,
    6_972_593,
    13_466_917,
    20_996_011,
    24_036_583,
    25_964_951,
    30_402_457,
    32_582_657,
    37_156_667,
    42_643_801,
    43_112_609,
    57_885_161,
    74_207_281,
    77_232_917,
    82_589_933,
    136_279_841,
];
const fn contains(xs: &[usize], x: usize) -> bool {
    let mut i = 0;
    while i < xs.len() {
        if xs[i] == x {
            return true;
        }
        i += 1;
    }
    false
}
const _: () = {
    assert!(
        contains(ALLOWED, MERSENNE_EXP),
        "The Mersenne exponent is not valid"
    );
};

/// This trait gathers other traits and functions necessary for finite field arithmetics
pub trait FieldElement:
    for<'a> AddAssign<&'a Self>
    + for<'a> Add<&'a Self, Output = Self>
    + for<'a> MulAssign<&'a Self>
    + for<'a> Mul<&'a Self, Output = Self>
    + for<'a> SubAssign<&'a Self>
    + for<'a> Sub<&'a Self, Output = Self>
    + for<'a> DivAssign<&'a Self>
    + for<'a> Div<&'a Self, Output = Self>
    + From<Vec<u8>>
    + Into<Vec<u8>>
    + for<'a> From<&'a Share>
    + Into<Share>
    + From<usize>
    + From<i32>
    + Clone
    + Send
    + Sync
    + Default
    + Neg<Output = Self>
    + PartialOrd
    + Ord
    + PartialEq
    + Eq
    + 'static
    + Debug
{
    fn zero() -> Self;
    fn one() -> Self;
    fn gen_random(rng: &mut ChaCha20Rng) -> Self;
    fn from_slice(value: &[u8]) -> Self;
}
