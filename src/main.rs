use std::env;
use std::fs;
use std::time::{Duration, Instant};

use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use serde::Deserialize;

use arithmetics::birkhoff::primitives::{BirkhoffShare, SquareLinearSystem};
use arithmetics::implementations::Element;
use arithmetics::secret_sharing::shamir::{get_lagrange_factors, lagrange, shamir, Parameters};
use arithmetics::secret_sharing::Polynomial;
use arithmetics::{FieldElement, Share, SHARE_BYTE_SIZE};

#[derive(Debug, Deserialize)]
struct NetworkConfig {
    nodes: usize,
    degree: usize,
}

#[derive(Debug, Deserialize)]
struct Config {
    mode: String,          // "standard" or "local"
    degree_p: usize,       // The degree of polynomial P (deg(P))
    networks: Vec<NetworkConfig>, // Mother and daughter configurations (deg(Q_i) or deg(R_i))
    iterations: u32,       // Number of iterations for benchmarking
}

/// Simulates standard MULTISS using P(1) and derivatives P'(i+1)
fn simulate_standard_sharing(
    secret: &Share,
    config: &Config,
    rng: &mut ChaCha20Rng,
) -> (Duration, Duration, Share) {
    let start_share = Instant::now();
    let l_networks = config.networks.len();
    assert!(l_networks > config.degree_p, "Error: Standard MULTISS needs at least deg(P) daughter networks.");

    // Generate polynomial P
    let p = Polynomial::<Element>::new_shamir(secret, config.degree_p, rng);

    // Q0 (Mother Network) uses P(1)
    let p_1: Share = p.evaluate_with_horner_method(&Element::from(1), 0).into();
    let mut params_n0 = Parameters::new(config.networks[0].degree + 1, config.networks[0].nodes).unwrap();
    let q0_shares = shamir::<Element>(&p_1, &mut params_n0);

    // Qi (Daughter Networks) use P'(i+1)
    let mut daughters_shares = Vec::new();
    for i in 1..l_networks {
        let p_prime_val: Share = p.evaluate_with_horner_method(&Element::from((i + 1) as i32), 1).into();
        let mut params_ni = Parameters::new(config.networks[i].degree + 1, config.networks[i].nodes).unwrap();
        daughters_shares.push(shamir::<Element>(&p_prime_val, &mut params_ni));
    }
    let share_time = start_share.elapsed();

    // -- Reconstruction Phase
    let start_recon = Instant::now();

    // Reconstruct Q0(0) = P(1)
    let xs_n0: Vec<i32> = (1..=(config.networks[0].degree + 1) as i32).collect();
    let factors_n0 = get_lagrange_factors::<Element>(&xs_n0).unwrap();
    let p_1_rec = lagrange::<Element>(&factors_n0, &q0_shares[..xs_n0.len()]).unwrap();

    let mut birkhoff_shares = Vec::new();
    birkhoff_shares.push(BirkhoffShare::new(Element::from(1), 0, Element::from(&p_1_rec)));

    // Reconstruct Q_i(0) = P'(i+1) for deg(P) daughter networks
    for i in 1..=config.degree_p {
        let dn_idx = i;
        let xs_ni: Vec<i32> = (1..=(config.networks[dn_idx].degree + 1) as i32).collect();
        let factors_ni = get_lagrange_factors::<Element>(&xs_ni).unwrap();
        let p_prime_rec = lagrange::<Element>(&factors_ni, &daughters_shares[dn_idx - 1][..xs_ni.len()]).unwrap();

        birkhoff_shares.push(BirkhoffShare::new(
            Element::from((dn_idx + 1) as i32), 1, Element::from(&p_prime_rec),
        ));
    }

    let system = SquareLinearSystem::new_birkhoff(config.degree_p, &mut birkhoff_shares)
        .expect("Error initializing Birkhoff linear system.");
    let coefficients = system.solution().expect("The linear system has no solution.");
    let final_secret: Share = coefficients[0].clone().into();
    let recon_time = start_recon.elapsed();

    (share_time, recon_time, final_secret)
}

/// Simulates Local MULTISS using polynomials Q_i of degree 1 and R_i for their derivatives
fn simulate_local_sharing(
    secret: &Share,
    config: &Config,
    rng: &mut ChaCha20Rng,
) -> (Duration, Duration, Share) {
    let start_share = Instant::now();
    let p = Polynomial::<Element>::new_shamir(secret, config.degree_p, rng);

    let l_networks = config.networks.len();
    assert!(l_networks >= config.degree_p + 1, "Error: Local MULTISS needs at least deg(P)+1 daughter networks to reconstruct P.");

    let mut qi_1_values = Vec::new();
    let mut daughters_shares = Vec::new();

    for i in 1..=l_networks {
        // Evaluate P(i)
        let p_i: Share = p.evaluate_with_horner_method(&Element::from(i as i32), 0).into();

        // Q_i has degree 1 in Local MULTISS
        let qi = Polynomial::<Element>::new_shamir(&p_i, 1, rng);
        let qi_1: Share = qi.evaluate_with_horner_method(&Element::from(1), 0).into();
        qi_1_values.push(qi_1);

        // Q'_i(2) (Which is the linear coefficient of Q_i)
        let qi_prime_2: Share = qi.evaluate_with_horner_method(&Element::from(2), 1).into();

        // Distribute Q'_i(2) as R_i through Shamir in the i-th daughter network
        let mut params_ni = Parameters::new(config.networks[i-1].degree + 1, config.networks[i-1].nodes).unwrap();
        daughters_shares.push(shamir::<Element>(&qi_prime_2, &mut params_ni));
    }
    let share_time = start_share.elapsed();

    // -- Reconstruction Phase
    let start_recon = Instant::now();
    let mut p_points_y = Vec::new();
    let mut p_points_x: Vec<i32> = Vec::new();

    // We only need deg(P) + 1 evaluations of P to reconstruct it
    for i in 1..=(config.degree_p + 1) {
        let dn_idx = i - 1;

        // 1. Reconstruct Q'_i(2) from R_i
        let xs_ni: Vec<i32> = (1..=(config.networks[dn_idx].degree + 1) as i32).collect();
        let factors_ni = get_lagrange_factors::<Element>(&xs_ni).unwrap();
        let qi_prime_2_rec = lagrange::<Element>(&factors_ni, &daughters_shares[dn_idx][..xs_ni.len()]).unwrap();

        // 2. Q_i(1) is available from the mother network
        let a = Element::from(&qi_prime_2_rec);
        let q1 = Element::from(&qi_1_values[dn_idx]);

        // 3. Since Q_i(X) = a*X + b, and Q_i(0) = b, we have b = Q_i(1) - a
        let mut b = q1;
        b -= &a;

        // b equals P(i)
        p_points_x.push(i as i32);
        p_points_y.push(b.into());
    }

    let factors_p = get_lagrange_factors::<Element>(&p_points_x).unwrap();
    let final_secret = lagrange::<Element>(&factors_p, &p_points_y).unwrap();
    let recon_time = start_recon.elapsed();

    (share_time, recon_time, final_secret)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run -- <path_to_config.json>");
        std::process::exit(1);
    }
    let config_path = &args[1];

    // Parse configuration file
    let config_data = fs::read_to_string(config_path)
        .unwrap_or_else(|_| panic!("Failed to read config file at: {}", config_path));
    let config: Config = serde_json::from_str(&config_data)
        .unwrap_or_else(|_| panic!("Failed to parse config. Ensure strict typing against schema."));

    println!("=== Testing Environment ===");
    println!("Mode       : {}", config.mode.to_uppercase());
    println!("deg(P)     : {}", config.degree_p);
    println!("Networks   : {:?}", config.networks);
    println!("Iterations : {}\n", config.iterations);
    println!("Share size : {} bytes", SHARE_BYTE_SIZE);

    let mut rng = ChaCha20Rng::from_seed(Default::default());
    let initial_secret: Share = Element::gen_random(&mut rng).into();

    let mut total_share_time = Duration::ZERO;
    let mut total_recon_time = Duration::ZERO;

    for i in 1..=config.iterations {
        let (share_time, recon_time, final_secret) = match config.mode.as_str() {
            "standard" => simulate_standard_sharing(&initial_secret, &config, &mut rng),
            "local" => simulate_local_sharing(&initial_secret, &config, &mut rng),
            _ => panic!("Unknown mode. Please use 'standard' or 'local'."),
        };

        // Strict correctness check per iteration
        assert_eq!(
            initial_secret, final_secret,
            "Error on iteration {}: the reconstructed secret does not match the initial one.", i
        );

        total_share_time += share_time;
        total_recon_time += recon_time;
    }

    let avg_share = total_share_time / config.iterations;
    let avg_recon = total_recon_time / config.iterations;

    println!("All evaluations succeeded seamlessly!");
    println!("Average Sharing Time        : {:?}", avg_share);
    println!("Average Reconstruction Time : {:?}", avg_recon);
}

