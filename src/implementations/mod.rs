#[cfg(feature = "bigint")]
pub mod bigint_element;
#[cfg(feature = "bigint")]
use bigint_element::BigIntElement;
#[cfg(feature = "malachite")]
pub mod malachite_element;
#[cfg(feature = "malachite")]
use malachite_element::MalachiteElement;
#[cfg(feature = "rug")]
pub mod rug_element;
#[cfg(feature = "rug")]
use rug_element::RugElement;

#[cfg(test)]
use crate::FieldElement;

#[cfg(any(
    all(feature = "rug", feature = "malachite"),
    all(feature = "rug", feature = "bigint"),
    all(feature = "malachite", feature = "bigint"),
))]
compile_error!("Feature `malachite` (default), `rug`, and `bigint` are mutually exclusive and cannot be enabled together, add `no-default-feature` if you enable `rug` or `bigint`");

#[cfg(feature = "rug")]
pub type Element = RugElement;
#[cfg(feature = "malachite")]
pub type Element = MalachiteElement;
#[cfg(feature = "bigint")]
pub type Element = BigIntElement;

#[macro_export]
macro_rules! vecbi {
    // Match a sequence of expressions, separated by commas, optionally ending with a comma.
    ($($x:expr),* $(,)?) => {
        {
            // Create a vector by mapping each expression to a BigInt using the from() method.
            vec![$(Element::from($x)),*]
        }
    };
}
#[cfg(test)]
/// This function tests the trait and
/// needs to be called for each implementation
/// It gathers all the tests in one function for an easier call
fn test_field_element<T: FieldElement>() {
    use rand::Rng;
    let mut rng = rand::rng();
    let secret: Vec<u8> = (0..10).map(|_| rng.random()).collect();
    let secret_t: T = secret.clone().into();
    assert_eq!(secret, secret_t.into(), "Failure with From<Vec<u8>>");

    let a = rng.random::<i16>() as i32;
    let b = rng.random::<i16>() as i32;
    let a_t = T::from(a);
    let b_t = T::from(b);
    assert_eq!(a_t.clone() + &b_t, T::from(a + b), "Failure with addition");
    assert_eq!(
        a_t.clone() - &b_t,
        T::from(a - b),
        "Failure with substraction"
    );
    assert_eq!(
        a_t.clone() * &b_t,
        T::from(a * b),
        "Failure with multiplication"
    );
    assert_eq!(
        b_t.clone() * &(a_t.clone() / &b_t),
        a_t,
        "Failure with division"
    );
}
