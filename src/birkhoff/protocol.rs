use rand_chacha::ChaCha20Rng;

use crate::{errors::BirkhoffError, secret_sharing::Polynomial, FieldElement, Share};

use super::primitives::BirkhoffShare;

pub struct BProtocol {
    degree: usize,
    subnetworks: Vec<usize>, // Vec[i] contains the number of shares for ith-derivative order
}

impl BProtocol {
    /// Returns a new Birkhoff protocol
    ///
    /// # Errors
    ///
    /// This function will return an error if the parameters would not make the system solvable
    pub fn new(degree: usize, subnetworks: Vec<usize>) -> Result<Self, BirkhoffError> {
        if Self::is_valid(degree, &subnetworks) {
            Ok(Self {
                degree,
                subnetworks,
            })
        } else {
            Err(BirkhoffError::ProtocolNonSolvable)
        }
    }

    fn is_valid(degree: usize, subnetworks: &Vec<usize>) -> bool {
        let mut number_useful_eq = 0;
        let mut current_threshold = 0;
        let required_equations = degree + 1;

        for points in subnetworks {
            number_useful_eq += points;

            // Update the threshold: Add new points, but subtract 1 for the current step
            current_threshold += *points;
            // If at any point the threshold is negative, it's impossible to determine all coefficients
            if current_threshold == 0 {
                return false;
            }
            current_threshold -= 1;

            // If the number of useful equations meets or exceeds the required number, return true
            if number_useful_eq >= required_equations {
                return true;
            }
        }

        // Final check: Ensure we have enough equations
        number_useful_eq >= required_equations
    }
}

pub fn birkhoff_split<T: FieldElement>(
    secret: &Share,
    protocol_parameters: &BProtocol,
    rng: &mut ChaCha20Rng,
) -> Vec<BirkhoffShare<T>> {
    let polynomial = Polynomial::<T>::new_shamir(secret, protocol_parameters.degree, rng);

    let mut shares_formated = Vec::new();

    for (derivative_order, number_of_shares) in protocol_parameters.subnetworks.iter().enumerate() {
        for position in 1..=*number_of_shares {
            let share =
                polynomial.evaluate_with_horner_method(&T::from(position), derivative_order);

            shares_formated.push(BirkhoffShare::new(
                T::from(position),
                derivative_order,
                share,
            ));
        }
    }

    shares_formated
}

#[cfg(test)]
mod tests {

    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    use crate::{
        birkhoff::{
            primitives::SquareLinearSystem,
            protocol::{birkhoff_split, BProtocol},
        },
        implementations::Element,
        SHARE_BYTE_SIZE,
    };

    #[test]
    fn generate_shares_order_1() {
        let birkhoff_protocol = BProtocol::new(4, vec![1, 4]).unwrap();
        let secret = [5u8; SHARE_BYTE_SIZE];

        let mut rng = ChaCha20Rng::from_os_rng();
        let mut birkhoff_shares = birkhoff_split::<Element>(&secret, &birkhoff_protocol, &mut rng);

        let system =
            SquareLinearSystem::new_birkhoff(birkhoff_protocol.degree, &mut birkhoff_shares)
                .unwrap();

        let solution = system.solution().unwrap();
        assert_eq!(Element::from(&secret), solution[0]);
    }

    #[test]
    fn generate_shares_order_2() {
        let birkhoff_protocol = BProtocol {
            degree: 4,
            subnetworks: vec![2, 0, 3], // We want the shares to be 2 shares of order 0: P(1) P(2)
                                        // and 3 shares of order 2: P''(1) P''(2) P''(3)
        };
        let secret = [5u8; SHARE_BYTE_SIZE];

        let mut rng = ChaCha20Rng::from_os_rng();
        let mut birkhoff_shares = birkhoff_split::<Element>(&secret, &birkhoff_protocol, &mut rng);

        let system =
            SquareLinearSystem::new_birkhoff(birkhoff_protocol.degree, &mut birkhoff_shares)
                .unwrap();

        let solution = system.solution().unwrap();
        assert_eq!(Element::from(&secret), solution[0]);
    }

    #[test]
    fn generate_shares_order_3() {
        let birkhoff_protocol = BProtocol {
            degree: 4,
            subnetworks: vec![3, 0, 0, 2], // We want the shares to be 2 shares of order 0: P(1) P(2)
                                           // and 3 shares of order 2: P''(1) P''(2) P''(3)
        };
        let secret = [5u8; SHARE_BYTE_SIZE];

        let mut rng = ChaCha20Rng::from_os_rng();
        let mut birkhoff_shares = birkhoff_split::<Element>(&secret, &birkhoff_protocol, &mut rng);

        let system =
            SquareLinearSystem::new_birkhoff(birkhoff_protocol.degree, &mut birkhoff_shares)
                .unwrap();

        let solution = system.solution().unwrap();
        assert_eq!(Element::from(&secret), solution[0]);
    }

    #[test]
    fn protocol_verif() {
        let protocol1 = BProtocol::new(7, vec![2, 0, 1, 0, 0, 1000]);

        assert!(protocol1.is_err());

        let protocol2 = BProtocol::new(7, vec![0, 115, 0, 1]);

        assert!(protocol2.is_err());

        let protocol3 = BProtocol::new(7, vec![2, 0, 0, 1000]);
        assert!(protocol3.is_err());

        let protocol4 = BProtocol::new(7, vec![2, 0, 1, 1000]);
        assert!(protocol4.is_ok());

        let protocol5 = BProtocol::new(7, vec![6, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert!(protocol5.is_ok());
    }
}
