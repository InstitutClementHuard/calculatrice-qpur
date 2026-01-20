// src/noyau/lecture.rs

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use super::expr::Expr;

/* ------------------------ Décimal (scaled -> texte) ------------------------ */

fn pow10(n: usize) -> BigInt {
    BigInt::from(10).pow(n as u32)
}

/// Convertit un entier “scalé” (×10^digits) en texte décimal tronqué.
pub fn scaled_to_decimal(mut scaled: BigInt, digits: usize) -> String {
    let neg = scaled.is_negative();
    if neg {
        scaled = -scaled;
    }

    let scale = pow10(digits);
    let int_part = &scaled / &scale;
    let frac_part = &scaled % &scale;

    if digits == 0 {
        return if neg {
            format!("-{int_part}")
        } else {
            format!("{int_part}")
        };
    }

    let mut frac = frac_part.to_str_radix(10);
    while frac.len() < digits {
        frac.insert(0, '0');
    }

    if neg {
        format!("-{int_part}.{frac}")
    } else {
        format!("{int_part}.{frac}")
    }
}

/// r -> entier “scalé” = floor(r * 10^digits)
fn rational_scaled(r: &BigRational, digits: usize) -> BigInt {
    let scale = pow10(digits);
    (r.numer() * scale) / r.denom()
}

/* ------------------------ π (Machin) + cache ------------------------ */

/// arctan(1/q) en entier scalé (troncature) via série:
/// atan(z) = z - z^3/3 + z^5/5 - ...
fn arctan_inv_q_scaled(q: i64, scale: &BigInt) -> BigInt {
    let q = BigInt::from(q);

    let mut k: usize = 0;
    let mut sign_pos = true;

    // q^(2k+1)
    let mut q_pow = q.clone();
    let mut sum = BigInt::zero();

    loop {
        let denom = BigInt::from((2 * k + 1) as i64);
        let d = &q_pow * &denom;

        let term = scale / &d;
        if term.is_zero() {
            break;
        }

        if sign_pos {
            sum += &term;
        } else {
            sum -= &term;
        }

        // q_pow *= q^2
        q_pow *= &q;
        q_pow *= &q;

        sign_pos = !sign_pos;
        k += 1;
    }

    sum
}

fn pi_scaled_compute(digits: usize) -> BigInt {
    // extra pour amortir les erreurs de troncature
    let extra = 10usize;
    let scale = pow10(digits + extra);

    // Machin : π = 16*atan(1/5) - 4*atan(1/239)
    let a = arctan_inv_q_scaled(5, &scale);
    let b = arctan_inv_q_scaled(239, &scale);

    let mut pi = BigInt::from(16) * a - BigInt::from(4) * b;

    // retire les digits extra
    pi /= pow10(extra);
    pi
}

static PI_CACHE: OnceLock<Mutex<HashMap<usize, BigInt>>> = OnceLock::new();

fn pi_scaled_cached(digits: usize) -> BigInt {
    let m = PI_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = m.lock().expect("mutex π");

    if let Some(v) = guard.get(&digits) {
        return v.clone();
    }

    let v = pi_scaled_compute(digits);
    guard.insert(digits, v.clone());
    v
}

/* ------------------------ √ en lecture (approx scalée) ------------------------ */

/// sqrt(r) en entier scalé : floor( sqrt(r) * 10^digits )
/// r = n/d
fn rational_sqrt_scaled(r: &BigRational, digits: usize) -> BigInt {
    let n = r.numer().clone();
    let d = r.denom().clone();

    if n.is_zero() {
        return BigInt::zero();
    }

    // On veut y ≈ sqrt(n/d) * 10^digits
    // => y^2 ≈ (n * 10^(2*digits)) / d
    let scale2 = pow10(2 * digits);
    let target = n * scale2;

    // point de départ
    let mut y = pow10(digits);
    if y.is_zero() {
        y = BigInt::one();
    }

    // Newton sur y pour sqrt(target/d)
    loop {
        let denom = &d * &y;
        if denom.is_zero() {
            break;
        }

        let q = &target / denom;
        let y_next = (&y + q) >> 1;

        if y_next == y || y_next == (&y - 1u32) {
            // ajustement final (floor)
            let mut y_adj = y_next;

            while (&y_adj + 1u32) * (&y_adj + 1u32) * &d <= target {
                y_adj += 1u32;
            }
            while &y_adj * &y_adj * &d > target {
                y_adj -= 1u32;
            }
            return y_adj;
        }

        y = y_next;
    }

    y
}

/* ------------------------ ΣLocal : évaluation scalée ------------------------ */

/// Évalue une expression en entier “scalé” (×10^digits).
/// - Bloque si Indefini.
/// - Bloque si Var (défense en profondeur).
/// - Pi utilise cache.
/// - Trig: on compte sur simplify() (angles spéciaux) => Rat ou Indefini.
/// - PowInt: base rationnelle seulement (MVP).
/// - Sqrt: argument rationnel seulement (MVP).
pub fn eval_scaled(expr: &Expr, digits: usize) -> Result<BigInt, String> {
    use Expr::*;

    let scale = pow10(digits);

    match expr {
        Indefini => Err("indéfini".into()),

        // ✅ défense en profondeur : ΣLocal exige une valeur pour chaque Var
        Var(_) => Err("variable non évaluable (ΣLocal bloquée)".into()),

        Rat(r) => Ok(rational_scaled(r, digits)),
        Pi => Ok(pi_scaled_cached(digits)),

        Add(a, b) => Ok(eval_scaled(a, digits)? + eval_scaled(b, digits)?),
        Sub(a, b) => Ok(eval_scaled(a, digits)? - eval_scaled(b, digits)?),

        Mul(a, b) => {
            let sa = eval_scaled(a, digits)?;
            let sb = eval_scaled(b, digits)?;
            Ok((sa * sb) / &scale)
        }

        Div(a, b) => {
            let sa = eval_scaled(a, digits)?;
            let sb = eval_scaled(b, digits)?;
            if sb.is_zero() {
                return Err("division par zéro".into());
            }
            Ok((sa * &scale) / sb)
        }

        PowInt(base, n) => {
            // MVP : seulement si base rationnelle
            if let Rat(r) = &**base {
                let rr = rational_pow_int(r.clone(), *n);
                return Ok(rational_scaled(&rr, digits));
            }
            Err("puissance : base non rationnelle (à étendre)".into())
        }

        Sqrt(x) => {
            // MVP : seulement si argument rationnel
            let xr = match &**x {
                Rat(r) => r.clone(),
                _ => return Err("√ : argument non rationnel (à étendre)".into()),
            };
            if xr.is_negative() {
                return Err("√ : argument négatif".into());
            }
            Ok(rational_sqrt_scaled(&xr, digits))
        }

        Sin(_) | Cos(_) | Tan(_) => {
            // MVP : on simplifie d’abord; si ça devient Rat/Indefini/Pi, ok; sinon non reconnu
            let simp = expr.clone().simplify();
            match simp {
                Indefini => Err("indéfini".into()),
                Var(_) => Err("variable non évaluable (ΣLocal bloquée)".into()),
                Rat(r) => Ok(rational_scaled(&r, digits)),
                Pi => Ok(pi_scaled_cached(digits)),
                _ => Err("trig : angle non reconnu (angles spéciaux seulement)".into()),
            }
        }
    }
}

/* ------------------------ Outil interne (PowInt) ------------------------ */

fn rational_pow_int(base: BigRational, exp: i64) -> BigRational {
    if exp == 0 {
        return BigRational::one();
    }
    if exp < 0 {
        let pos = rational_pow_int(base.clone(), -exp);
        return BigRational::one() / pos;
    }

    let mut e = exp as u64;
    let mut acc = BigRational::one();
    let mut b = base;

    while e > 0 {
        if (e & 1) == 1 {
            acc *= b.clone();
        }
        e >>= 1;
        if e > 0 {
            b *= b.clone();
        }
    }
    acc
}
