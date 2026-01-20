// src/noyau/jetons.rs

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};

#[derive(Clone, Debug)]
pub enum Tok {
    Num(BigRational),
    Pi,

    // Fonctions + variables (tout ce qui n’est pas pi / opérateur / nombre)
    // NOTE: le parse (RPN->Expr) décidera si c’est une fonction (sin/cos/...) ou une variable.
    Ident(String),

    Plus,
    Minus,
    Star,
    Slash,
    Caret, // ^

    LPar,
    RPar,
}

/// Tokenize une chaîne en jetons.
/// Supporte:
/// - entiers (ex: 12)
/// - fractions littérales sans espaces (ex: 12/34) -> Num(12/34)
/// - opérateurs + - * / ^
/// - parenthèses ( )
/// - π ou pi
/// - identifiants [a-zA-Z_][a-zA-Z0-9_]* (normalisés en minuscules)
/// - √ (équivaut à ident("sqrt"))
pub fn tokenize(s: &str) -> Result<Vec<Tok>, String> {
    let mut out = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i: usize = 0;

    while i < chars.len() {
        let c = chars[i];

        if c.is_whitespace() {
            i += 1;
            continue;
        }

        // Parenthèses
        if c == '(' {
            out.push(Tok::LPar);
            i += 1;
            continue;
        }
        if c == ')' {
            out.push(Tok::RPar);
            i += 1;
            continue;
        }

        // Opérateurs
        match c {
            '+' => {
                out.push(Tok::Plus);
                i += 1;
                continue;
            }
            '-' => {
                out.push(Tok::Minus);
                i += 1;
                continue;
            }
            '*' => {
                out.push(Tok::Star);
                i += 1;
                continue;
            }
            '/' => {
                out.push(Tok::Slash);
                i += 1;
                continue;
            }
            '^' => {
                out.push(Tok::Caret);
                i += 1;
                continue;
            }
            _ => {}
        }

        // π : "π" ou "pi" / "PI" (insensible à la casse)
        if c == 'π' {
            out.push(Tok::Pi);
            i += 1;
            continue;
        }
        if (c == 'p' || c == 'P')
            && i + 1 < chars.len()
            && (chars[i + 1] == 'i' || chars[i + 1] == 'I')
        {
            out.push(Tok::Pi);
            i += 2;
            continue;
        }

        // Racine carrée unicode : √  => ident("sqrt")
        if c == '√' {
            out.push(Tok::Ident("sqrt".to_string()));
            i += 1;
            continue;
        }

        // Identifiants ASCII : [a-zA-Z_][a-zA-Z0-9_]*
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let w = word.to_lowercase();

            // Si tu veux ULTRA strict (x seulement), remplace ceci par:
            // if w != "x" && w != "sin" && w != "cos" && w != "tan" && w != "sqrt" && w != "pi" { ... }
            // mais ici on reste “un peu plus général” comme demandé.

            // Normalisation : "pi" devient Tok::Pi (même si on gère déjà "PI" plus haut)
            if w == "pi" {
                out.push(Tok::Pi);
            } else {
                out.push(Tok::Ident(w));
            }
            continue;
        }

        // Nombre entier ou fraction littérale a/b (sans espaces)
        if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let int_str: String = chars[start..i].iter().collect();
            let n = BigInt::parse_bytes(int_str.as_bytes(), 10).ok_or("nombre invalide")?;

            // par défaut: entier
            let mut rat = BigRational::from_integer(n.clone());

            // fraction immédiate: 12/34 (pas de parenthèses, pas d’espaces)
            if i < chars.len() && chars[i] == '/' {
                let save = i;
                i += 1;
                let start_d = i;

                // si pas un chiffre après '/', c’est une division normale (on recule)
                if start_d >= chars.len() || !chars[start_d].is_ascii_digit() {
                    i = save; // on remet sur '/'
                } else {
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                    let d_str: String = chars[start_d..i].iter().collect();
                    let d =
                        BigInt::parse_bytes(d_str.as_bytes(), 10).ok_or("dénominateur invalide")?;
                    if d.is_zero() {
                        return Err("division par zéro dans une fraction".into());
                    }
                    rat = BigRational::new(n, d);
                }
            }

            out.push(Tok::Num(rat));
            continue;
        }

        return Err(format!("caractère inattendu: '{c}'"));
    }

    Ok(out)
}

/// Format utilitaire (debug/“démarche”) : liste de jetons en texte.
pub fn format_tokens(tokens: &[Tok]) -> String {
    fn format_rat(r: &BigRational) -> String {
        let n = r.numer();
        let d = r.denom();
        if d.is_one() {
            format!("{n}")
        } else {
            format!("{n}/{d}")
        }
    }

    let mut out = Vec::new();
    for t in tokens {
        let s = match t {
            Tok::Num(r) => format_rat(r),
            Tok::Pi => "π".to_string(),
            Tok::Ident(name) => name.clone(),

            Tok::Plus => "+".to_string(),
            Tok::Minus => "-".to_string(),
            Tok::Star => "*".to_string(),
            Tok::Slash => "/".to_string(),
            Tok::Caret => "^".to_string(),

            Tok::LPar => "(".to_string(),
            Tok::RPar => ")".to_string(),
        };
        out.push(s);
    }
    out.join(" ")
}
