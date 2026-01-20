// src/noyau/rpn.rs
//
// Shunting-yard -> RPN -> AST
// Objectif:
// - Convertir une suite de Tok en RPN (postfix)
// - Puis reconstruire Expr
//
// Règles:
// - Ident(name):
//    - si name ∈ {sin, cos, tan, sqrt} => fonction unaire (postfixée en RPN)
//    - sinon => variable/atome (Expr::Var)
// - Moins unaire:
//    - si '-' arrive quand on n’attend PAS une valeur, on injecte 0 : "-x" => "0 x -"
//
// NOTE:
// - Les fonctions sont traitées comme des opérateurs “collés” à leur argument
//   et sont sorties après la parenthèse fermante.

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};

use super::expr::Expr;
use super::jetons::Tok;

fn precedence(t: &Tok) -> i32 {
    match t {
        Tok::Plus | Tok::Minus => 1,
        Tok::Star | Tok::Slash => 2,
        Tok::Caret => 3,
        _ => 0,
    }
}

fn is_right_associative(t: &Tok) -> bool {
    matches!(t, Tok::Caret)
}

/// Identificateurs reconnus comme fonctions (unaire).
fn is_fonction_ident(name: &str) -> bool {
    matches!(name, "sin" | "cos" | "tan" | "sqrt")
}

/// Convertit une suite de jetons en RPN (notation polonaise inversée).
///
/// Exemple:
///   tokens: [Ident("sin"), LPar, Pi, Slash, Num(2), RPar]
///   rpn:    [Pi, Num(2), Slash, Ident("sin")]
pub fn to_rpn(tokens: &[Tok]) -> Result<Vec<Tok>, String> {
    let mut out: Vec<Tok> = Vec::new();
    let mut ops: Vec<Tok> = Vec::new();

    // “valeur” = un atome ou une expression fermée.
    // Sert à détecter le moins unaire.
    let mut prev_was_value = false;

    for tok in tokens.iter().cloned() {
        match tok {
            Tok::Num(_) | Tok::Pi => {
                out.push(tok);
                prev_was_value = true;
            }

            Tok::Ident(name) => {
                if is_fonction_ident(&name) {
                    // fonction : on la garde sur la pile (elle sortira après son argument)
                    ops.push(Tok::Ident(name));
                    prev_was_value = false;
                } else {
                    // variable/atome : sortie directe
                    out.push(Tok::Ident(name));
                    prev_was_value = true;
                }
            }

            Tok::LPar => {
                ops.push(tok);
                prev_was_value = false;
            }

            Tok::RPar => {
                // dépile jusqu’à '('
                while let Some(top) = ops.pop() {
                    if matches!(top, Tok::LPar) {
                        break;
                    }
                    out.push(top);
                }

                // si une fonction est au sommet, on la sort aussi
                // (forme Clippy: pas de if-let imbriqué inutile)
                if let Some(Tok::Ident(name)) = ops.last() {
                    if is_fonction_ident(name.as_str()) {
                        out.push(ops.pop().unwrap());
                    }
                }

                prev_was_value = true;
            }

            Tok::Plus | Tok::Star | Tok::Slash | Tok::Caret => {
                // dépile tant que:
                // - on n'est pas bloqué par '('
                // - et on ne traverse pas une fonction (fonction reste collée à son argument)
                // - et la précédence/associativité exige de sortir l'opérateur du haut
                while let Some(top) = ops.last() {
                    if matches!(top, Tok::LPar) {
                        break;
                    }
                    if let Tok::Ident(name) = top {
                        if is_fonction_ident(name.as_str()) {
                            break;
                        }
                    }

                    let p_top = precedence(top);
                    let p_tok = precedence(&tok);

                    let doit_pop = if is_right_associative(&tok) {
                        p_top > p_tok
                    } else {
                        p_top >= p_tok
                    };

                    if doit_pop {
                        out.push(ops.pop().unwrap());
                    } else {
                        break;
                    }
                }

                ops.push(tok);
                prev_was_value = false;
            }

            Tok::Minus => {
                // moins unaire : si pas de valeur avant, injecte 0
                if !prev_was_value {
                    out.push(Tok::Num(BigRational::zero()));
                }

                while let Some(top) = ops.last() {
                    if matches!(top, Tok::LPar) {
                        break;
                    }
                    if let Tok::Ident(name) = top {
                        if is_fonction_ident(name.as_str()) {
                            break;
                        }
                    }
                    if precedence(top) >= precedence(&Tok::Minus) {
                        out.push(ops.pop().unwrap());
                    } else {
                        break;
                    }
                }

                ops.push(Tok::Minus);
                prev_was_value = false;
            }
        }
    }

    // vide la pile ops
    while let Some(op) = ops.pop() {
        if matches!(op, Tok::LPar) {
            return Err("parenthèses non fermées".into());
        }
        out.push(op);
    }

    Ok(out)
}

/// Construit une Expr à partir d’une RPN.
///
/// - Ident(name):
///     - si name ∈ {sin,cos,tan,sqrt} => fonction unaire
///     - sinon => variable : Expr::Var(name)
pub fn from_rpn(rpn: &[Tok]) -> Result<Expr, String> {
    let mut st: Vec<Expr> = Vec::new();

    for tok in rpn.iter().cloned() {
        match tok {
            Tok::Num(r) => st.push(Expr::Rat(r)),
            Tok::Pi => st.push(Expr::Pi),

            Tok::Plus | Tok::Minus | Tok::Star | Tok::Slash | Tok::Caret => {
                let b = st.pop().ok_or("expression invalide")?;
                let a = st.pop().ok_or("expression invalide")?;

                let e = match tok {
                    Tok::Plus => Expr::Add(Box::new(a), Box::new(b)),
                    Tok::Minus => Expr::Sub(Box::new(a), Box::new(b)),
                    Tok::Star => Expr::Mul(Box::new(a), Box::new(b)),
                    Tok::Slash => Expr::Div(Box::new(a), Box::new(b)),
                    Tok::Caret => {
                        // exposant entier seulement
                        let n = match b {
                            Expr::Rat(r) => {
                                if !r.denom().is_one() {
                                    return Err("exposant doit être entier".into());
                                }
                                big_to_i64(r.numer()).ok_or("exposant trop grand")?
                            }
                            _ => return Err("exposant doit être entier".into()),
                        };
                        Expr::PowInt(Box::new(a), n)
                    }
                    _ => unreachable!(),
                };

                st.push(e);
            }

            Tok::Ident(name) => {
                if is_fonction_ident(name.as_str()) {
                    let x = st.pop().ok_or("fonction sans argument")?;
                    let e = match name.as_str() {
                        "sqrt" => Expr::Sqrt(Box::new(x)),
                        "sin" => Expr::Sin(Box::new(x)),
                        "cos" => Expr::Cos(Box::new(x)),
                        "tan" => Expr::Tan(Box::new(x)),
                        _ => unreachable!(),
                    };
                    st.push(e);
                } else {
                    st.push(Expr::Var(name));
                }
            }

            Tok::LPar | Tok::RPar => return Err("parenthèse inattendue en RPN".into()),
        }
    }

    if st.len() != 1 {
        return Err("expression invalide".into());
    }
    Ok(st.pop().unwrap())
}

/// Conversion SAFE vers i64.
/// (MVP: exposant doit rentrer dans i64, sinon on refuse)
fn big_to_i64(x: &BigInt) -> Option<i64> {
    x.to_string().parse::<i64>().ok()
}
