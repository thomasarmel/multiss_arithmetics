use std::{
    sync::mpsc::{sync_channel, Receiver},
    thread,
};

use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use tokio::{sync::mpsc, task::spawn_blocking};
use tracing::{debug, error};

use crate::{
    errors::ShamirError, secret_sharing::Polynomial, FieldElement, Share, SHARE_BYTE_SIZE,
};

#[derive(Debug, Clone)]
pub struct Parameters {
    threshold: usize,
    nb_of_shares: usize,
    chacha_rand: ChaCha20Rng,
}

impl Parameters {
    /// Return a new and valid threshold, nb of shares pair along with a random generator.
    ///
    /// # Errors
    ///
    /// This function will return an error if :
    /// - `threshold` is too small (<= 1)
    /// - `nb_of_shares` is smaller than `threshold`
    pub fn new(threshold: usize, nb_of_shares: usize) -> Result<Self, ShamirError> {
        if threshold > nb_of_shares {
            return Err(ShamirError::ThresholdGreaterThanNbOfShares {
                threshold,
                number_of_shares: nb_of_shares,
            });
        }

        if threshold <= 1 {
            return Err(ShamirError::ThresholdTooSmall);
        }
        let chacha_rand = ChaCha20Rng::from_os_rng();

        Ok(Parameters {
            threshold,
            nb_of_shares,
            chacha_rand,
        })
    }

    pub fn threshold(&self) -> usize {
        self.threshold
    }

    pub fn nb_of_shares(&self) -> usize {
        self.nb_of_shares
    }

    pub fn chacha_rand(&mut self) -> &mut ChaCha20Rng {
        &mut self.chacha_rand
    }
}

/// Returns a vector of Strings corresponding to the values of Shamir's Secret Sharing Scheme.
/// The corresponding points are always 1 to n (included).
///
/// # Parameters
///
/// - self : the secret
/// - `parameters`: needs to be mutable because of the rng, which has an internal state
pub fn shamir<T: FieldElement>(secret: &Share, parameters: &mut Parameters) -> Vec<Share> {
    let polynomial = Polynomial::<T>::new_shamir(
        secret,
        parameters.threshold - 1,
        &mut parameters.chacha_rand,
    );

    let mut shares = Vec::with_capacity(parameters.nb_of_shares);
    for i in 1..=parameters.nb_of_shares {
        let share: Share = polynomial.evaluate_with_horner_method(&i.into(), 0).into();
        if share.is_empty() {
            shares.push([0u8; SHARE_BYTE_SIZE]);
        } else {
            shares.push(share);
        }
    }
    shares
}
pub struct Shamirizer<T> {
    random_shares: Receiver<T>,
    degree: usize,
    n: usize,
}
impl<T: FieldElement> Shamirizer<T> {
    pub fn initialize(degree: usize, n: usize) -> Self {
        let (sender, receiver) = sync_channel(10);
        thread::spawn(move || {
            let mut rng = ChaCha20Rng::from_os_rng();
            loop {
                let share = T::gen_random(&mut rng);
                if sender.send(share).is_err() {
                    debug!("Random shares receiver is closed.");
                    return;
                }
            }
        });
        Self {
            random_shares: receiver,
            degree,
            n,
        }
    }
    pub fn shamirize(&self, secret: &Share) -> Vec<Share> {
        let mut coefficients = Vec::with_capacity(self.degree + 1);
        coefficients.push(T::from(secret));
        for _ in 1..=self.degree {
            coefficients.push(self.random_shares.recv().unwrap());
        }
        let polynomial = Polynomial(coefficients);
        let mut shares = Vec::with_capacity(self.n);
        for i in 1..=self.n {
            let share: Share = polynomial.evaluate_with_horner_method(&i.into(), 0).into();
            if share.is_empty() {
                shares.push([0u8; SHARE_BYTE_SIZE]);
            } else {
                shares.push(share);
            }
        }
        shares
    }
}

/// Function used by `lagrange`
/// It calculates the optimized lagrange polynomials using [this formula](https://wikimedia.org/api/rest_v1/media/math/render/svg/d8baebefd1c30e68f599e610ccf8d6e6635ff1d2)
/// More info [here](https://en.wikipedia.org/wiki/Shamir's_secret_sharing#Computationally_efficient_approach)
///
/// # Errors
///
/// Will return `Err` if two abscissas (`xs`) are identical
pub fn get_lagrange_factors<T: FieldElement>(xs: &[i32]) -> Result<Vec<T>, ShamirError> {
    let n = xs.len();
    let xs = xs.iter().map(|x| T::from(*x)).collect::<Vec<T>>();
    let product: T = xs.iter().fold(T::one(), |prod, x| prod * x);

    let mut denominators = Vec::with_capacity(n);

    for (i, xi) in xs.iter().enumerate() {
        denominators.push(xi.clone());
        for (j, xj) in xs.iter().enumerate() {
            if i != j {
                if xi == xj {
                    return Err(ShamirError::IdenticalAbsc);
                }
                denominators[i] *= &(xj.clone() - xi);
            }
        }
    }

    let mut factors = vec![product.clone(); n];

    for i in 0..n {
        factors[i] /= &denominators[i];
    }

    Ok(factors)
}

/// Returns a `Part` corresponding to the secret hidden in the shares, the result of Lagrange's interpolation calculated in 0.
///
/// # Parameters
///
/// - points : the horizontal axis values of the shares, always from 1 to n
/// - shares : the values of Shamir's secret sharing scheme
/// - p : the modulo for the calculation
///
/// # Errors
///
/// This function will return an [`InputSim`](crate::errors::Error::InputSim) error if some points have the same value.
pub fn lagrange<T: FieldElement>(factors: &[T], ys: &[Share]) -> Result<Share, ShamirError> {
    let mut secret = T::zero();

    for (factor, y) in factors.iter().zip(ys.iter()) {
        let mut term = T::from(y);
        term *= factor;
        secret += &term;
    }

    Ok(secret.into())
}

/// This function takes a secret and a set of parameters and returns a vector of channels.
/// Each channel will stream the bytes of a single Shamir share.
/// The function returns immediately. The share generation is done in a background thread.
pub fn stream_shamir_shares<T: FieldElement>(
    secret: Share,
    parameters: &mut Parameters,
) -> Vec<mpsc::Receiver<Share>> {
    let mut senders = Vec::with_capacity(parameters.nb_of_shares);
    let mut receivers = Vec::with_capacity(parameters.nb_of_shares);

    for _ in 0..parameters.nb_of_shares {
        let (sender, receiver) = mpsc::channel(1);
        senders.push(sender);
        receivers.push(receiver);
    }

    let mut chacha_rand = parameters.chacha_rand.clone();
    let threshold = parameters.threshold;

    spawn_blocking(move || {
        let polynomial = Polynomial::<T>::new_shamir(&secret, threshold - 1, &mut chacha_rand);

        senders.into_par_iter().enumerate().for_each(|(i, sender)| {
            // .enumerate() gives a 0-based index, so we add 1 for the share value.
            let share_index = i + 1;

            let share = polynomial.evaluate_with_horner_method(&share_index.into(), 0);

            if sender.blocking_send(share.into()).is_err() {
                error!("Error sending shares, receiver seems to be down.");
            }
        });
    });

    receivers
}

#[cfg(test)]
mod tests {

    use crate::{implementations::Element, vecbi};

    use super::*;
    use rand::{seq::SliceRandom, Rng, SeedableRng};
    use tokio_stream::{wrappers::ReceiverStream, StreamExt, StreamMap};

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
    fn test_new_shamir() {
        let mut rng = rand::rng();

        let mut chacha_rand = ChaCha20Rng::from_os_rng();
        let secret: Share = Element::gen_random(&mut chacha_rand).into();
        let degree = rng.random_range(1..10);
        let mut rng = ChaCha20Rng::from_os_rng();
        let polynomial = Polynomial::<Element>::new_shamir(&secret, degree, &mut rng);
        assert_eq!(polynomial.0.len(), degree + 1);
        assert_eq!(polynomial.0[0], Element::from(&secret));
    }

    #[test]
    fn shamir_correct_number_of_shares() {
        let mut rng = rand::rng();
        let mut chacha_rand = ChaCha20Rng::from_os_rng();
        let secret: Share = Element::gen_random(&mut chacha_rand).into();

        let k = rng.random_range(1..100);
        let n = rng.random_range(k..200);
        let mut parameters = Parameters::new(k, n).unwrap();

        let shares = shamir::<Element>(&secret, &mut parameters);
        assert_eq!(shares.len(), n, "n shares should be produced");
    }
    #[test]
    fn get_num_and_denoms_input_sim() {
        let mut rng = rand::rng();
        let a = rng.random::<i32>();
        match get_lagrange_factors::<Element>(&[a, a]) {
            Err(ShamirError::IdenticalAbsc) => (),
            _ => panic!("Sould return InputSim error"),
        }
    }
    #[test]
    fn get_lagrange_factors_correct_num() {
        let factors = get_lagrange_factors::<Element>(&vec![1, 2, 3, 4]).unwrap();
        assert_eq!(factors, vecbi!(4, -6, 4, -1));
    }
    #[test]
    fn get_lagrange_factors_permutation() {
        let n = rand::rng().random_range(2..10);
        let (mut values, _) = generate_random_distinct_values(n);
        let factors1 = get_lagrange_factors::<Element>(&values).unwrap();
        values.shuffle(&mut rand::rng());
        let factors2 = get_lagrange_factors::<Element>(&values).unwrap();
        assert!(factors1.iter().all(|x| factors2.contains(x)));
    }
    #[test]
    fn lagrange_correct_input() {
        let xs = vec![1, 2, 3];
        let mut y1 = [0u8; SHARE_BYTE_SIZE];
        y1[SHARE_BYTE_SIZE - 1] = 15;
        let mut y2 = [0u8; SHARE_BYTE_SIZE];
        y2[SHARE_BYTE_SIZE - 1] = 10;
        let mut y3 = [0u8; SHARE_BYTE_SIZE];
        y3[SHARE_BYTE_SIZE - 1] = 37;
        let ys = vec![y1, y2, y3];
        let factors = get_lagrange_factors(&xs).unwrap();
        let res = lagrange::<Element>(&factors, &ys).unwrap();
        let mut expected = [0u8; SHARE_BYTE_SIZE];
        expected[SHARE_BYTE_SIZE - 1] = 52;
        assert_eq!(res, expected)
    }
    #[test]
    fn lagrange_correct_input_negative() {
        let xs = vec![1, 2, 3];
        let mut y1 = [0u8; SHARE_BYTE_SIZE];
        y1[SHARE_BYTE_SIZE - 1] = 15;
        let mut y2 = [0u8; SHARE_BYTE_SIZE];
        y2[SHARE_BYTE_SIZE - 1] = 100;
        let mut y3 = [0u8; SHARE_BYTE_SIZE];
        y3[SHARE_BYTE_SIZE - 1] = 37;
        let ys = vec![y1, y2, y3];
        let secret: Share = Element::from(-218).into();
        let factors = get_lagrange_factors(&xs).unwrap();
        let res = lagrange::<Element>(&factors, &ys).unwrap();
        assert_eq!(res, secret);
    }
    #[test]
    fn lagrange_input_permutation() {
        let mut chacha_rand = ChaCha20Rng::from_os_rng();
        let a1: Share = Element::gen_random(&mut chacha_rand).into();
        let a2: Share = Element::gen_random(&mut chacha_rand).into();
        let mut xs = vec![1, 2];
        let mut ys = vec![a1, a2];
        let factors = get_lagrange_factors(&xs).unwrap();
        let res = lagrange::<Element>(&factors, &ys).unwrap();
        xs.reverse();
        ys.reverse();
        let factors = get_lagrange_factors(&xs).unwrap();
        let res_reverse = lagrange::<Element>(&factors, &ys).unwrap();
        assert_eq!(
            res, res_reverse,
            "Result should be insensible to input permutation"
        );
    }

    #[test]
    fn shamir_lagrange() {
        let mut rng = rand::rng();
        let mut chacha_rand = ChaCha20Rng::from_os_rng();
        let e = Element::gen_random(&mut chacha_rand);
        let secret: Share = e.into();

        let k = rng.random_range(2..100);
        let n = rng.random_range(k..200);
        let mut parameters = Parameters::new(k, n).unwrap();
        let mut ys = shamir::<Element>(&secret, &mut parameters);
        let xs = (1..=k).map(|i| i as i32).collect::<Vec<i32>>();
        ys.truncate(k);
        let factors = get_lagrange_factors(&xs).unwrap();
        let res = lagrange::<Element>(&factors, &ys).unwrap();

        assert_eq!(secret, res);
    }

    #[tokio::test]
    async fn test_stream_shamir_shares() {
        let mut chacha_rand = ChaCha20Rng::from_os_rng();
        let secret: Share = Element::gen_random(&mut chacha_rand).into();

        let k = 5;
        let n = 10;
        let mut parameters = Parameters::new(k, n).unwrap();

        let receivers = stream_shamir_shares::<Element>(secret.clone(), &mut parameters);
        assert_eq!(receivers.len(), n);

        let mut stream_map = StreamMap::new();
        for (i, rx) in receivers.into_iter().enumerate() {
            stream_map.insert(i, ReceiverStream::new(rx));
        }

        let mut received_shares = Vec::with_capacity(n);
        while let Some((key, value)) = stream_map.next().await {
            received_shares.push((key, value));
        }

        // Sort shares by their original index to ensure correct order for Lagrange interpolation
        received_shares.sort_by_key(|(key, _)| *key);

        let shares_for_reconstruction: Vec<[u8; SHARE_BYTE_SIZE]> = received_shares
            .into_iter()
            .take(k)
            .map(|(_, share)| share)
            .collect();

        let xs = (1..=k).map(|i| i as i32).collect::<Vec<i32>>();
        let factors = get_lagrange_factors(&xs).unwrap();
        let res = lagrange::<Element>(&factors, &shares_for_reconstruction).unwrap();

        assert_eq!(secret, res);
    }
}
