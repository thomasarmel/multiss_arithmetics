use crate::{FieldElement, SHARE_BYTE_SIZE};
use serde::{Deserialize, Serialize};

use crate::errors::BirkhoffError;

/// Represents the share received by a player: (x, 3, y) => P'''(x) = y
/// (abscissa, degree, value)
#[derive(Debug, Serialize, Deserialize)]
pub struct BirkhoffShare<T> {
    position: T,
    degree: usize,
    value: T,
}

impl<T: FieldElement> BirkhoffShare<T> {
    pub fn new(position: T, degree: usize, value: T) -> Self {
        Self {
            position,
            degree,
            value,
        }
    }
    pub fn position(&self) -> T {
        self.position.clone()
    }
    pub fn degree(&self) -> usize {
        self.degree
    }
    pub fn value(&self) -> T {
        self.value.clone()
    }
    pub fn value_as_bytes(&self) -> Vec<u8> {
        let mut value: Vec<u8> = self.value.clone().into();
        let padding = SHARE_BYTE_SIZE - value.len();
        for _ in 0..padding {
            value.insert(0, 0);
        }
        value
    }
}

pub type Matrix<T> = Vec<Vec<T>>;

pub struct SquareLinearSystem<T> {
    matrix: Matrix<T>,
    values: Vec<T>,
}

impl<T: FieldElement> SquareLinearSystem<T> {
    /// Returns a square linear system
    ///
    /// # Errors
    ///
    /// This function will return an error if :
    /// - the matrix is not square
    /// - the values has a different length than the matrix
    fn new(matrix: Matrix<T>, values: Vec<T>) -> Result<Self, BirkhoffError> {
        let n = matrix.len();
        let m = if matrix.is_empty() {
            0
        } else {
            matrix[0].len()
        };

        if n != m {
            return Err(BirkhoffError::LinearSystem {
                e: format!("Matrix (size is {n},{m})should be square."),
            });
        } else if n != values.len() {
            return Err(BirkhoffError::LinearSystem {
                e: format!(
                    "Matrix {m} and values {} should have the same length",
                    values.len()
                ),
            });
        }
        Ok(Self { matrix, values })
    }
    fn compute_derivative_terms(x: &T, degree: usize, derivative_order: usize) -> Vec<T> {
        if derivative_order > degree {
            return vec![T::zero(); degree + 1];
        }
        let mut terms = vec![T::zero(); degree + 1];
        for (i, terms_i) in terms.iter_mut().enumerate().skip(derivative_order) {
            let mut term = T::from(1);
            for j in 0..derivative_order {
                term *= &T::from(i - j);
            }
            // The matrix computed just above is only fixed by the degree of the polynomial to interpolate.
            // The matrix computed just above is only fixed by the degree of the polynome to interpolate.
            // We could speedup the computation by having these matrices precomputed.

            *terms_i = term.clone();
            for _ in 0..(i - derivative_order) {
                *terms_i *= x;
            }
        }
        terms
    }

    /// Returns a new square linear system
    ///
    /// # Errors
    ///
    /// This function will return an error if the degree is too small compared to the number of shares
    pub fn new_birkhoff(
        degree: usize,
        shares: &mut [BirkhoffShare<T>],
    ) -> Result<Self, BirkhoffError> {
        if shares.len() < degree + 1 {
            return Err(BirkhoffError::NbEqnsTooLow);
        }
        shares.sort_by_key(BirkhoffShare::degree);

        let mut matrix = vec![vec![T::zero(); degree + 1]; degree + 1];
        let mut result = vec![T::zero(); degree + 1];

        for (i, share) in shares.iter().take(degree + 1).enumerate() {
            matrix[i] = Self::compute_derivative_terms(&share.position(), degree, share.degree());
            result[i] = share.value();
        }
        Self::new(matrix, result)
    }

    /// Return the solution of the system, if any.
    pub fn solution(mut self) -> Option<Vec<T>> {
        let n = self.matrix.len();

        for i in 0..n {
            // Find pivot row and swap
            let mut max_row = i;
            for j in i + 1..n {
                if self.matrix[j][i] > self.matrix[max_row][i] {
                    max_row = j;
                }
            }
            self.matrix.swap(i, max_row);
            self.values.swap(i, max_row);

            // Pivot within A and b
            let pivot = self.matrix[i][i].clone();
            if pivot == T::zero() {
                return None;
            }
            for j in i + 1..n {
                let c = self.matrix[j][i].clone() / &pivot;
                let temp = c.clone() * &self.values[i];
                self.values[j] -= &temp;
                for k in i..n {
                    let temp = c.clone() * &self.matrix[i][k];
                    self.matrix[j][k] -= &temp;
                }
            }
        }
        let mut x = vec![T::zero(); n];
        for i in (0..n).rev() {
            x[i] = self.values[i].clone();
            for j in i + 1..n {
                let temp = self.matrix[i][j].clone() * &x[j];
                x[i] -= &temp;
            }
            x[i] /= &self.matrix[i][i];
        }

        Some(x)
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        birkhoff::primitives::{BirkhoffShare, SquareLinearSystem},
        implementations::Element,
        vecbi,
    };

    // TODO Add metamorphic test
    #[test]
    fn compute_derivative_terms_ok() {
        // This corresponds to computing the terms for the polynomial: x^4 + x^3 + x^2 + x + 1
        let order_0 = SquareLinearSystem::compute_derivative_terms(&Element::from(1), 4, 0);
        assert_eq!(order_0, vecbi![1, 1, 1, 1, 1]);
        let order_1 = SquareLinearSystem::compute_derivative_terms(&Element::from(1), 4, 1);
        assert_eq!(order_1, vecbi![0, 1, 2, 3, 4]);
        let order_2 = SquareLinearSystem::compute_derivative_terms(&Element::from(1), 4, 2);
        assert_eq!(order_2, vecbi![0, 0, 2, 6, 12]);
        let order_3 = SquareLinearSystem::compute_derivative_terms(&Element::from(1), 4, 3);
        assert_eq!(order_3, vecbi![0, 0, 0, 6, 24]);
        let order_4 = SquareLinearSystem::compute_derivative_terms(&Element::from(1), 4, 4);
        assert_eq!(order_4, vecbi![0, 0, 0, 0, 24]);
        let order_5 = SquareLinearSystem::compute_derivative_terms(&Element::from(1), 4, 5);
        assert_eq!(order_5, vecbi![0, 0, 0, 0, 0]);
    }

    #[test]
    fn construct_birkhoff_matrix_ok() {
        let degree = 3;

        // Suppose:
        // the polynomial P(x) = x^3 + x^2 + x^1 + 1
        // the first derivative P'(x) = 3 * x^2 + 2 * x + 1
        // the second derivative P''(x) = 6 * x + 2
        // the third derivative P''(x) = 6
        let mut shares: Vec<BirkhoffShare<Element>> = vec![
            BirkhoffShare::new(Element::from(1), 0, Element::from(4)), // P(1) = 4
            BirkhoffShare::new(Element::from(1), 1, Element::from(6)), // P'(1) = 6
            BirkhoffShare::new(Element::from(2), 1, Element::from(11)), // P'(2) = 11
            BirkhoffShare::new(Element::from(3), 2, Element::from(20)), // P''(3) = 20
        ];
        let system = SquareLinearSystem::new_birkhoff(degree, &mut shares).unwrap();
        let expected_matrix = vec![
            vecbi![1, 1, 1, 1],
            vecbi![0, 1, 2, 3],
            vecbi![0, 1, 4, 12],
            vecbi![0, 0, 2, 18],
        ];

        assert_eq!(expected_matrix, system.matrix);
        assert_eq!(vecbi![4, 6, 11, 20], system.values);
    }
    #[test]
    fn construct_birkhoff_matrix_other_ok() {
        let degree = 3;
        // Suppose:
        // the polynomial P(x) = 2 * x^3 + 3 * x^2 + 4 * x^1 + 5
        // the first derivative P'(x) = 6 * x^2 + 6 * x + 4
        // the second derivative P''(x) = 12 * x + 6
        // the third derivative P'''(x) = 12

        let mut shares: Vec<BirkhoffShare<Element>> = vec![
            BirkhoffShare::new(Element::from(1), 0, Element::from(14)), // P(1) = 14
            BirkhoffShare::new(Element::from(5), 1, Element::from(184)), // P'(5) = 184
            BirkhoffShare::new(Element::from(4), 1, Element::from(124)), // P'(4) = 124
            BirkhoffShare::new(Element::from(3), 2, Element::from(42)), // P''(3) = 42
        ];
        let system = SquareLinearSystem::new_birkhoff(degree, &mut shares).unwrap();
        let expected_matrix = vec![
            vecbi![1, 1, 1, 1],
            vecbi![0, 1, 10, 75],
            vecbi![0, 1, 8, 48],
            vecbi![0, 0, 2, 18],
        ];

        assert_eq!(expected_matrix, system.matrix);
        assert_eq!(vecbi![14, 184, 124, 42], system.values);
    }

    #[test]
    fn gaussian_elimination_ok() {
        let degree = 4;
        // Suppose:
        // the polynomial P(x) = 2 * x^4 + 3 * x^3 + 5 * x² + 7 * x^1 + 11
        // the derivative P'(x) = 8x³ + 9x² + 10x + 7

        let mut shares: Vec<BirkhoffShare<Element>> = vec![
            BirkhoffShare::new(Element::from(1), 0, Element::from(28)), // P(1) = 28
            BirkhoffShare::new(Element::from(1), 1, Element::from(34)), // P'(1) = 34
            BirkhoffShare::new(Element::from(2), 1, Element::from(127)), // P'(2) = 127
            BirkhoffShare::new(Element::from(3), 1, Element::from(334)), // P'(3) = 334
            BirkhoffShare::new(Element::from(4), 1, Element::from(703)), // P'(4) = 703
        ];
        let system = SquareLinearSystem::new_birkhoff(degree, &mut shares).unwrap();
        let solution = system.solution();
        assert_eq!(solution, Some(vecbi![11, 7, 5, 3, 2]));
    }

    #[test]
    fn gaussian_elimination_other_ok() {
        let degree = 5;
        // Suppose:
        // the polynomial P(x) = 3*x⁵ 2x⁴ + 4x³ + 5x² + 7x + 11
        let mut shares: Vec<BirkhoffShare<Element>> = vec![
            BirkhoffShare::new(Element::from(1), 0, Element::from(32)), // P(1) = 32
            BirkhoffShare::new(Element::from(2), 0, Element::from(205)), // P(2) = 205
            BirkhoffShare::new(Element::from(1), 2, Element::from(118)), // P''(1) = 118
            BirkhoffShare::new(Element::from(2), 2, Element::from(634)), // P''(2) = 634
            BirkhoffShare::new(Element::from(3), 2, Element::from(1918)), // P''(3) = 1918
            BirkhoffShare::new(Element::from(4), 2, Element::from(4330)), // P''(4) = 4330
        ];
        let system = SquareLinearSystem::new_birkhoff(degree, &mut shares).unwrap();
        let solution = system.solution();
        assert_eq!(solution, Some(vecbi![11, 7, 5, 4, 2, 3]));
    }
}
