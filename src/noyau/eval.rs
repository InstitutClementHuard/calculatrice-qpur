//! Noyau — évaluation (pipeline réel)
//!
//! tokenize -> RPN -> Expr -> simplify -> trig spéciale (récursive)
//!        -> re-simplify -> identités trig (SAFE) -> re-simplify -> canon -> EXACT -> ΣLocal
//!
//! Remarque : trig spéciale est appliquée ici (pas encore dans Expr::simplify),
//! pour garder la “preuve” hors de l’AST.

use super::expr::Expr;
use super::format::{format_exact_final, format_expr_pretty};
use super::identites_trig::trig_identites;
use super::jetons::{format_tokens, tokenize};
use super::lecture::{eval_scaled, scaled_to_decimal};
use super::rpn::{from_rpn, to_rpn};
// trig_special + preuve
use super::trig::{trig_special, TrigFn, TrigOutcome};

#[derive(Default, Clone, Debug)]
pub struct DemarcheNoyau {
    pub jetons: String,
    pub rpn: String,
    pub avant: String,
    pub apres: String,
    pub note: String,
    pub preuve: String,
}

/// API publique : évalue une expression et retourne:
/// - EXACT (forme finie)
/// - ΣLocal (lecture décimale tronquée) : None si indéfini OU si variable
/// - Démarche (jetons, rpn, avant/après, preuve)
pub fn eval_expression(
    expr_str: &str,
    digits: usize,
) -> Result<(String, Option<String>, DemarcheNoyau), String> {
    let s = expr_str.trim();
    if s.is_empty() {
        return Err("Entrée vide".into());
    }

    // 1) Jetons
    let jetons = tokenize(s)?;
    let jetons_txt = format_tokens(&jetons);

    // 2) RPN
    let rpn = to_rpn(&jetons)?;
    let rpn_txt = format_tokens(&rpn);

    // 3) AST (Expr)
    let expr0 = from_rpn(&rpn)?;

    // 4) Simplification de base
    let expr_s0 = expr0.clone().simplify();

    // 5) Trig spéciale (récursive) : remplace sin/cos/tan dès que possible + accumule preuve
    //    OPTI: preuve mut (zéro concat lourde, pas de String retournée en cascade)
    let mut preuve = String::new();
    let expr_s1 = applique_trig_speciale(&expr_s0, &mut preuve);

    // 5b) Re-simplify (important : après remplacements trig)
    let expr_s = expr_s1.simplify();

    // 5c) Identités trig (SAFE) puis re-simplify (important : nettoie Sub(0,·), etc.)
    let expr_b = trig_identites(expr_s).simplify();

    // 5d) Canon
    let expr_c = expr_b.canon();

    // 6) EXACT final (sur la forme canon)
    let exact = format_exact_final(&expr_c);

    // 7) ΣLocal (bloquée si indéfini OU si variable) (sur la forme canon)
    let lecture = match &expr_c {
        Expr::Indefini => None,
        _ if contient_var(&expr_c) => None,
        _ => {
            let scaled = eval_scaled(&expr_c, digits)?;
            Some(scaled_to_decimal(scaled, digits))
        }
    };

    // 8) Démarche
    let d = DemarcheNoyau {
        jetons: jetons_txt,
        rpn: rpn_txt,
        avant: format_expr_pretty(&expr0),
        apres: format_expr_pretty(&expr_c), // reflète la forme finale (identités + canon)
        note: "Pipeline: jetons → RPN → Expr → simplify → trig spéciale → re-simplify → identités trig → re-simplify → canon → EXACT → ΣLocal.".into(),
        preuve,
    };

    Ok((exact, lecture, d))
}

/// Détecte si une expression contient au moins une variable.
/// Itératif + garde-fous : si l'arbre est trop gros, on retourne true (SAFE => bloque ΣLocal).
fn contient_var(expr: &Expr) -> bool {
    use Expr::*;

    const MAX_PILE: usize = 8192;
    const MAX_NOEUDS: usize = 200_000;

    let mut pile: Vec<&Expr> = Vec::with_capacity(64);
    pile.push(expr);

    let mut visites: usize = 0;

    while let Some(e) = pile.pop() {
        visites += 1;
        if visites > MAX_NOEUDS || pile.len() > MAX_PILE {
            // garde-fou : si c’est trop gros, on “assume” variable possible => on bloque ΣLocal
            return true;
        }

        match e {
            Var(_) => return true,

            Rat(_) | Pi | Indefini => {}

            Sqrt(x) | Sin(x) | Cos(x) | Tan(x) => pile.push(x.as_ref()),

            PowInt(x, _) => pile.push(x.as_ref()),

            Add(a, b) | Sub(a, b) | Mul(a, b) | Div(a, b) => {
                pile.push(a.as_ref());
                pile.push(b.as_ref());
            }
        }
    }

    false
}

/// Trig spéciale récursive : applique trig_special PARTOUT dans l’arbre.
/// Accumule la preuve (une ligne par match trig réussi) dans `preuve`.
fn applique_trig_speciale(expr: &Expr, preuve: &mut String) -> Expr {
    use Expr::*;

    fn push_preuve(preuve: &mut String, ligne: &str) {
        if ligne.is_empty() {
            return;
        }
        if !preuve.is_empty() {
            preuve.push('\n');
        }
        preuve.push_str(ligne);
    }

    let out = match expr {
        // --- trig au noeud courant ---
        Sin(x) => match trig_special(x, TrigFn::Sin) {
            Some(TrigOutcome::Valeur(v, p)) => {
                push_preuve(preuve, &p);
                v
            }
            Some(TrigOutcome::Indefini(p)) => {
                push_preuve(preuve, &p);
                Indefini
            }
            None => {
                let xx = applique_trig_speciale(x, preuve);
                Sin(Box::new(xx))
            }
        },

        Cos(x) => match trig_special(x, TrigFn::Cos) {
            Some(TrigOutcome::Valeur(v, p)) => {
                push_preuve(preuve, &p);
                v
            }
            Some(TrigOutcome::Indefini(p)) => {
                push_preuve(preuve, &p);
                Indefini
            }
            None => {
                let xx = applique_trig_speciale(x, preuve);
                Cos(Box::new(xx))
            }
        },

        Tan(x) => match trig_special(x, TrigFn::Tan) {
            Some(TrigOutcome::Valeur(v, p)) => {
                push_preuve(preuve, &p);
                v
            }
            Some(TrigOutcome::Indefini(p)) => {
                push_preuve(preuve, &p);
                Indefini
            }
            None => {
                let xx = applique_trig_speciale(x, preuve);
                Tan(Box::new(xx))
            }
        },

        // --- descente structurée ---
        Add(a, b) => {
            let aa = applique_trig_speciale(a, preuve);
            let bb = applique_trig_speciale(b, preuve);
            Add(Box::new(aa), Box::new(bb))
        }
        Sub(a, b) => {
            let aa = applique_trig_speciale(a, preuve);
            let bb = applique_trig_speciale(b, preuve);
            Sub(Box::new(aa), Box::new(bb))
        }
        Mul(a, b) => {
            let aa = applique_trig_speciale(a, preuve);
            let bb = applique_trig_speciale(b, preuve);
            Mul(Box::new(aa), Box::new(bb))
        }
        Div(a, b) => {
            let aa = applique_trig_speciale(a, preuve);
            let bb = applique_trig_speciale(b, preuve);
            Div(Box::new(aa), Box::new(bb))
        }

        // --- autres noeuds utiles (pour ne pas “bloquer” la trig dans l’arbre) ---
        Sqrt(x) => {
            let xx = applique_trig_speciale(x, preuve);
            Sqrt(Box::new(xx))
        }
        PowInt(x, n) => {
            let xx = applique_trig_speciale(x, preuve);
            PowInt(Box::new(xx), *n)
        }

        // --- feuilles ---
        Rat(_) | Pi | Indefini | Var(_) => expr.clone(),
    };

    // Un seul simplify à la fin.
    out.simplify()
}

#[cfg(test)]
mod tests {
    use super::eval_expression;

    fn ok_exact(s: &str, digits: usize) -> (String, Option<String>) {
        let (exact, lecture_opt, _d) = eval_expression(s, digits)
            .unwrap_or_else(|e| panic!("eval_expression({s:?}) erreur: {e}"));
        (exact, lecture_opt)
    }

    fn ok_exact_only(s: &str) -> String {
        ok_exact(s, 50).0
    }

    fn ok_dec(s: &str, digits: usize) -> String {
        ok_exact(s, digits)
            .1
            .unwrap_or_else(|| panic!("ΣLocal indisponible pour {s:?}"))
    }

    fn assert_contains(hay: &str, needle: &str) {
        if !hay.contains(needle) {
            panic!("attendu que {hay:?} contienne {needle:?}");
        }
    }

    fn assert_eq_trim(a: &str, b: &str) {
        let aa = a.trim();
        let bb = b.trim();
        if aa != bb {
            panic!("diff:\nA={aa:?}\nB={bb:?}");
        }
    }

    // --- Variables ---

    #[test]
    fn var_parse_et_affiche() {
        let (exact, lecture_opt, _d) = eval_expression("x + 1/2", 20).unwrap();
        // EXACT doit contenir x
        assert!(exact.contains("x"));
        // ΣLocal doit être bloquée (pas évaluable sans valeur pour x)
        assert!(lecture_opt.is_none());
    }

    // --- Trig de base ---

    #[test]
    fn trig_sin_pi_4() {
        let exact = ok_exact_only("sin(pi/4)");
        assert_contains(&exact, "√2");
        assert_contains(&exact, "/2");

        let dec = ok_dec("sin(pi/4)", 20);
        assert_contains(&dec, "0.707106781186547524");
    }

    #[test]
    fn trig_tan_pi_6() {
        let exact = ok_exact_only("tan(pi/6)");
        assert_contains(&exact, "√3");
        assert_contains(&exact, "/3");

        let dec = ok_dec("tan(pi/6)", 20);
        assert_contains(&dec, "0.577350269189625764");
    }

    #[test]
    fn trig_tan_pi_2_indefini() {
        let (exact, lecture_opt) = ok_exact("tan(pi/2)", 20);
        assert_eq_trim(&exact, "indéfini");
        assert!(
            lecture_opt.is_none(),
            "ΣLocal devrait être indisponible pour tan(pi/2)"
        );
    }

    // --- Trig récursive ---

    #[test]
    fn trig_dans_expression_composee() {
        let exact = ok_exact_only("1/2 + sin(pi/4)");
        assert_contains(&exact, "√2");
    }

    // --- Rationnels ---

    #[test]
    fn rationnel_add() {
        let exact = ok_exact_only("1/2 + 1/3");
        assert_contains(&exact.replace(' ', ""), "5/6");
    }

    #[test]
    fn rationnel_mul() {
        let exact = ok_exact_only("2/3 * 3/4");
        assert_contains(&exact.replace(' ', ""), "1/2");
    }

    #[test]
    fn rationnel_neg_et_parentheses() {
        let exact = ok_exact_only("-(1/2) + 1");
        assert_contains(&exact.replace(' ', ""), "1/2");
    }

    // --------------------------------------------------------------------
    // Tests “béton” : angles spéciaux + cos + combos + modulo
    // --------------------------------------------------------------------

    #[test]
    fn trig_cos_pi_3() {
        // cos(π/3) = 1/2
        let exact = ok_exact_only("cos(pi/3)");
        assert_contains(&exact.replace(' ', ""), "1/2");

        let dec = ok_dec("cos(pi/3)", 20);
        assert_contains(&dec, "0.5");
    }

    #[test]
    fn trig_cos_pi() {
        // cos(π) = -1
        let exact = ok_exact_only("cos(pi)");
        assert_contains(&exact, "1");
        assert_contains(&exact, "-");
    }

    #[test]
    fn trig_sin_3pi_2_modulo() {
        // sin(3π/2) = -1
        let exact = ok_exact_only("sin(3*pi/2)");
        assert_contains(&exact, "-");
        assert_contains(&exact, "1");
    }

    #[test]
    fn trig_tan_3pi_2_indefini() {
        // tan(3π/2) indéfini
        let (exact, lecture_opt) = ok_exact("tan(3*pi/2)", 20);
        assert_eq_trim(&exact, "indéfini");
        assert!(lecture_opt.is_none());
    }

    #[test]
    fn trig_combo_mul() {
        // 2*sin(pi/4) = √2
        let exact = ok_exact_only("2*sin(pi/4)");
        assert_contains(&exact, "√2");
    }

    #[test]
    fn trig_combo_prod_sqrt() {
        // sin(pi/4)*sqrt(2) = 1
        let exact = ok_exact_only("sin(pi/4)*sqrt(2)");
        assert_contains(&exact.replace(' ', ""), "1");
    }

    #[test]
    fn espaces_et_majuscules() {
        let exact = ok_exact_only("  SIN ( PI / 4 ) ");
        assert_contains(&exact, "√2");
    }
}
