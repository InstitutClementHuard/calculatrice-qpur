// src/noyau/canon.rs
//
// Canonicalisation forte (déterministe) :
// - aplatissement Add/Sub et Mul
// - suppression neutres (x+0, x*1, etc.)
// - extraction / remontée du signe (Sub(0,x) comme “-x” canon)
// - regroupement des rationnels
// - tri déterministe des termes/facteurs (ordre total)
// - reconstruction “jolie” : utilise Sub quand le terme suivant est négatif
// - simplif √(n) -> a*√b (extraction des carrés parfaits) pour n entier ≥ 0
//
// Note : on reste volontairement “local” (pas d’identités trig générales ici).

use crate::noyau::expr::Expr;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};
use std::cmp::Ordering;

pub fn canon_expr(e: Expr) -> Expr {
    use Expr::*;

    match e {
        Rat(_) | Pi | Indefini | Var(_) => e,

        Sqrt(x) => canon_sqrt(canon_expr(*x)),
        PowInt(x, n) => canon_pow(canon_expr(*x), n),

        Sin(x) => Sin(Box::new(canon_expr(*x))),
        Cos(x) => Cos(Box::new(canon_expr(*x))),
        Tan(x) => Tan(Box::new(canon_expr(*x))),

        Add(a, b) => canon_addsub(Add(Box::new(canon_expr(*a)), Box::new(canon_expr(*b)))),
        Sub(a, b) => canon_addsub(Sub(Box::new(canon_expr(*a)), Box::new(canon_expr(*b)))),

        Mul(a, b) => canon_mul(Mul(Box::new(canon_expr(*a)), Box::new(canon_expr(*b)))),

        Div(a, b) => canon_div(Div(Box::new(canon_expr(*a)), Box::new(canon_expr(*b)))),
    }
}

/* ------------------------ utilitaires signe ------------------------ */

fn is_zero(e: &Expr) -> bool {
    matches!(e, Expr::Rat(r) if r.is_zero())
}

fn is_one(e: &Expr) -> bool {
    matches!(e, Expr::Rat(r) if r.is_one())
}

/// Renvoie (negatif?, valeur_absolue)
fn split_signe(e: Expr) -> (bool, Expr) {
    use Expr::*;
    match e {
        Rat(r) if r.is_negative() => (true, Rat(-r)),
        Sub(a, b) if is_zero(&a) => (true, *b),
        other => (false, other),
    }
}

fn neg(e: Expr) -> Expr {
    use Expr::*;
    match e {
        Rat(r) => Rat(-r),
        other => Sub(Box::new(Rat(BigRational::zero())), Box::new(other)),
    }
}

/* ------------------------ clef de tri déterministe ------------------------ */

fn rang(e: &Expr) -> u8 {
    use Expr::*;
    match e {
        Rat(_) => 0,
        Var(_) => 1, // ← NOUVEAU
        Sqrt(_) => 2,
        Pi => 3,
        PowInt(_, _) => 4,
        Sin(_) | Cos(_) | Tan(_) => 5,
        Mul(_, _) | Div(_, _) => 6,
        Add(_, _) | Sub(_, _) => 7,
        Indefini => 255,
    }
}

fn key_string(e: &Expr) -> String {
    use Expr::*;
    match e {
        Rat(r) => {
            let n = r.numer().to_string();
            let d = r.denom().to_string();
            format!("R{n}/{d}")
        }
        Var(s) => format!("VAR({s})"),
        Pi => "PI".to_string(),
        Indefini => "INDEF".to_string(),

        Sqrt(x) => format!("SQRT({})", key_string(x)),
        PowInt(x, n) => format!("POW({},{n})", key_string(x)),

        Sin(x) => format!("SIN({})", key_string(x)),
        Cos(x) => format!("COS({})", key_string(x)),
        Tan(x) => format!("TAN({})", key_string(x)),

        Add(a, b) => format!("ADD({},{})", key_string(a), key_string(b)),
        Sub(a, b) => format!("SUB({},{})", key_string(a), key_string(b)),
        Mul(a, b) => format!("MUL({},{})", key_string(a), key_string(b)),
        Div(a, b) => format!("DIV({},{})", key_string(a), key_string(b)),
    }
}

fn cmp_expr(a: &Expr, b: &Expr) -> Ordering {
    let ra = rang(a);
    let rb = rang(b);
    ra.cmp(&rb).then_with(|| key_string(a).cmp(&key_string(b)))
}

/* ------------------------ Add/Sub : aplatissement + tri + reconstruction ------------------------ */

fn collect_addsub(e: Expr, out: &mut Vec<Expr>) {
    use Expr::*;
    match e {
        Add(a, b) => {
            collect_addsub(*a, out);
            collect_addsub(*b, out);
        }
        Sub(a, b) => {
            collect_addsub(*a, out);
            out.push(neg(*b));
        }
        other => out.push(other),
    }
}

fn canon_addsub(e: Expr) -> Expr {
    // On reçoit déjà des sous-termes canonisés (canon_expr).
    let mut termes: Vec<Expr> = Vec::new();
    collect_addsub(e, &mut termes);

    // Retirer les zéros + regrouper les rationnels.
    let mut somme_rat = BigRational::zero();
    let mut v: Vec<Expr> = Vec::with_capacity(termes.len());

    for t in termes {
        match t {
            Expr::Rat(r) => somme_rat += r,
            other => {
                if !is_zero(&other) {
                    v.push(other);
                }
            }
        }
    }

    if !somme_rat.is_zero() {
        v.push(Expr::Rat(somme_rat));
    }

    if v.is_empty() {
        return Expr::Rat(BigRational::zero());
    }

    // Tri déterministe
    v.sort_by(cmp_expr);

    // Reconstruction “jolie” : si le terme suivant est négatif, on utilise Sub(acc, abs).
    let mut acc = v[0].clone();
    for t in v.into_iter().skip(1) {
        let (negatif, abs) = split_signe(t);
        if negatif {
            acc = Expr::Sub(Box::new(acc), Box::new(abs));
        } else {
            acc = Expr::Add(Box::new(acc), Box::new(abs));
        }
    }
    acc
}

/* ------------------------ Mul : aplatissement + signe + tri + reconstruction ------------------------ */

fn collect_mul(e: Expr, out: &mut Vec<Expr>) {
    use Expr::*;
    match e {
        Mul(a, b) => {
            collect_mul(*a, out);
            collect_mul(*b, out);
        }
        other => out.push(other),
    }
}

fn canon_mul(e: Expr) -> Expr {
    use Expr::*;

    let mut facteurs: Vec<Expr> = Vec::new();
    collect_mul(e, &mut facteurs);

    // Si un facteur est indéfini → indéfini.
    if facteurs.iter().any(|x| matches!(x, Indefini)) {
        return Indefini;
    }

    // Extraire signe global, regrouper rationnels, retirer *1, court-circuit *0.
    let mut signe_neg = false;
    let mut prod_rat = BigRational::one();
    let mut v: Vec<Expr> = Vec::with_capacity(facteurs.len());

    for f in facteurs {
        // 0 * ... = 0
        if is_zero(&f) {
            return Rat(BigRational::zero());
        }

        let (neg_f, abs_f) = split_signe(f);
        if neg_f {
            signe_neg = !signe_neg;
        }

        match abs_f {
            Rat(r) => {
                // r peut être 0 déjà traité ; ici r != 0
                if r.is_one() {
                    // ignore
                } else {
                    prod_rat *= r;
                }
            }
            other => {
                if !is_one(&other) {
                    v.push(other);
                }
            }
        }
    }

    // Appliquer prod_rat
    if prod_rat.is_zero() {
        return Rat(BigRational::zero());
    }

    // Pousser le signe dans prod_rat si possible
    if signe_neg {
        prod_rat = -prod_rat;
    }

    // Si prod_rat == 1 et pas d’autres facteurs : 1
    // Si prod_rat != 1 : on le garde comme facteur.
    if !prod_rat.is_one() || v.is_empty() {
        v.push(Rat(prod_rat));
    }

    // Nettoyage (au cas où)
    v.retain(|x| !is_one(x));

    if v.is_empty() {
        return Rat(BigRational::one());
    }
    if v.len() == 1 {
        return v.pop().unwrap();
    }

    // Tri déterministe des facteurs
    v.sort_by(cmp_expr);

    // Reconstruction left-assoc
    let mut acc = v[0].clone();
    for f in v.into_iter().skip(1) {
        acc = Mul(Box::new(acc), Box::new(f));
    }
    acc
}

/* ------------------------ Div : signe + cas simples ------------------------ */

fn canon_div(e: Expr) -> Expr {
    use Expr::*;

    let Div(a, b) = e else { return e };

    if matches!(a.as_ref(), Indefini) || matches!(b.as_ref(), Indefini) {
        return Indefini;
    }

    // a/1 => a
    if is_one(&b) {
        return *a;
    }

    // 0/b => 0 (même si b=0, on reste symbolique ailleurs ; ici on garde 0)
    if is_zero(&a) {
        return Rat(BigRational::zero());
    }

    // Remonter le signe du dénominateur : a/(-b) => -(a/b)
    let (neg_b, abs_b) = split_signe(*b);
    let mut num = *a;
    let den = abs_b;

    if neg_b {
        num = neg(num);
    }

    // Tri léger : si num et den ont des canonisations internes, elles sont déjà faites.
    Div(Box::new(num), Box::new(den))
}

/* ------------------------ PowInt / Sqrt ------------------------ */

fn canon_pow(base: Expr, n: i64) -> Expr {
    use Expr::*;
    if matches!(base, Indefini) {
        return Indefini;
    }
    if n == 0 {
        return Rat(BigRational::one());
    }
    PowInt(Box::new(base), n)
}

fn canon_sqrt(x: Expr) -> Expr {
    use Expr::*;

    if matches!(x, Indefini) {
        return Indefini;
    }

    // √(rat) : si entier >= 0, on extrait les carrés parfaits : √(s²*t) = s*√t
    if let Rat(r) = &x {
        if r.is_zero() {
            return Rat(BigRational::zero());
        }
        if r.is_positive() && r.denom().is_one() {
            let n = r.numer().clone();
            let (s, t) = extrait_carre_parfait(&n);
            if t.is_one() {
                return Rat(BigRational::from_integer(s));
            }
            if !s.is_one() {
                return canon_mul(Mul(
                    Box::new(Rat(BigRational::from_integer(s))),
                    Box::new(Sqrt(Box::new(Rat(BigRational::from_integer(t))))),
                ));
            }
            return Sqrt(Box::new(Rat(BigRational::from_integer(t))));
        }
    }

    Sqrt(Box::new(x))
}

/// Décompose n >= 0 en n = s^2 * t, avec t “sans facteur carré” (approx. par essais).
fn extrait_carre_parfait(n: &BigInt) -> (BigInt, BigInt) {
    if n.is_zero() {
        return (BigInt::zero(), BigInt::zero());
    }
    if n.is_one() {
        return (BigInt::one(), BigInt::one());
    }

    let mut reste = n.clone();
    let mut s = BigInt::one();

    // Essai par p = 2 puis impairs. Suffisant pour nos petits entiers (cas √2, √3, √12, √75, etc.)
    let mut p = BigInt::from(2);
    while &p * &p <= reste {
        let p2 = &p * &p;

        while (&reste % &p2).is_zero() {
            reste /= &p2;
            s *= &p;
        }

        if p == BigInt::from(2) {
            p = BigInt::from(3);
        } else {
            p += 2;
        }
    }

    (s, reste)
}
