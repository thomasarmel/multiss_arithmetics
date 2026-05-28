use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use arithmetics::birkhoff::primitives::SquareLinearSystem;
use arithmetics::birkhoff::protocol::{birkhoff_split, BProtocol};
use arithmetics::implementations::Element;
use arithmetics::{FieldElement, Share};

fn main() {
    // -----------------------------------------------------------------
    // 1. Initialisation et création du secret
    // -----------------------------------------------------------------
    let mut rng = ChaCha20Rng::from_seed(Default::default());
    let secret: Share = Element::gen_random(&mut rng).into();

    println!("=== Secret Initial ===");
    println!("{:?}\n", secret);

    // -----------------------------------------------------------------
    // 2. Définition du protocole de Birkhoff
    // -----------------------------------------------------------------
    // Degré du polynôme : 4
    // subnetworks = vec![1, 4] signifie :
    // - Index 0 (P)  : 1 part (évaluation du polynôme)
    // - Index 1 (P') : 4 parts (évaluations du polynôme dérivé)
    let degree = 4;
    let protocol = BProtocol::new(degree, vec![1, 4])
        .expect("Les paramètres du protocole de Birkhoff sont non-résolubles.");

    println!("Protocole de Birkhoff validé : Degré {}, Parts requises : 1 ordre 0, 4 ordre 1\n", degree);

    // -----------------------------------------------------------------
    // 3. Génération des 'Shares'
    // -----------------------------------------------------------------
    // birkhoff_split renvoie un Vec<BirkhoffShare>
    let mut shares = birkhoff_split::<Element>(&secret, &protocol, &mut rng);

    println!("=== Parts générées ===");
    for (i, share) in shares.iter().enumerate() {
        println!(
            "Part {} - Abscisse x: {:?} | Ordre (dérivation): {} | Valeur: {:?}",
            i + 1,
            share.position(),
            share.degree(),
            share.value()
        );
    }
    println!();

    // -----------------------------------------------------------------
    // 4. Reconstruction via matrice de Birkhoff
    // -----------------------------------------------------------------
    println!("=== Reconstruction en cours... ===\n");

    // Le système carré (matrice) est généré en injectant nos parts
    // SquareLinearSystem utilise l'élimination de Gauss-Jordan pour retrouver les coefficients
    let system = SquareLinearSystem::new_birkhoff(degree, &mut shares)
        .expect("Erreur lors de la construction du système linéaire de Birkhoff");

    // Résolution du système : on obtient tous les coefficients du polynôme (de c_0 à c_n)
    let coefficients = system.solution().expect("Le système linéaire n'a pas de solution.");

    // La constante c_0 (index 0) correspond au secret
    let reconstructed_secret_element = &coefficients[0];
    let final_secret: Share = reconstructed_secret_element.to_owned().into();

    // -----------------------------------------------------------------
    // 5. Vérification
    // -----------------------------------------------------------------
    println!("=== Secret Reconstruit ===");
    println!("{:?}", final_secret);

    assert_eq!(
        secret, final_secret,
        "Erreur : Le secret reconstruit ne correspond pas !"
    );

    println!("\n🎉 Succès complet ! L'interpolation de Birkhoff axée sur le polynôme dérivé a fonctionné.");
}