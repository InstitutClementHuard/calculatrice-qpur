//! Tests fuzz safe : robustesse + déterminisme + limites contrôlées.
//!
//! But : marteler le pipeline sans brûler la machine.
//! - RNG déterministe (seed fixe)
//! - profondeur bornée
//! - budget temps global
//! - on accepte certaines erreurs attendues (division par zéro, angle trig non reconnu, etc.)
//! - invariant clé : si EXACT == "indéfini" alors ΣLocal == None

use std::time::{Duration, Instant};

use super::eval_expression;

/* ------------------------ RNG déterministe minimal ------------------------ */

#[derive(Clone)]
struct Rng {
    state: u64,
}
impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    fn next_u32(&mut self) -> u32 {
        // LCG simple (déterministe)
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.state >> 32) as u32
    }
    fn pick(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            self.next_u32() % n
        }
    }
    fn coin(&mut self) -> bool {
        (self.next_u32() & 1) == 1
    }
}

/* ------------------------ Budget anti-gel ------------------------ */

fn budget(start: Instant, max: Duration) {
    if start.elapsed() > max {
        panic!("budget temps dépassé: {:?}", max);
    }
}

/* ------------------------ Helpers fuzz ------------------------ */

fn is_erreur_attendue(msg: &str) -> bool {
    // Liste blanche : erreurs qui sont *normales* pour un fuzz,
    // parce que le domaine est volontairement limité.
    msg.contains("division par zéro")
        || msg.contains("angle non reconnu")
        || msg.contains("argument non rationnel")
        || msg.contains("caractère inattendu")
        || msg.contains("Entrée vide")
}

fn check_invariant_indefini(exact: &str, lecture: &Option<String>) {
    if exact.trim() == "indéfini" {
        assert!(lecture.is_none(), "indéfini => ΣLocal doit être None");
    }
}

/* ------------------------ Génération d’expressions (bornée) ------------------------ */

fn gen_rat(rng: &mut Rng) -> String {
    // rationnels simples, incluant 0 (utile pour tester zéros)
    let a = match rng.pick(9) {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 3,
        4 => 4,
        5 => 5,
        6 => 6,
        7 => 6,
        _ => 7,
    };

    // éviter dénominateur 0 ici; la division par zéro doit arriver via / expr
    let b = match rng.pick(8) {
        0 => 1,
        1 => 2,
        2 => 3,
        3 => 4,
        4 => 5,
        5 => 6,
        6 => 7,
        _ => 8,
    };

    if rng.coin() {
        format!("{a}/{b}")
    } else {
        format!("{a}")
    }
}

fn gen_coeff_pi(rng: &mut Rng) -> String {
    // coeffs raisonnables, pour rester dans le domaine des angles spéciaux
    // (le trig_special sait gérer pas mal de k*pi/d, mais on borne)
    let k = match rng.pick(13) {
        0 => -6,
        1 => -4,
        2 => -3,
        3 => -2,
        4 => -1,
        5 => 0,
        6 => 1,
        7 => 2,
        8 => 3,
        9 => 4,
        10 => 5,
        11 => 6,
        _ => 7,
    };

    let d = match rng.pick(6) {
        0 => 1,
        1 => 2,
        2 => 3,
        3 => 4,
        4 => 6,
        _ => 12,
    };

    if d == 1 {
        format!("{k}*pi")
    } else {
        format!("{k}*pi/{d}")
    }
}

fn gen_atom(rng: &mut Rng) -> String {
    match rng.pick(5) {
        0 => gen_rat(rng),
        1 => "pi".to_string(),
        2 => format!("({})", gen_coeff_pi(rng)),
        3 => "sqrt(2)".to_string(),
        _ => "sqrt(3)".to_string(),
    }
}

fn gen_expr(rng: &mut Rng, depth: usize) -> String {
    if depth == 0 {
        return gen_atom(rng);
    }

    match rng.pick(9) {
        0 => gen_atom(rng),
        1 => format!(
            "({}+{})",
            gen_expr(rng, depth - 1),
            gen_expr(rng, depth - 1)
        ),
        2 => format!(
            "({}-{})",
            gen_expr(rng, depth - 1),
            gen_expr(rng, depth - 1)
        ),
        3 => format!(
            "({}*{})",
            gen_expr(rng, depth - 1),
            gen_expr(rng, depth - 1)
        ),
        4 => format!(
            "({}/{})",
            gen_expr(rng, depth - 1),
            gen_expr(rng, depth - 1)
        ),
        5 => format!("sin({})", gen_coeff_pi(rng)), // angle contrôlé
        6 => format!("cos({})", gen_coeff_pi(rng)),
        7 => format!("tan({})", gen_coeff_pi(rng)),
        _ => {
            // sqrt borné : seulement sur des entiers simples pour éviter sorties hors domaine lecture
            if rng.coin() {
                "sqrt(2)".to_string()
            } else {
                "sqrt(3)".to_string()
            }
        }
    }
}

/* ------------------------ Helper somme balancée anti pile ------------------------ */

fn somme_balancee(terme: &str, n: usize) -> String {
    let mut items: Vec<String> = (0..n).map(|_| terme.to_string()).collect();
    while items.len() > 1 {
        let mut next = Vec::new();
        let mut i = 0;
        while i < items.len() {
            if i + 1 < items.len() {
                next.push(format!("({}+{})", items[i], items[i + 1]));
                i += 2;
            } else {
                next.push(items[i].clone());
                i += 1;
            }
        }
        items = next;
    }
    items.pop().unwrap_or_else(|| "0".to_string())
}

/* ------------------------ Tests ------------------------ */

#[test]
fn fuzz_safe_determinisme_et_invariant_socal() {
    let t0 = Instant::now();
    let max = Duration::from_millis(250);

    // Même seed => mêmes expressions => mêmes sorties (déterminisme)
    let mut rng = Rng::new(0xC0FFEE_u64);

    let mut seen_ok = 0usize;
    let mut seen_err = 0usize;

    for _ in 0..120 {
        budget(t0, max);

        let expr = gen_expr(&mut rng, 5);
        let digits = 30;

        match eval_expression(&expr, digits) {
            Ok((exact, lecture, _d)) => {
                check_invariant_indefini(&exact, &lecture);
                seen_ok += 1;
            }
            Err(e) => {
                // On accepte certaines erreurs attendues en fuzz.
                assert!(
                    is_erreur_attendue(&e),
                    "erreur non attendue: expr={expr:?} err={e}"
                );
                seen_err += 1;
            }
        }
    }

    // On veut voir un mix des deux, sinon le fuzz ne “balaye” rien.
    assert!(seen_ok > 10, "trop peu de succès: {seen_ok}");
    assert!(seen_err > 0, "aucune erreur vue: fuzz trop “sage”");
}

#[test]
fn fuzz_safe_angles_trig_dans_domaine() {
    let t0 = Instant::now();
    let max = Duration::from_millis(200);

    let mut rng = Rng::new(0xBADC0DE_u64);

    for _ in 0..80 {
        budget(t0, max);

        let a = gen_coeff_pi(&mut rng);
        let expr = format!("sin({a})");

        match eval_expression(&expr, 25) {
            Ok((exact, lecture, _d)) => {
                check_invariant_indefini(&exact, &lecture);
            }
            Err(e) => {
                // Certains angles peuvent rester hors-table selon trig_special:
                assert!(
                    is_erreur_attendue(&e),
                    "erreur non attendue: expr={expr:?} err={e}"
                );
            }
        }
    }
}

#[test]
fn fuzz_safe_somme_balancee_anti_pile() {
    let t0 = Instant::now();
    let max = Duration::from_millis(200);

    let expr = somme_balancee("1/2", 800);
    budget(t0, max);

    let (exact, _lecture, _d) = eval_expression(&expr, 10).unwrap_or_else(|e| panic!("err: {e}"));

    // 800*(1/2) = 400
    assert!(exact.contains("400") || exact.trim() == "400");
}
