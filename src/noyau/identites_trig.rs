// src/noyau/identites_trig.rs
//
// Identités trigonométriques exactes — version SAFE (anti-boucle)
//
// Objectifs :
// - Règles toujours sûres (réduisent / normalisent sans exploser)
// - Garde-fous anti-boucle via score (noeuds, profondeur) + passes bornées
// - Zéro flottants, zéro heuristique “magique”
//
// Règles incluses :
// B1 Parité (via Sub(0,x))
//   sin(0-x) -> 0 - sin(x)
//   cos(0-x) -> cos(x)
//   tan(0-x) -> 0 - tan(x)
// B2 Décalage par ±π
//   sin(x±π) -> 0 - sin(x)
//   cos(x±π) -> 0 - cos(x)
//   tan(x±π) -> tan(x)
// B3 Pythagoricienne (forme stricte)
//   sin(x)^2 + cos(x)^2 -> 1
// B4 Périodicité (forme stricte)
//   sin(x±2π) -> sin(x)
//   cos(x±2π) -> cos(x)
//   tan(x±π)  -> tan(x)
// B5 Décalage par ±π/2 (forme stricte)
//   sin(x + π/2) -> cos(x)
//   sin(x - π/2) -> 0 - cos(x)
//   cos(x + π/2) -> 0 - sin(x)
//   cos(x - π/2) -> sin(x)
//   tan(x ± π/2) -> indéfini
// B6 Symétrie (forme stricte)
//   sin(π - x) -> sin(x)
//   cos(π - x) -> 0 - cos(x)
// BONUS (safe) : (sin(x)/cos(x)) -> tan(x) si ça réduit le score
//
// IMPORTANT : on N’EXPAND PAS tan(x) -> sin/cos (risque de boucles / indéfinis).
// IMPORTANT : B7 (développement) est volontairement évité : ça GROSSIT l’arbre.
//

use crate::noyau::expr::Expr;
use num_rational::BigRational;
use num_traits::{One, Zero};

pub fn trig_identites(e: Expr) -> Expr {
    // Passes bornées : on réécrit tant que ça n’empire pas le score
    // (et comme nos règles ne sont pas inversibles, score égal est safe et utile).
    let mut cur = e;
    let mut cur_score = score(&cur);

    // 6 passes max : suffisant (B1..B6 + div->tan)
    for _ in 0..6 {
        let next = rewrite_once(cur.clone());
        let next_score = score(&next);

        if next == cur {
            break;
        }

        // Garde-fou : accepter si score DIMINUE ou RESTE ÉGAL.
        if next_score <= cur_score {
            cur = next;
            cur_score = next_score;
        } else {
            break;
        }
    }

    cur
}

/* ------------------------ réécriture : 1 passe ------------------------ */

fn rewrite_once(e: Expr) -> Expr {
    use Expr::*;

    match e {
        Expr::Rat(_) | Expr::Pi | Expr::Indefini | Expr::Var(_) => e,

        // --- trig noeud courant + descente ---
        Sin(x) => {
            let x = rewrite_once(*x);
            match x.clone() {
                // B1: sin(0 - t) => 0 - sin(t)
                Sub(a, b) if is_zero(&a) => neg(Sin(Box::new(*b))),

                // B2: sin(t ± π) => 0 - sin(t)  (deux ordres)
                Add(a, b) if is_pi(&b) => neg(Sin(Box::new(*a))),
                Add(a, b) if is_pi(&a) => neg(Sin(Box::new(*b))),
                Sub(a, b) if is_pi(&b) => neg(Sin(Box::new(*a))),

                // B4: sin(t ± 2π) => sin(t) (deux ordres sur Add)
                Add(a, b) if is_two_pi(&b) => Sin(Box::new(*a)),
                Add(a, b) if is_two_pi(&a) => Sin(Box::new(*b)),
                Sub(a, b) if is_two_pi(&b) => Sin(Box::new(*a)),

                // B5: sin(t ± π/2) => ±cos(t) (deux ordres sur Add)
                Add(a, b) if is_pi_sur_2(&b) => Cos(Box::new(*a)),
                Add(a, b) if is_pi_sur_2(&a) => Cos(Box::new(*b)),
                Sub(a, b) if is_pi_sur_2(&b) => neg(Cos(Box::new(*a))),

                // B6: sin(π - t) => sin(t) (strict)
                Sub(a, b) if is_pi(&a) => Sin(Box::new(*b)),

                _ => Sin(Box::new(x)),
            }
        }

        Cos(x) => {
            let x = rewrite_once(*x);
            match x.clone() {
                // B1: cos(0 - t) => cos(t)
                Sub(a, b) if is_zero(&a) => Cos(Box::new(*b)),

                // B2: cos(t ± π) => 0 - cos(t) (deux ordres)
                Add(a, b) if is_pi(&b) => neg(Cos(Box::new(*a))),
                Add(a, b) if is_pi(&a) => neg(Cos(Box::new(*b))),
                Sub(a, b) if is_pi(&b) => neg(Cos(Box::new(*a))),

                // B4: cos(t ± 2π) => cos(t) (deux ordres sur Add)
                Add(a, b) if is_two_pi(&b) => Cos(Box::new(*a)),
                Add(a, b) if is_two_pi(&a) => Cos(Box::new(*b)),
                Sub(a, b) if is_two_pi(&b) => Cos(Box::new(*a)),

                // B5: cos(t ± π/2) => ∓sin(t) (deux ordres sur Add)
                Add(a, b) if is_pi_sur_2(&b) => neg(Sin(Box::new(*a))),
                Add(a, b) if is_pi_sur_2(&a) => neg(Sin(Box::new(*b))),
                Sub(a, b) if is_pi_sur_2(&b) => Sin(Box::new(*a)),

                // B6: cos(π - t) => 0 - cos(t) (strict)
                Sub(a, b) if is_pi(&a) => neg(Cos(Box::new(*b))),

                _ => Cos(Box::new(x)),
            }
        }

        Tan(x) => {
            let x = rewrite_once(*x);
            match x.clone() {
                // B1: tan(0 - t) => 0 - tan(t)
                Sub(a, b) if is_zero(&a) => neg(Tan(Box::new(*b))),

                // B2/B4: tan(t ± π) => tan(t) (deux ordres sur Add)
                Add(a, b) if is_pi_expr(&b) => Tan(Box::new(*a)),
                Add(a, b) if is_pi_expr(&a) => Tan(Box::new(*b)),
                Sub(a, b) if is_pi_expr(&b) => Tan(Box::new(*a)),

                // B5: tan(t ± π/2) => indéfini (cos(...)=0)
                Add(a, b) if is_pi_sur_2(&b) => {
                    let _ = a;
                    Indefini
                }
                Add(a, b) if is_pi_sur_2(&a) => {
                    let _ = b;
                    Indefini
                }
                Sub(a, b) if is_pi_sur_2(&b) => {
                    let _ = a;
                    Indefini
                }

                _ => Tan(Box::new(x)),
            }
        }

        // --- sqrt / pow : descente ---
        Sqrt(x) => Sqrt(Box::new(rewrite_once(*x))),
        PowInt(x, n) => PowInt(Box::new(rewrite_once(*x)), n),

        // --- binaires : descente puis règles structurales ---
        Add(a, b) => {
            let a = rewrite_once(*a);
            let b = rewrite_once(*b);

            // B3: sin(x)^2 + cos(x)^2 -> 1
            if let Some(one) = pythagore(&a, &b) {
                return one;
            }
            if let Some(one) = pythagore(&b, &a) {
                return one;
            }

            Add(Box::new(a), Box::new(b))
        }

        Sub(a, b) => Sub(Box::new(rewrite_once(*a)), Box::new(rewrite_once(*b))),

        Mul(a, b) => Mul(Box::new(rewrite_once(*a)), Box::new(rewrite_once(*b))),

        Div(a, b) => {
            let a2 = rewrite_once(*a);
            let b2 = rewrite_once(*b);

            // BONUS (safe): sin(x)/cos(x) -> tan(x) si x identique et score réduit
            if let (Expr::Sin(x1), Expr::Cos(x2)) = (&a2, &b2) {
                if x1.as_ref() == x2.as_ref() {
                    let cand = Expr::Tan(Box::new((**x1).clone()));
                    let cur_div = Expr::Div(Box::new(a2.clone()), Box::new(b2.clone()));
                    if score(&cand) < score(&cur_div) {
                        return cand;
                    }
                }
            }

            Div(Box::new(a2), Box::new(b2))
        }
    }
}

/* ------------------------ pythagore strict ------------------------ */

fn pythagore(a: &Expr, b: &Expr) -> Option<Expr> {
    // sin(x)^2 + cos(x)^2 -> 1
    // Forme stricte : PowInt(Sin(x),2) et PowInt(Cos(x),2) avec même x
    match (a, b) {
        (Expr::PowInt(sa, 2), Expr::PowInt(cb, 2)) => match (sa.as_ref(), cb.as_ref()) {
            (Expr::Sin(x1), Expr::Cos(x2)) if x1.as_ref() == x2.as_ref() => {
                Some(Expr::Rat(BigRational::one()))
            }
            _ => None,
        },
        _ => None,
    }
}

/* ------------------------ score anti-boucle ------------------------ */

fn score(e: &Expr) -> (usize, usize) {
    // (noeuds, profondeur)
    fn walk(e: &Expr) -> (usize, usize) {
        use Expr::*;
        match e {
            Rat(_) | Pi | Indefini | Var(_) => (1, 1),

            Sqrt(x) | Sin(x) | Cos(x) | Tan(x) => {
                let (n, d) = walk(x);
                (n + 1, d + 1)
            }

            PowInt(x, _) => {
                let (n, d) = walk(x);
                (n + 1, d + 1)
            }

            Add(a, b) | Sub(a, b) | Mul(a, b) | Div(a, b) => {
                let (na, da) = walk(a);
                let (nb, db) = walk(b);
                (na + nb + 1, 1 + da.max(db))
            }
        }
    }
    walk(e)
}

/* ------------------------ helpers ------------------------ */

fn is_zero(e: &Expr) -> bool {
    matches!(e, Expr::Rat(r) if r.is_zero())
}

fn is_pi(e: &Expr) -> bool {
    matches!(e, Expr::Pi)
}

fn neg(e: Expr) -> Expr {
    // 0 - e
    Expr::Sub(Box::new(Expr::Rat(BigRational::zero())), Box::new(e))
}

/* --- helpers structurels (formes strictes) --- */

fn is_rat_i(e: &Expr, i: i64) -> bool {
    match e {
        Expr::Rat(r) => r == &BigRational::from_integer(i.into()),
        _ => false,
    }
}

// Détecte exactement 2π : Mul(Rat(2), Pi) ou Mul(Pi, Rat(2)).
fn is_two_pi(e: &Expr) -> bool {
    use Expr::*;
    match e {
        Mul(a, b) => (is_rat_i(a, 2) && is_pi(b)) || (is_pi(a) && is_rat_i(b, 2)),
        _ => false,
    }
}

// Détecte exactement π (forme stricte) : ici, seulement Pi.
// (Nom gardé pour lisibilité et extension future.)
fn is_pi_expr(e: &Expr) -> bool {
    is_pi(e)
}

// Détecte exactement π/2 : Div(Pi, Rat(2)) (forme que ton parseur produit).
fn is_pi_sur_2(e: &Expr) -> bool {
    use Expr::*;
    match e {
        Div(a, b) => is_pi(a) && is_rat_i(b, 2),
        _ => false,
    }
}

/* ------------------------ tests ------------------------ */

#[cfg(test)]
mod tests {
    use super::trig_identites;
    use crate::noyau::expr::Expr;
    use num_rational::BigRational;
    use num_traits::{One, Zero};

    fn rat_i(i: i64) -> Expr {
        Expr::Rat(BigRational::from_integer(i.into()))
    }
    fn zero() -> Expr {
        rat_i(0)
    }
    fn canon_strict(e: Expr) -> Expr {
        // identités -> simplify -> canon (comparaison propre)
        e.simplify().canon()
    }

    #[test]
    fn b1_parite_sin() {
        // sin(0 - pi/6) -> 0 - sin(pi/6)
        let e = Expr::Sin(Box::new(Expr::Sub(
            Box::new(zero()),
            Box::new(Expr::Div(Box::new(Expr::Pi), Box::new(rat_i(6)))),
        )));

        let out = trig_identites(e).simplify().canon();

        match out {
            Expr::Sub(a, b) => {
                assert!(matches!(*a, Expr::Rat(r) if r.is_zero()));
                assert!(matches!(*b, Expr::Sin(_)));
            }
            _ => panic!("attendu Sub(0, Sin(...)), obtenu: {out:?}"),
        }
    }

    #[test]
    fn b1_parite_cos() {
        // cos(0 - pi/3) -> cos(pi/3)
        let e = Expr::Cos(Box::new(Expr::Sub(
            Box::new(zero()),
            Box::new(Expr::Div(Box::new(Expr::Pi), Box::new(rat_i(3)))),
        )));

        let out = trig_identites(e).simplify().canon();

        match out {
            Expr::Cos(_) => {}
            _ => panic!("attendu Cos(...), obtenu: {out:?}"),
        }
    }

    #[test]
    fn b2_decalage_pi_sin() {
        // sin(pi/7 + pi) -> 0 - sin(pi/7)
        let pi_sur_7 = Expr::Div(Box::new(Expr::Pi), Box::new(rat_i(7)));
        let e = Expr::Sin(Box::new(Expr::Add(Box::new(pi_sur_7), Box::new(Expr::Pi))));

        let out = trig_identites(e).simplify().canon();

        match out {
            Expr::Sub(a, b) => {
                assert!(matches!(*a, Expr::Rat(r) if r.is_zero()));
                assert!(matches!(*b, Expr::Sin(_)));
            }
            _ => panic!("attendu Sub(0, Sin(pi/7)), obtenu: {out:?}"),
        }
    }

    #[test]
    fn b3_pythagore_strict() {
        // sin(x)^2 + cos(x)^2 -> 1, avec x = pi/5
        let x = Expr::Div(Box::new(Expr::Pi), Box::new(rat_i(5)));

        let sin2 = Expr::PowInt(Box::new(Expr::Sin(Box::new(x.clone()))), 2);
        let cos2 = Expr::PowInt(Box::new(Expr::Cos(Box::new(x.clone()))), 2);

        let e = Expr::Add(Box::new(sin2), Box::new(cos2));

        let out = canon_strict(trig_identites(e));

        assert!(
            matches!(out, Expr::Rat(ref r) if r.is_one()),
            "attendu 1, obtenu: {out:?}"
        );
    }

    #[test]
    fn bonus_sin_sur_cos_vers_tan() {
        // sin(pi/4)/cos(pi/4) -> tan(pi/4)
        let x = Expr::Div(Box::new(Expr::Pi), Box::new(rat_i(4)));

        let e = Expr::Div(
            Box::new(Expr::Sin(Box::new(x.clone()))),
            Box::new(Expr::Cos(Box::new(x))),
        );

        let out = trig_identites(e).simplify().canon();

        match out {
            Expr::Tan(_) => {}
            _ => panic!("attendu Tan(...), obtenu: {out:?}"),
        }
    }

    // -------------------------
    // B4 — périodicité (strict)
    // -------------------------

    #[test]
    fn b4_periodicite_sin_2pi() {
        // sin(x + 2π) -> sin(x)
        let x = Expr::Div(Box::new(Expr::Pi), Box::new(rat_i(7)));
        let two_pi = Expr::Mul(Box::new(rat_i(2)), Box::new(Expr::Pi));
        let e = Expr::Sin(Box::new(Expr::Add(Box::new(x.clone()), Box::new(two_pi))));
        let out = canon_strict(trig_identites(e));
        assert!(
            matches!(out, Expr::Sin(_)),
            "attendu Sin(...), obtenu: {out:?}"
        );
    }

    #[test]
    fn b4_periodicite_cos_2pi() {
        // cos(x - 2π) -> cos(x)
        let x = Expr::Div(Box::new(Expr::Pi), Box::new(rat_i(5)));
        let two_pi = Expr::Mul(Box::new(rat_i(2)), Box::new(Expr::Pi));
        let e = Expr::Cos(Box::new(Expr::Sub(Box::new(x.clone()), Box::new(two_pi))));
        let out = canon_strict(trig_identites(e));
        assert!(
            matches!(out, Expr::Cos(_)),
            "attendu Cos(...), obtenu: {out:?}"
        );
    }

    #[test]
    fn b4_periodicite_tan_pi() {
        // tan(x + π) -> tan(x)
        let x = Expr::Div(Box::new(Expr::Pi), Box::new(rat_i(9)));
        let e = Expr::Tan(Box::new(Expr::Add(Box::new(x.clone()), Box::new(Expr::Pi))));
        let out = canon_strict(trig_identites(e));
        assert!(
            matches!(out, Expr::Tan(_)),
            "attendu Tan(...), obtenu: {out:?}"
        );
    }

    // -------------------------
    // B5 — ±π/2 (strict)
    // -------------------------

    #[test]
    fn b5_decalage_sin_pi_sur_2() {
        // sin(x + π/2) -> cos(x)
        let x = Expr::Div(Box::new(Expr::Pi), Box::new(rat_i(7)));
        let pi2 = Expr::Div(Box::new(Expr::Pi), Box::new(rat_i(2)));
        let e = Expr::Sin(Box::new(Expr::Add(Box::new(x.clone()), Box::new(pi2))));
        let out = canon_strict(trig_identites(e));
        assert!(
            matches!(out, Expr::Cos(_)),
            "attendu Cos(...), obtenu: {out:?}"
        );
    }

    #[test]
    fn b5_decalage_cos_pi_sur_2() {
        // cos(x + π/2) -> 0 - sin(x)
        let x = Expr::Div(Box::new(Expr::Pi), Box::new(rat_i(5)));
        let pi2 = Expr::Div(Box::new(Expr::Pi), Box::new(rat_i(2)));
        let e = Expr::Cos(Box::new(Expr::Add(Box::new(x.clone()), Box::new(pi2))));
        let out = canon_strict(trig_identites(e));
        match out {
            Expr::Sub(a, b) => {
                assert!(matches!(*a, Expr::Rat(r) if r.is_zero()));
                assert!(matches!(*b, Expr::Sin(_)));
            }
            _ => panic!("attendu Sub(0, Sin(...)), obtenu: {out:?}"),
        }
    }

    #[test]
    fn b5_tan_pi_sur_2_indefini() {
        // tan(x + π/2) -> indéfini
        let x = Expr::Div(Box::new(Expr::Pi), Box::new(rat_i(9)));
        let pi2 = Expr::Div(Box::new(Expr::Pi), Box::new(rat_i(2)));
        let e = Expr::Tan(Box::new(Expr::Add(Box::new(x.clone()), Box::new(pi2))));
        let out = canon_strict(trig_identites(e));
        assert!(
            matches!(out, Expr::Indefini),
            "attendu Indefini, obtenu: {out:?}"
        );
    }

    // -------------------------
    // B6 — π - x (strict)
    // -------------------------

    #[test]
    fn b6_sin_pi_moins_x() {
        // sin(π - x) -> sin(x)
        let x = Expr::Div(Box::new(Expr::Pi), Box::new(rat_i(7)));
        let e = Expr::Sin(Box::new(Expr::Sub(Box::new(Expr::Pi), Box::new(x))));
        let out = canon_strict(trig_identites(e));
        assert!(
            matches!(out, Expr::Sin(_)),
            "attendu Sin(...), obtenu: {out:?}"
        );
    }

    #[test]
    fn b6_cos_pi_moins_x() {
        // cos(π - x) -> 0 - cos(x)
        let x = Expr::Div(Box::new(Expr::Pi), Box::new(rat_i(7)));
        let e = Expr::Cos(Box::new(Expr::Sub(Box::new(Expr::Pi), Box::new(x))));
        let out = canon_strict(trig_identites(e));
        match out {
            Expr::Sub(a, b) => {
                assert!(matches!(*a, Expr::Rat(r) if r.is_zero()));
                assert!(matches!(*b, Expr::Cos(_)));
            }
            _ => panic!("attendu Sub(0, Cos(...)), obtenu: {out:?}"),
        }
    }
}
