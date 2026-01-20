//! Noyau exact Q-pur
//!
//! Organisation interne :
//! - expr.rs     : AST exact + simplify + coeff*π
//! - format.rs   : affichage EXACT “joli” (√2/2, √3/3, π/2…)
//! - jetons.rs   : tokenisation
//! - rpn.rs      : shunting-yard + construction Expr
//! - trig.rs     : angles spéciaux + indéfini
//! - lecture.rs  : ΣLocal (décimal tronqué) + cache π
//! - eval.rs     : pipeline complet

pub mod canon;
pub mod eval;
pub mod expr;
pub mod format;
pub mod identites_trig;
pub mod jetons;
pub mod lecture;
pub mod rpn;
pub mod trig;

#[cfg(test)]
mod tests_scientifiques;

#[cfg(test)]
mod tests_fuzz_safe;

// API publique minimale
pub use eval::eval_expression;
