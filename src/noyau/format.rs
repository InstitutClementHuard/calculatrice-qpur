// src/noyau/format.rs

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};

use super::expr::Expr;

/* ------------------------ Helpers rationnels ------------------------ */

fn format_rat_pretty(r: &BigRational) -> String {
    let n = r.numer();
    let d = r.denom();
    if d.is_one() {
        format!("{n}")
    } else {
        format!("{n}/{d}")
    }
}

fn format_sqrt_of_int(n: &BigInt) -> String {
    format!("√{n}")
}

/// (p/q)*√n -> p√n/q ; √n/q si p=1 ; -√n/q si p=-1
fn format_mul_rat_sqrt(r: &BigRational, n: &BigInt) -> String {
    let p = r.numer();
    let q = r.denom();

    if p.is_zero() {
        return "0".to_string();
    }

    // p == 1
    if p == &BigInt::one() {
        if q.is_one() {
            return format_sqrt_of_int(n);
        }
        return format!("{}/{}", format_sqrt_of_int(n), q);
    }

    // p == -1
    if p == &BigInt::from(-1) {
        if q.is_one() {
            return format!("-{}", format_sqrt_of_int(n));
        }
        return format!("-{}/{}", format_sqrt_of_int(n), q);
    }

    // p entier quelconque
    if q.is_one() {
        return format!("{p}{}", format_sqrt_of_int(n));
    }
    format!("{p}{}/{}", format_sqrt_of_int(n), q)
}

/// Tente de reconnaître √(entier) et renvoie cet entier (n) si oui.
fn as_sqrt_of_int(e: &Expr) -> Option<&BigInt> {
    if let Expr::Sqrt(inner) = e {
        if let Expr::Rat(r) = inner.as_ref() {
            if r.denom().is_one() {
                return Some(r.numer());
            }
        }
    }
    None
}

/// Tente de reconnaître (Rat r) * √(entier) ou √(entier) * (Rat r).
/// Renvoie (r, n) si oui.
fn as_mul_rat_sqrt(e: &Expr) -> Option<(BigRational, BigInt)> {
    if let Expr::Mul(a, b) = e {
        // Rat * Sqrt(int)
        if let (Expr::Rat(r), Some(n)) = (a.as_ref(), as_sqrt_of_int(b.as_ref())) {
            return Some((r.clone(), n.clone()));
        }
        // Sqrt(int) * Rat
        if let (Some(n), Expr::Rat(r)) = (as_sqrt_of_int(a.as_ref()), b.as_ref()) {
            return Some((r.clone(), n.clone()));
        }
    }
    None
}

fn is_zero_expr(e: &Expr) -> bool {
    matches!(e, Expr::Rat(r) if r.is_zero())
}

fn needs_parens_for_unary_minus(e: &Expr) -> bool {
    matches!(e, Expr::Add(_, _) | Expr::Sub(_, _))
}

/* ------------------------ π “joli” ------------------------ */

/// coeff*π : affichage joli (π/2, 3π/2, -2π, etc.)
pub fn format_coeff_pi(coeff: &BigRational) -> String {
    let n = coeff.numer();
    let d = coeff.denom();

    if coeff.is_zero() {
        return "0".to_string();
    }

    // ±π
    if d.is_one() && (n == &BigInt::one() || n == &BigInt::from(-1)) {
        return if n == &BigInt::one() {
            "π".to_string()
        } else {
            "-π".to_string()
        };
    }

    // kπ
    if d.is_one() {
        return format!("{n}π");
    }

    // π/d
    if n == &BigInt::one() {
        return format!("π/{d}");
    }
    if n == &BigInt::from(-1) {
        return format!("-π/{d}");
    }

    // kπ/d
    format!("{n}π/{d}")
}

/* ------------------------ Affichage EXACT “joli” ------------------------ */

/// Formate l’expression EXACT, en privilégiant une sortie lisible:
/// - √2/2, √3/3, -√2/2, etc.
/// - évite les parenthèses lourdes quand possible
pub fn format_expr_pretty(e: &Expr) -> String {
    use Expr::*;

    match e {
        Indefini => "indéfini".to_string(),

        Rat(r) => format_rat_pretty(r),
        Pi => "π".to_string(),
        Var(s) => s.clone(),

        // √2, √3, etc. si argument entier
        Sqrt(x) => match &**x {
            Rat(r) if r.denom().is_one() => format_sqrt_of_int(r.numer()),
            _ => format!("√({})", format_expr_pretty(x)),
        },

        PowInt(x, n) => format!("({})^{n}", format_expr_pretty(x)),

        Sin(x) => format!("sin({})", format_expr_pretty(x)),
        Cos(x) => format!("cos({})", format_expr_pretty(x)),
        Tan(x) => format!("tan({})", format_expr_pretty(x)),

        // cas joli : (p/q)*√n => p√n/q (donc √2/2, √3/3, etc.)
        Mul(a, b) => {
            // (Rat)*(Sqrt(Rat(int)))
            if let (Rat(r), Sqrt(inner)) = (&**a, &**b) {
                if let Rat(nr) = &**inner {
                    if nr.denom().is_one() {
                        return format_mul_rat_sqrt(r, nr.numer());
                    }
                }
            }
            // (Sqrt(Rat(int)))*(Rat)
            if let (Sqrt(inner), Rat(r)) = (&**a, &**b) {
                if let Rat(nr) = &**inner {
                    if nr.denom().is_one() {
                        return format_mul_rat_sqrt(r, nr.numer());
                    }
                }
            }

            format!("({}*{})", format_expr_pretty(a), format_expr_pretty(b))
        }

        // a/b : on renforce les cas “√.../k” et “(p/q)*√.../k”
        Div(a, b) => {
            // denom entier simple ?
            if let Rat(rden) = &**b {
                if rden.denom().is_one() {
                    let k = rden.numer();

                    // √n / k  -> √n/k
                    if let Some(n) = as_sqrt_of_int(a.as_ref()) {
                        return format!("{}/{}", format_sqrt_of_int(n), k);
                    }

                    // ((p/q)*√n) / k -> (p/qk)*√n -> p√n/(qk)
                    if let Some((r, n)) = as_mul_rat_sqrt(a.as_ref()) {
                        let rk = r / BigRational::from_integer(k.clone());
                        return format_mul_rat_sqrt(&rk, &n);
                    }

                    // cas général : expr/k
                    let sa = format_expr_pretty(a);
                    return format!("{sa}/{}", k);
                }
            }

            // sinon affichage normal
            let sa = format_expr_pretty(a);
            format!("{sa}/{}", format_expr_pretty(b))
        }

        Add(a, b) => format!("({}+{})", format_expr_pretty(a), format_expr_pretty(b)),

        // 0 - x => -x (rendu propre), sinon affichage normal
        Sub(a, b) => {
            if is_zero_expr(a) {
                let sb = format_expr_pretty(b);
                if needs_parens_for_unary_minus(b) {
                    format!("-({sb})")
                } else {
                    format!("-{sb}")
                }
            } else {
                format!("({}-{})", format_expr_pretty(a), format_expr_pretty(b))
            }
        }
    }
}

/* ------------------------ EXACT final (avec coeff*π si reconnu) ------------------------ */

/// EXACT final : si l’expression est de la forme coeff*π, on affiche π joliment.
/// Sinon, on utilise format_expr_pretty.
pub fn format_exact_final(expr_simplifie: &Expr) -> String {
    if matches!(expr_simplifie, Expr::Indefini) {
        return "indéfini".to_string();
    }
    if let Some(c) = expr_simplifie.as_coeff_pi() {
        return format_coeff_pi(&c);
    }
    format_expr_pretty(expr_simplifie)
}
