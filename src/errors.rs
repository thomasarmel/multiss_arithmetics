use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum ShamirError {
    #[snafu(display("KGreaterThanN error : threshold (={threshold}) should be equal or smaller than the number of shares (={number_of_shares})."))]
    ThresholdGreaterThanNbOfShares {
        threshold: usize,
        number_of_shares: usize,
    },

    #[snafu(display("Threshold must be greater than 1"))]
    ThresholdTooSmall,
    #[snafu(display(
        "TooFewCoef error : there should be at least 1 coefficient to calculate a polynomial."
    ))]
    TooFewCoefs,
    #[snafu(display("IdenticalAbsc error : Lagrange abscissas should all be different."))]
    IdenticalAbsc,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum BirkhoffError {
    #[snafu(display("given the protocol inputs, the secret cannot be recovered !"))]
    ProtocolNonSolvable,
    #[snafu(display("LinearSystem error: {e}"))]
    LinearSystem { e: String },
    #[snafu(display("not enough equations provided for the given degree."))]
    NbEqnsTooLow,
}
