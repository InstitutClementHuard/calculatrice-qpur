//! Tests scientifiques (campagne) : invariants + robustesse + limites contrôlées.
//!
//! But : trouver les limites sans faire chauffer la machine.
//! - budget temps global
//! - tailles bornées (profondeur, longueur)
//! - digits limités pour ΣLocal
//!
//! Notes importantes (aligné avec l’état actuel du noyau) :
//! - Trig spéciale reconnaît des angles “spéciaux” sous forme coeff*π (via as_coeff_pi).
//!   Donc: "pi/4 + 2*pi" n’est pas reconnu (Add), à moins d’une réduction modulo dans trig.
//!   On teste donc la périodicité via des angles équivalents directement reconnus (9π/4, 7π/3, 7π/6).
//! - Zéro algébrique : le noyau simplifie bien les rationnels,
//!   mais ne fait pas encore l’“annulation structurelle” A - A => 0 pour expressions non-Rat.
//!   Donc on teste des zéros qui passent par la simplification rationnelle / racines exactes déjà supportées.
//! - Stress : on évite les expressions qui causent profondeur récursive énorme (risque stack overflow).
//!   On reste sur des bornes petites + budgets courts.

use std::time::{Duration, Instant};

use super::eval_expression;

fn eval_ok(expr: &str, digits: usize) -> (String, Option<String>) {
    let (exact, lecture, _d) =
        eval_expression(expr, digits).unwrap_or_else(|e| panic!("expr={expr:?} err={e}"));
    (exact, lecture)
}

fn assert_indefini(expr: &str) {
    let (exact, lecture) = eval_ok(expr, 40);
    assert_eq!(exact.trim(), "indéfini", "expr={expr:?}");
    assert!(
        lecture.is_none(),
        "ΣLocal devrait être None pour expr={expr:?}"
    );
}

fn assert_exact_eq(expr: &str, attendu: &str) {
    let (exact, _lecture) = eval_ok(expr, 50);
    assert_eq!(exact.trim(), attendu.trim(), "expr={expr:?}");
}

/// Budget global anti-gel (scientifique + safe).
fn budget(start: Instant, max: Duration) {
    if start.elapsed() > max {
        panic!("budget temps dépassé: {:?}", max);
    }
}

/* ------------------------ Invariants trig (angles spéciaux) ------------------------ */

#[test]
fn sci_indefinis_tan() {
    assert_indefini("tan(pi/2)");
    assert_indefini("tan(3*pi/2)");
    assert_indefini("tan(-pi/2)");
}

#[test]
fn sci_identites_symetrie() {
    // sin(-x) = -sin(x)
    assert_exact_eq("sin(-pi/4)", "-√2/2");
    assert_exact_eq("sin(pi/4)", "√2/2");

    // cos(-x) = cos(x)
    assert_exact_eq("cos(-pi/3)", "1/2");
    assert_exact_eq("cos(pi/3)", "1/2");

    // tan(-x) = -tan(x) (hors indéfini)
    assert_exact_eq("tan(-pi/6)", "-√3/3");
    assert_exact_eq("tan(pi/6)", "√3/3");
}

#[test]
fn sci_periodicite_angles() {
    // Périodicité testée via angles déjà reconnus (coeff*π), sans "+ 2*pi".
    // sin(x + 2π) = sin(x) : π/4 + 2π = 9π/4
    assert_exact_eq("sin(9*pi/4)", "√2/2");

    // cos(x + 2π) = cos(x) : π/3 + 2π = 7π/3
    assert_exact_eq("cos(7*pi/3)", "1/2");

    // tan(x + π) = tan(x) : π/6 + π = 7π/6
    // tan(7π/6) = tan(π + π/6) = tan(π/6) = √3/3
    assert_exact_eq("tan(7*pi/6)", "√3/3");
}

#[test]
fn sci_propagation_indefini() {
    // indéfini doit contaminer les opérations
    assert_indefini("1 + tan(pi/2)");
    assert_indefini("tan(pi/2) + 1");
    assert_indefini("2 * tan(pi/2)");
    assert_indefini("tan(pi/2) / 3");
}

/* ------------------------ Cohérence algébrique (zéro) ------------------------ */

#[test]
fn sci_zero_algebrique() {
    // (1/2 + 1/3) - 5/6 = 0
    assert_exact_eq("(1/2 + 1/3) - 5/6", "0");

    // (2/3 * 3/4) - 1/2 = 0
    assert_exact_eq("(2/3 * 3/4) - 1/2", "0");

    // sqrt(2)*sqrt(2) - 2 = 0
    assert_exact_eq("sqrt(2)*sqrt(2) - 2", "0");

    // 1/sqrt(3) = √3/3 (dans notre noyau, la rationalisation renvoie √3/3)
    // donc √3/3 - √3/3 = 0 serait une annulation structurelle (pas encore),
    // on teste plutôt l’égalité en comparant les deux côtés séparément.
    assert_exact_eq("1/sqrt(3)", "√3/3");
    assert_exact_eq("sqrt(3)/3", "√3/3");
}

/* ------------------------ Stress contrôlé (sans brûler) ------------------------ */

#[test]
fn sci_stress_profondeur_sqrt_safe() {
    let t0 = Instant::now();
    let max = Duration::from_millis(200);

    // IMPORTANT : ton noyau n’accepte sqrt(x) en lecture ΣLocal que si argument rationnel
    // à l’intérieur de lecture.rs (et simplify fait seulement √(rat) exact si carré parfait).
    // Donc on ne “chaîne” pas sqrt(sqrt(...)) : ça devient non rationnel -> erreur.
    // Stress safe : profondeur modérée sur une forme rationnelle (carrés parfaits).
    //
    // Exemple: sqrt(4) -> 2 ; sqrt(9) -> 3 ; etc. On alterne pour garder rationnel.
    let mut expr = "4".to_string();
    for k in 0..60 {
        expr = if k % 2 == 0 {
            format!("sqrt({expr})") // reste rationnel ici (car expr sera un carré parfait à chaque étape)
        } else {
            // on remonte à un carré parfait rationnel
            format!("({expr})^2")
        };
        budget(t0, max);
    }

    let (exact, _lec) = eval_ok(&expr, 20);
    assert!(!exact.trim().is_empty());
}

#[test]
fn sci_stress_taille_somme_safe() {
    let t0 = Instant::now();
    let max = Duration::from_millis(200);

    // On évite une somme énorme (risque stack overflow via AST récursif profond).
    // Stress safe : 80 termes (suffisant pour détecter régressions, sans exploser la pile).
    let mut expr = String::new();
    for k in 0..80 {
        if k > 0 {
            expr.push_str(" + ");
        }
        expr.push_str("1/2");
        budget(t0, max);
    }

    // 80*(1/2)=40
    let (exact, _lec) = eval_ok(&expr, 20);
    assert!(exact.trim() == "40" || exact.contains("40"));
}

#[test]
fn sci_stress_bigint_safe() {
    let t0 = Instant::now();
    let max = Duration::from_millis(200);

    // gros numérateur contrôlé (100 chiffres)
    let big = "9".repeat(100);
    let expr = format!("{big}/7 + 1/7");
    budget(t0, max);

    // doit rester rationnel et ne pas geler
    let (exact, _lec) = eval_ok(&expr, 20);
    assert!(!exact.trim().is_empty());
}

/* ------------------------ ΣLocal : cohérence minimale ------------------------ */

#[test]
fn sci_socal_coherence_basic() {
    // si EXACT n'est pas indéfini, ΣLocal doit exister (tant que lecture sait évaluer l’expression)
    let (exact, lec) = eval_ok("sin(pi/4)", 30);
    assert_ne!(exact.trim(), "indéfini");
    assert!(lec.is_some());

    // indéfini -> ΣLocal none
    let (exact2, lec2) = eval_ok("tan(pi/2)", 30);
    assert_eq!(exact2.trim(), "indéfini");
    assert!(lec2.is_none());
}
