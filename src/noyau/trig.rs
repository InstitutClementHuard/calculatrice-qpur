// src/noyau/trig.rs
//
// Trig spéciale (angles “exactement reconnus”) pour sin/cos/tan
// -----------------------------------------------------------
// - Extraction coeff·π via as_coeff_pi_ext()
// - Réduction modulo période via mod_rationnel() (sin/cos: 2 ; tan: 1)
// - Table angles spéciaux sur n ∈ {1,2,3,4,6}

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::ToPrimitive;

use super::expr::{mod_rationnel, Expr};

#[derive(Clone, Copy, Debug)]
pub enum TrigFn {
    Sin,
    Cos,
    Tan,
}

#[derive(Clone, Debug)]
pub enum TrigOutcome {
    Valeur(Expr, String),
    Indefini(String),
}

/// Reconnaît les angles spéciaux pour sin/cos/tan lorsque l’entrée est un multiple rationnel de π.
///
/// Retour:
/// - Some(Valeur(expr_exact, preuve)) si reconnu
/// - Some(Indefini(preuve)) si indéfini (tan(π/2), tan(3π/2))
/// - None si non reconnu
pub fn trig_special(x: &Expr, f: TrigFn) -> Option<TrigOutcome> {
    // 1) extraire coeff·π sur domaine étendu (Add/Sub/Mul/Div rationnels)
    let coeff = x.as_coeff_pi_ext()?;

    // 2) réduire modulo période
    let coeff_reduit = match f {
        TrigFn::Sin | TrigFn::Cos => mod_rationnel(&coeff, 2),
        TrigFn::Tan => mod_rationnel(&coeff, 1),
    };

    // 3) convertir en k/n "petit"
    let (k, n) = rational_to_small_kn(&coeff_reduit)?; // k/n

    // 4) réduction modulo 2π : k mod (2n) (tables sin/cos/tan codées sur [0,2π))
    let k_mod = k.rem_euclid(2 * n);

    // Constructeurs
    let rat = |a: i64, b: i64| Expr::Rat(BigRational::new(BigInt::from(a), BigInt::from(b)));
    let sub0 = |e: Expr| Expr::Sub(Box::new(rat(0, 1)), Box::new(e));

    let zero = rat(0, 1);
    let one = rat(1, 1);
    let neg_one = rat(-1, 1);
    let half = rat(1, 2);
    let neg_half = rat(-1, 2);

    let sqrt2 = Expr::Sqrt(Box::new(Expr::Rat(BigRational::from_integer(
        BigInt::from(2),
    ))));
    let sqrt3 = Expr::Sqrt(Box::new(Expr::Rat(BigRational::from_integer(
        BigInt::from(3),
    ))));

    let sqrt2_over_2 = Expr::Div(Box::new(sqrt2.clone()), Box::new(rat(2, 1)));
    let neg_sqrt2_over_2 = sub0(sqrt2_over_2.clone());

    let sqrt3_over_2 = Expr::Div(Box::new(sqrt3.clone()), Box::new(rat(2, 1)));
    let neg_sqrt3_over_2 = sub0(sqrt3_over_2.clone());

    let sqrt3_over_3 = Expr::Div(Box::new(sqrt3.clone()), Box::new(rat(3, 1)));
    let neg_sqrt3_over_3 = sub0(sqrt3_over_3.clone());

    let angle_txt = format_angle_kn_pi(k_mod, n);
    let a = (k_mod, n);

    let out = match f {
        TrigFn::Sin => match a {
            (0, _) => TrigOutcome::Valeur(zero.clone(), format!("sin({angle_txt}) = 0")),

            (1, 6) | (5, 6) => TrigOutcome::Valeur(half.clone(), format!("sin({angle_txt}) = 1/2")),
            (7, 6) | (11, 6) => {
                TrigOutcome::Valeur(neg_half.clone(), format!("sin({angle_txt}) = -1/2"))
            }

            (1, 4) | (3, 4) => {
                TrigOutcome::Valeur(sqrt2_over_2.clone(), format!("sin({angle_txt}) = √2/2"))
            }
            (5, 4) | (7, 4) => TrigOutcome::Valeur(
                neg_sqrt2_over_2.clone(),
                format!("sin({angle_txt}) = -√2/2"),
            ),

            (1, 3) | (2, 3) => {
                TrigOutcome::Valeur(sqrt3_over_2.clone(), format!("sin({angle_txt}) = √3/2"))
            }
            (4, 3) | (5, 3) => TrigOutcome::Valeur(
                neg_sqrt3_over_2.clone(),
                format!("sin({angle_txt}) = -√3/2"),
            ),

            (1, 2) => TrigOutcome::Valeur(one.clone(), format!("sin({angle_txt}) = 1")),
            (3, 2) => TrigOutcome::Valeur(neg_one.clone(), format!("sin({angle_txt}) = -1")),

            (1, 1) | (2, 1) => TrigOutcome::Valeur(zero.clone(), format!("sin({angle_txt}) = 0")),

            _ => return None,
        },

        TrigFn::Cos => match a {
            (0, _) | (2, 1) => TrigOutcome::Valeur(one.clone(), format!("cos({angle_txt}) = 1")),
            (1, 1) => TrigOutcome::Valeur(neg_one.clone(), format!("cos({angle_txt}) = -1")),

            (1, 6) | (11, 6) => {
                TrigOutcome::Valeur(sqrt3_over_2.clone(), format!("cos({angle_txt}) = √3/2"))
            }
            (5, 6) | (7, 6) => TrigOutcome::Valeur(
                neg_sqrt3_over_2.clone(),
                format!("cos({angle_txt}) = -√3/2"),
            ),

            (1, 4) | (7, 4) => {
                TrigOutcome::Valeur(sqrt2_over_2.clone(), format!("cos({angle_txt}) = √2/2"))
            }
            (3, 4) | (5, 4) => TrigOutcome::Valeur(
                neg_sqrt2_over_2.clone(),
                format!("cos({angle_txt}) = -√2/2"),
            ),

            (1, 3) | (5, 3) => TrigOutcome::Valeur(half.clone(), format!("cos({angle_txt}) = 1/2")),
            (2, 3) | (4, 3) => {
                TrigOutcome::Valeur(neg_half.clone(), format!("cos({angle_txt}) = -1/2"))
            }

            (1, 2) | (3, 2) => TrigOutcome::Valeur(zero.clone(), format!("cos({angle_txt}) = 0")),

            _ => return None,
        },

        TrigFn::Tan => match a {
            (0, _) | (1, 1) | (2, 1) => {
                TrigOutcome::Valeur(zero.clone(), format!("tan({angle_txt}) = 0"))
            }

            (1, 6) | (7, 6) => {
                TrigOutcome::Valeur(sqrt3_over_3.clone(), format!("tan({angle_txt}) = √3/3"))
            }
            (5, 6) | (11, 6) => TrigOutcome::Valeur(
                neg_sqrt3_over_3.clone(),
                format!("tan({angle_txt}) = -√3/3"),
            ),

            (1, 4) | (5, 4) => TrigOutcome::Valeur(one.clone(), format!("tan({angle_txt}) = 1")),
            (3, 4) | (7, 4) => {
                TrigOutcome::Valeur(neg_one.clone(), format!("tan({angle_txt}) = -1"))
            }

            (1, 3) | (4, 3) => TrigOutcome::Valeur(sqrt3.clone(), format!("tan({angle_txt}) = √3")),
            (2, 3) | (5, 3) => {
                TrigOutcome::Valeur(sub0(sqrt3.clone()), format!("tan({angle_txt}) = -√3"))
            }

            (1, 2) | (3, 2) => TrigOutcome::Indefini(format!("tan({angle_txt}) = indéfini")),

            _ => return None,
        },
    };

    Some(out)
}

/* ------------------------ Outils ------------------------ */

fn format_angle_kn_pi(k: i64, n: i64) -> String {
    if k == 0 {
        return "0".to_string();
    }
    if n == 1 {
        return match k {
            1 => "π".to_string(),
            _ => format!("{k}π"),
        };
    }
    if k == 1 {
        return format!("π/{n}");
    }
    format!("{k}π/{n}")
}

/// Convertit un rationnel en (k,n) i64 réduit.
/// Accepte seulement n ∈ {1,2,3,4,6}.
fn rational_to_small_kn(r: &BigRational) -> Option<(i64, i64)> {
    let denom = r.denom().to_i64()?;
    let numer = r.numer().to_i64()?;

    let g = gcd_i64(numer.abs(), denom.abs());
    let k = numer / g;
    let n = denom / g;

    if [1, 2, 3, 4, 6].contains(&n) {
        Some((k, n))
    } else {
        None
    }
}

fn gcd_i64(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a.abs()
}
