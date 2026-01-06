use super::{FieldElement, Parameters};
use crate::errors::ShamirError;

#[derive(Debug)]
pub struct LagrangeConstants<T>(Vec<T>);

impl<T: FieldElement> LagrangeConstants<T> {
    pub fn compute(point: T, points: Vec<T>) -> Result<LagrangeConstants<T>, ShamirError> {
        let mut constants = Vec::with_capacity(points.len());
        for (i, xi) in points.clone().iter().enumerate() {
            let mut num = T::one();
            let mut denum = T::one();
            for (j, xj) in points.clone().into_iter().enumerate() {
                if j != i {
                    num *= &(xj.clone() - &point.clone());
                    denum *= &(xj - xi);
                }
            }
            let coef = num / &denum;
            constants.push(coef);
        }
        Ok(LagrangeConstants(constants))
    }

    pub fn interpolate(&self, values: Vec<T>) -> Result<T, ShamirError> {
        let constants = &self.0;
        if values.len() != constants.len() {
            return Err(ShamirError::TooFewCoefs);
        }
        Ok(values
            .into_iter()
            .zip(constants)
            .map(|(v, w)| v * w)
            .fold(T::zero(), |sum, term| sum + &term))
    }
}

pub fn lagrange_interpolation_at_point<T: FieldElement>(
    point: T,
    xs: Vec<T>,
    ys: Vec<T>,
) -> Result<T, ShamirError> {
    let constants = LagrangeConstants::compute(point, xs)?; // Todo: precompute this
    constants.interpolate(ys)
}

/// parameters: Needs to be mutable because of the contained rng that changes its internal state.
pub fn share_lagrange<T: FieldElement>(
    secrets: Vec<T>,
    parameters: &mut Parameters,
) -> Result<Vec<T>, ShamirError> {
    let num_values = parameters.threshold + secrets.len();
    // Secret points
    let secret_points = (0..num_values).map(T::from).map(|p| -p).collect::<Vec<T>>();

    let mut values = secrets;

    // Generate and append random values
    for _ in 0..parameters.threshold {
        let v = T::gen_random(&mut parameters.chacha_rand);
        values.push(v);
    }

    let shares = (1..=num_values)
        .map(T::from)
        .map(|p| {
            lagrange_interpolation_at_point::<T>(p, secret_points.clone(), values.clone()).unwrap()
        })
        .collect::<Vec<T>>();

    Ok(shares)
}

#[cfg(test)]
mod tests {
    use crate::{Element, FieldElement, Parameters};

    use super::{lagrange_interpolation_at_point, share_lagrange};

    #[test]
    fn shamir_packed_ok() {
        let secrets = [
            b"tests".to_vec(),
            b"with".to_vec(),
            b"more".to_vec(),
            b"data".to_vec(),
        ]
        .to_vec();
        let secrets_bi = secrets
            .clone()
            .into_iter()
            .map(|s| Element::from(s))
            .collect::<Vec<Element>>();

        let secrets_len = secrets.len();
        let k = 4;
        let n = 13;
        let mut parameters = Parameters::new(k, n).unwrap();

        let ys = share_lagrange(secrets_bi.clone(), &mut parameters).unwrap();
        let xs = (1..=k + secrets_len)
            .map(|i| Element::from(i))
            .collect::<Vec<Element>>();

        // Secret points
        let secret_points = (0..secrets_len)
            .map(Element::from)
            .map(|p| Element::zero() - &p)
            .collect::<Vec<Element>>();
        let res = secret_points
            .iter()
            .map(|p| {
                lagrange_interpolation_at_point::<Element>(p.clone(), xs.clone(), ys.clone())
                    .unwrap()
            })
            // .collect::<Vec<Element>>();
            .map(|s| s.into())
            .collect::<Vec<Vec<u8>>>();

        assert_eq!(secrets, res);
    }
}
