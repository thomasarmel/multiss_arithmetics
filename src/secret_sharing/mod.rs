use rand_chacha::ChaCha20Rng;

use crate::FieldElement;

mod birkoff;
mod packed;
pub mod shamir;

/// Represents a polynomial with its coefficients starting with the constant term
/// `vec![3,0,2]` represents P(x) = 2x²+3
pub struct Polynomial<T>(Vec<T>);
impl<T: FieldElement> Polynomial<T> {
    pub fn new_shamir(secret: &[u8], degree: usize, rng: &mut ChaCha20Rng) -> Self {
        let mut coefficients = Vec::with_capacity(degree + 1);
        coefficients.push(T::from_slice(secret));
        for _ in 1..=degree {
            coefficients.push(T::gen_random(rng));
        }
        Self(coefficients)
    }
    pub fn evaluate_with_horner_method(&self, x: &T, derivative_order: usize) -> T {
        let derived = if derivative_order != 0 {
            &self.derive(derivative_order)
        } else {
            self
        };
        let mut acc = T::zero();
        for c in derived.0.iter().rev() {
            acc *= x;
            acc += c;
        }
        acc
    }
    /// Derives d times the polynomial
    pub fn derive(&self, d: usize) -> Self {
        let n = self.0.len();
        if d >= n {
            return Self(Vec::new());
        }

        let mut result = Vec::with_capacity(n - d);
        let mut factor: usize = (1..=d).product();

        for (i, coef) in self.0.iter().skip(d).enumerate() {
            if i > 0 {
                factor = factor * (i + d) / i;
            }
            result.push(T::from(factor) * coef);
        }

        Self(result)
    }
}

#[cfg(test)]
mod tests {
    use rand::Rng;

    use crate::{implementations::Element, secret_sharing::Polynomial, vecbi};

    // values are generate as i16 and transformed to i32 to avoid overflowing later
    fn generate_random_distinct_values(n: usize) -> (Vec<i32>, Vec<Element>) {
        let mut rng = rand::rng();
        let mut values = Vec::new();
        for _ in 0..n {
            let mut value = rng.random::<i16>();
            while values.contains(&(value as i32)) {
                value = rng.random::<i16>();
            }
            values.push(value as i32);
        }
        let big_values = values
            .clone()
            .into_iter()
            .map(|x| Element::from(x))
            .collect();
        (values, big_values)
    }
    #[test]
    fn test_evaluate_with_horner_method() {
        let polynomial = Polynomial(vecbi!(40, 9, 5, 15));
        let x = Element::from(-100);
        let derivative_order = 0;
        let result = polynomial.evaluate_with_horner_method(&x, derivative_order);
        assert_eq!(result, Element::from(-14950860));
    }

    #[test]
    fn test_derive_order_zero() {
        let (_, big_values) = generate_random_distinct_values(5);
        let polynomial = Polynomial(big_values);
        let derived = polynomial.derive(0);
        assert_eq!(derived.0, polynomial.0);
    }

    #[test]
    fn test_derive_order_one() {
        let (values, big_values) = generate_random_distinct_values(5);
        let polynomial = Polynomial(big_values);
        let derived = polynomial.derive(1);
        assert_eq!(
            derived.0,
            vecbi!(values[1], values[2] * 2, values[3] * 3, values[4] * 4)
        );
    }
    #[test]
    fn test_derive_order_two() {
        let (values, big_values) = generate_random_distinct_values(5);
        let polynomial = Polynomial(big_values);
        let derived = polynomial.derive(2);
        assert_eq!(
            derived.0,
            vecbi!(values[2] * 2, values[3] * 6, values[4] * 12)
        );
    }

    #[test]
    fn test_evaluate_with_horner_method_derivative() {
        let polynomial = Polynomial(vec![Element::from(3), Element::from(2), Element::from(1)]);
        let x = Element::from(2);
        let derivative_order = 1;
        let result = polynomial.evaluate_with_horner_method(&x, derivative_order);
        assert_eq!(result, Element::from(6));
    }

    #[test]
    fn test_evaluate_with_horner_method_homomorphic() {
        let (_, big_values_a) = generate_random_distinct_values(5);
        let (_, big_values_b) = generate_random_distinct_values(5);
        let mut rng = rand::rng();
        let x = Element::from(rng.random::<i32>());
        let derivative_order = rng.random_range(0..=3);

        let polynomial_a = Polynomial(big_values_a.clone());
        let polynomial_b = Polynomial(big_values_b.clone());

        let result_a = polynomial_a.evaluate_with_horner_method(&x, derivative_order);
        let result_b = polynomial_b.evaluate_with_horner_method(&x, derivative_order);

        let polynomial_c = Polynomial(
            big_values_a
                .into_iter()
                .zip(big_values_b)
                .map(|(a, b)| a + &b)
                .collect(),
        );
        let result_c = polynomial_c.evaluate_with_horner_method(&x, derivative_order);

        assert_eq!(result_a + &result_b, result_c);
    }
}
