// src/noyau/expr.rs
//
// AST exact (sans flottants).
// - Rat : rationnel exact
// - Pi  : symbole π
// - Indefini : résultat exact indéfini (ex: tan(π/2))
// - Var : variable symbolique (ex: x)
//
// IMPORTANT (SAFE):
// - simplify() ne doit jamais “inventer” une valeur pour Var.
// - ΣLocal (lecture décimale) sera bloquée dès qu'il y a Var (défense en profondeur).

use crate::noyau::canon::canon_expr;

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    Rat(BigRational),
    Pi,
    Indefini, // ex: tan(pi/2)

    Var(String),

    Sqrt(Box<Expr>),        // √(x)
    PowInt(Box<Expr>, i64), // x^n (n entier)

    Sin(Box<Expr>),
    Cos(Box<Expr>),
    Tan(Box<Expr>),

    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
}

impl Expr {
    /// Canonicalisation forte (déterminisme structurel).
    /// On garde la canonisation hors de l’AST pour éviter les règles cachées.
    pub fn canon(self) -> Expr {
        canon_expr(self)
    }

    /// Simplification locale (SAFE), sans heuristiques.
    /// Objectif: réduire ce qui est strictement démontrable sans exploser l’arbre.
    pub fn simplify(self) -> Expr {
        use Expr::*;

        match self {
            // Feuilles: aucune simplification à faire
            Rat(_) | Pi | Indefini | Var(_) => self,

            Add(a, b) => {
                let a = a.simplify();
                let b = b.simplify();
                match (&a, &b) {
                    (Indefini, _) | (_, Indefini) => Indefini,
                    (Rat(x), Rat(y)) => Rat(x + y),
                    (Rat(x), _) if x.is_zero() => b,
                    (_, Rat(y)) if y.is_zero() => a,
                    _ => Add(Box::new(a), Box::new(b)),
                }
            }

            Sub(a, b) => {
                let a = a.simplify();
                let b = b.simplify();

                // x - x => 0 (renforce la normalisation)
                if a == b {
                    return Rat(BigRational::zero());
                }

                match (&a, &b) {
                    (Indefini, _) | (_, Indefini) => Indefini,
                    (Rat(x), Rat(y)) => Rat(x - y),
                    (_, Rat(y)) if y.is_zero() => a,
                    (Rat(x), _) if x.is_zero() => {
                        // 0 - b => on garde Sub(0,b) (utile pour signes / rendu / coeff·π)
                        Sub(Box::new(Rat(BigRational::zero())), Box::new(b))
                    }
                    _ => Sub(Box::new(a), Box::new(b)),
                }
            }

            Mul(a, b) => {
                let a = a.simplify();
                let b = b.simplify();

                if matches!(a, Indefini) || matches!(b, Indefini) {
                    return Indefini;
                }

                // √x * √x => x
                if let (Sqrt(x), Sqrt(y)) = (&a, &b) {
                    if x.as_ref() == y.as_ref() {
                        return (*x.clone()).simplify();
                    }
                }

                // √u * √v => √(u*v) si u,v rationnels >= 0
                if let (Sqrt(u), Sqrt(v)) = (&a, &b) {
                    if let (Expr::Rat(ru), Expr::Rat(rv)) = (u.as_ref(), v.as_ref()) {
                        if !ru.is_negative() && !rv.is_negative() {
                            return Expr::Sqrt(Box::new(Expr::Rat(ru.clone() * rv.clone())))
                                .simplify();
                        }
                    }
                }

                // (√x / k) * √x => x / k
                if let (Div(p, q), Sqrt(y)) = (&a, &b) {
                    if let (Sqrt(x), Rat(k)) = (p.as_ref(), q.as_ref()) {
                        if x.as_ref() == y.as_ref() {
                            return Div(
                                Box::new((*x.clone()).simplify()),
                                Box::new(Rat(k.clone())),
                            )
                            .simplify();
                        }
                    }
                }
                // √x * (√x / k) => x / k
                if let (Sqrt(y), Div(p, q)) = (&a, &b) {
                    if let (Sqrt(x), Rat(k)) = (p.as_ref(), q.as_ref()) {
                        if x.as_ref() == y.as_ref() {
                            return Div(
                                Box::new((*x.clone()).simplify()),
                                Box::new(Rat(k.clone())),
                            )
                            .simplify();
                        }
                    }
                }

                // (√x / k) * (√x / m) => x / (k*m)
                if let (Div(p1, q1), Div(p2, q2)) = (&a, &b) {
                    if let (Sqrt(x1), Rat(k)) = (p1.as_ref(), q1.as_ref()) {
                        if let (Sqrt(x2), Rat(m)) = (p2.as_ref(), q2.as_ref()) {
                            if x1.as_ref() == x2.as_ref() {
                                let km = k.clone() * m.clone();
                                return Div(Box::new((*x1.clone()).simplify()), Box::new(Rat(km)))
                                    .simplify();
                            }
                        }
                    }
                }

                match (&a, &b) {
                    (Rat(x), Rat(y)) => Rat(x * y),
                    (Rat(x), _) if x.is_zero() => Rat(BigRational::zero()),
                    (_, Rat(y)) if y.is_zero() => Rat(BigRational::zero()),
                    (Rat(x), _) if x.is_one() => b,
                    (_, Rat(y)) if y.is_one() => a,
                    _ => Mul(Box::new(a), Box::new(b)),
                }
            }

            Div(a, b) => {
                let a = a.simplify();
                let b = b.simplify();

                if matches!(a, Indefini) || matches!(b, Indefini) {
                    return Indefini;
                }

                // division par zéro : on garde symbolique ici (ΣLocal gérera l’erreur)
                if let Expr::Rat(y) = &b {
                    if y.is_zero() {
                        return Div(Box::new(a), Box::new(b));
                    }
                }

                // √x / √x => 1 (si x rationnel non nul)
                if let (Expr::Sqrt(x), Expr::Sqrt(y)) = (&a, &b) {
                    if x.as_ref() == y.as_ref() {
                        if let Expr::Rat(r) = x.as_ref() {
                            if !r.is_zero() {
                                return Expr::Rat(BigRational::one());
                            }
                        }
                    }
                }

                // √u / √v => √(u/v) si u,v rationnels > 0
                if let (Expr::Sqrt(u), Expr::Sqrt(v)) = (&a, &b) {
                    if let (Expr::Rat(ru), Expr::Rat(rv)) = (u.as_ref(), v.as_ref()) {
                        if ru.is_positive() && rv.is_positive() {
                            return Expr::Sqrt(Box::new(Expr::Rat(ru.clone() / rv.clone())))
                                .simplify();
                        }
                    }
                }

                match (&a, &b) {
                    (Rat(x), Rat(y)) => Rat(x / y),
                    (_, Rat(y)) if y.is_one() => a,

                    // (p/q) / √n  => (p/qn) * √n, si n entier > 0
                    (Rat(x), Sqrt(inner)) => {
                        if let Rat(rn) = &**inner {
                            if rn.is_positive() && rn.denom().is_one() {
                                let n = rn.clone(); // entier
                                let x_over_n = x.clone() / n.clone();
                                return Mul(
                                    Box::new(Rat(x_over_n)),
                                    Box::new(Sqrt(Box::new(Rat(n)))),
                                )
                                .simplify();
                            }
                        }
                        Div(Box::new(a), Box::new(b))
                    }

                    _ => Div(Box::new(a), Box::new(b)),
                }
            }

            PowInt(base, n) => {
                let base = base.simplify();
                if matches!(base, Indefini) {
                    return Indefini;
                }
                if n == 0 {
                    return Rat(BigRational::one());
                }
                if let Rat(r) = &base {
                    return Rat(rational_pow_int(r.clone(), n));
                }
                PowInt(Box::new(base), n)
            }

            Sqrt(x) => {
                let x = x.simplify();
                if matches!(x, Indefini) {
                    return Indefini;
                }
                if let Rat(r) = &x {
                    if let Some(s) = rational_sqrt_exact(r) {
                        return Rat(s);
                    }
                }
                Sqrt(Box::new(x))
            }

            Sin(x) => {
                let x = x.simplify();
                if matches!(x, Indefini) {
                    return Indefini;
                }
                Sin(Box::new(x))
            }
            Cos(x) => {
                let x = x.simplify();
                if matches!(x, Indefini) {
                    return Indefini;
                }
                Cos(Box::new(x))
            }
            Tan(x) => {
                let x = x.simplify();
                if matches!(x, Indefini) {
                    return Indefini;
                }
                Tan(Box::new(x))
            }
        }
    }

    /// Détecte un coeff·π (forme simple historique).
    ///
    /// SAFE: Var => None (on ne “devine” rien).
    pub fn as_coeff_pi(&self) -> Option<BigRational> {
        use Expr::*;

        match self {
            Pi => Some(BigRational::one()),

            // Feuilles non-π : pas de coeff·π
            Rat(_) | Indefini | Var(_) => None,

            Mul(a, b) => {
                if let Some(c) = a.as_coeff_pi() {
                    if let Rat(r) = &**b {
                        return Some(c * r.clone());
                    }
                }
                if let Some(c) = b.as_coeff_pi() {
                    if let Rat(r) = &**a {
                        return Some(c * r.clone());
                    }
                }
                None
            }

            Div(a, b) => {
                if let Some(c) = a.as_coeff_pi() {
                    if let Rat(r) = &**b {
                        if r.is_zero() {
                            return None;
                        }
                        return Some(c / r.clone());
                    }
                }
                None
            }

            Sub(a, b) => {
                // Sub(0, x) => -coeff(x)
                if let Rat(r0) = &**a {
                    if r0.is_zero() {
                        if let Some(c) = b.as_coeff_pi() {
                            return Some(-c);
                        }
                    }
                }
                None
            }

            // Add n'est pas géré ici (version simple)
            Add(_, _) => None,

            // IMPORTANT: Var(_) NE DOIT PAS ÊTRE RÉPÉTÉ ICI (sinon unreachable)
            Sqrt(_) | PowInt(_, _) | Sin(_) | Cos(_) | Tan(_) => None,
        }
    }

    /// Variante plus large (itérative), sans flottants.
    /// SAFE: si ça sort du domaine, retourne None.
    pub fn as_coeff_pi_ext(&self) -> Option<BigRational> {
        use Expr::*;

        const MAX_PILE: usize = 8192;
        const MAX_NOEUDS: usize = 200_000;

        #[derive(Copy, Clone)]
        enum Marque<'a> {
            Entrer(&'a Expr),
            Sortir(&'a Expr),
        }

        let mut pile: Vec<Marque<'_>> = Vec::with_capacity(64);
        let mut res: Vec<Option<BigRational>> = Vec::with_capacity(64);

        pile.push(Marque::Entrer(self));

        let mut visites: usize = 0;

        while let Some(m) = pile.pop() {
            visites += 1;
            if visites > MAX_NOEUDS {
                return None;
            }
            if pile.len() > MAX_PILE {
                return None;
            }

            match m {
                Marque::Entrer(e) => {
                    pile.push(Marque::Sortir(e));
                    match e {
                        Add(a, b) | Sub(a, b) | Mul(a, b) | Div(a, b) => {
                            pile.push(Marque::Entrer(b.as_ref()));
                            pile.push(Marque::Entrer(a.as_ref()));
                        }
                        _ => {}
                    }
                }

                Marque::Sortir(e) => match e {
                    Pi => res.push(Some(BigRational::one())),
                    Rat(_) | Indefini | Var(_) => res.push(None),

                    // On refuse de “pousser” coeff·π à travers trig/racines/etc.
                    Sqrt(_) | PowInt(_, _) | Sin(_) | Cos(_) | Tan(_) => res.push(None),

                    Add(_, _) => {
                        let rb = res.pop().unwrap_or(None);
                        let ra = res.pop().unwrap_or(None);
                        match (ra, rb) {
                            (Some(a), Some(b)) => res.push(Some(a + b)),
                            _ => res.push(None),
                        }
                    }

                    Sub(a, _b) => {
                        let rb = res.pop().unwrap_or(None);
                        let ra = res.pop().unwrap_or(None);

                        // Sub(0, x) => -coeff(x)
                        if let Rat(r0) = a.as_ref() {
                            if r0.is_zero() {
                                if let Some(cb) = rb {
                                    res.push(Some(-cb));
                                    continue;
                                }
                            }
                        }

                        match (ra, rb) {
                            (Some(a), Some(b)) => res.push(Some(a - b)),
                            _ => res.push(None),
                        }
                    }

                    Mul(_, _) => {
                        let rb = res.pop().unwrap_or(None);
                        let ra = res.pop().unwrap_or(None);

                        match (ra, rb) {
                            (Some(c), None) => {
                                if let Mul(a, b) = e {
                                    if let Rat(r) = b.as_ref() {
                                        res.push(Some(c * r.clone()));
                                    } else if let Rat(r) = a.as_ref() {
                                        res.push(Some(c * r.clone()));
                                    } else {
                                        res.push(None);
                                    }
                                } else {
                                    res.push(None);
                                }
                            }
                            (None, Some(c)) => {
                                if let Mul(a, b) = e {
                                    if let Rat(r) = a.as_ref() {
                                        res.push(Some(c * r.clone()));
                                    } else if let Rat(r) = b.as_ref() {
                                        res.push(Some(c * r.clone()));
                                    } else {
                                        res.push(None);
                                    }
                                } else {
                                    res.push(None);
                                }
                            }
                            // coeff·π * coeff·π => π² (hors domaine)
                            _ => res.push(None),
                        }
                    }

                    Div(_, _) => {
                        let rb = res.pop().unwrap_or(None);
                        let ra = res.pop().unwrap_or(None);

                        match (ra, rb) {
                            (Some(c), None) => {
                                if let Div(_a, b) = e {
                                    if let Rat(r) = b.as_ref() {
                                        if r.is_zero() {
                                            res.push(None);
                                        } else {
                                            res.push(Some(c / r.clone()));
                                        }
                                    } else {
                                        res.push(None);
                                    }
                                } else {
                                    res.push(None);
                                }
                            }
                            _ => res.push(None),
                        }
                    }
                },
            }
        }

        if res.len() == 1 {
            res.pop().unwrap_or(None)
        } else {
            None
        }
    }
}

/* ------------------------ Modulo rationnel exact (sans flottants) ------------------------ */

/// Réduction modulo `periode` sur un coefficient rationnel (ex: periode=2 pour sin/cos, 1 pour tan).
/// Retourne un rationnel dans [0, periode).
///
/// Si coeff = n/d, alors coeff mod periode = (n mod (periode*d))/d.
///
/// SAFE: si periode invalide ou denom invalide (devrait pas arriver), retourne coeff inchangé.
pub(crate) fn mod_rationnel(coeff: &BigRational, periode: i64) -> BigRational {
    if periode <= 0 {
        return coeff.clone();
    }
    if coeff.is_zero() {
        return BigRational::zero();
    }

    let d = coeff.denom().clone(); // denom > 0 (num_rational)
    if d.is_zero() {
        return coeff.clone();
    }

    let n = coeff.numer().clone();

    let p = BigInt::from(periode);
    let m = &p * &d; // periode*d  (m > 0)

    let r = mod_euclid_bigint(&n, &m);
    BigRational::new(r, d)
}

fn mod_euclid_bigint(a: &BigInt, m: &BigInt) -> BigInt {
    if m.is_zero() {
        return a.clone();
    }
    let mut r = a % m;
    if r.is_negative() {
        r += m;
    }
    r
}

/* ------------------------ Affichage debug (pas “joli” final) ------------------------ */

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Expr::*;
        match self {
            Rat(r) => {
                let n = r.numer();
                let d = r.denom();
                if d.is_one() {
                    write!(f, "{n}")
                } else {
                    write!(f, "{n}/{d}")
                }
            }
            Pi => write!(f, "π"),
            Indefini => write!(f, "indéfini"),
            Var(s) => write!(f, "{s}"),
            Sqrt(x) => write!(f, "√({x})"),
            PowInt(x, n) => write!(f, "({x})^{n}"),
            Sin(x) => write!(f, "sin({x})"),
            Cos(x) => write!(f, "cos({x})"),
            Tan(x) => write!(f, "tan({x})"),
            Add(a, b) => write!(f, "({a}+{b})"),
            Sub(a, b) => write!(f, "({a}-{b})"),
            Mul(a, b) => write!(f, "({a}*{b})"),
            Div(a, b) => write!(f, "({a}/{b})"),
        }
    }
}

/* ------------------------ Outils rationnels (utilisés par simplify) ------------------------ */

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

fn rational_sqrt_exact(r: &BigRational) -> Option<BigRational> {
    if r.is_negative() {
        return None;
    }
    let n = r.numer();
    let d = r.denom();
    let sn = int_sqrt_exact(n)?;
    let sd = int_sqrt_exact(d)?;
    Some(BigRational::new(sn, sd))
}

fn int_sqrt_exact(x: &BigInt) -> Option<BigInt> {
    if x.is_negative() {
        return None;
    }
    let s = int_sqrt_floor(x);
    if &s * &s == *x {
        Some(s)
    } else {
        None
    }
}

fn int_sqrt_floor(x: &BigInt) -> BigInt {
    if x.is_zero() {
        return BigInt::zero();
    }
    if x.is_negative() {
        return BigInt::zero();
    }

    let mut y = approx_sqrt_start(x);
    loop {
        let y_next = (&y + (x / &y)) >> 1;
        if y_next >= y {
            let mut z = y_next;
            while (&z + 1u32) * (&z + 1u32) <= *x {
                z += 1u32;
            }
            while &z * &z > *x {
                z -= 1u32;
            }
            return z;
        }
        y = y_next;
    }
}

fn approx_sqrt_start(x: &BigInt) -> BigInt {
    let bits = x.bits();
    let half = bits.div_ceil(2);
    BigInt::one() << half
}
