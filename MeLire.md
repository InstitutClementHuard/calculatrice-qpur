Calculatrice Q-pur — Architecture interne

Projet Rust / eframe
Objectif : moteur de calcul exact symbolique + projection décimale contrôlée (ΣLocal), avec preuves, invariants et robustesse testée.

Vue d’ensemble

Le projet est composé de deux couches strictement séparées :

UI (app) : interaction utilisateur, affichage, ergonomie

Noyau exact (noyau) : mathématiques symboliques, trig spéciale, simplification, tests scientifiques

src/
  main.rs
  app/
  noyau/
assets/fonts/

Couche UI (application)
src/main.rs

Rôle

Point d’entrée du programme

Lance eframe / egui

Installe les polices Unicode (π, √, etc.)

Démarre l’application AppCalc

Responsabilités

Configuration fenêtre

Chargement des polices

Boucle principale graphique

src/app/etat.rs

Rôle

État pur de l’application (aucun rendu, aucun calcul).

Contient

entree : expression utilisateur

exact : résultat exact symbolique

lecture : résultat ΣLocal (décimal)

erreur

digits

demarche

focus_entree

Fonctions

C, CLR, AC

set_digits

set_resultats

set_erreur

➡️ C’est le modèle (au sens MVC).

src/app/vue.rs

Rôle

Interface graphique complète (egui).

Gère

Champ d’entrée

Pavé numérique

Boutons trig

EXACT / ΣLocal

Démarche détaillée

Focus clavier

Appelle

crate::noyau::eval_expression(...)


➡️ C’est la vue + contrôleur léger.

src/app/mod.rs

Relie etat et vue, expose AppCalc.

Couche noyau exact (moteur scientifique)
src/noyau/mod.rs

Rôle

Déclare tous les modules mathématiques

Expose l’API publique :

pub use eval::eval_expression;


Branche les campagnes de tests :

tests_scientifiques
tests_fuzz_safe

src/noyau/jetons.rs

Rôle : Tokenizer

Transforme :

sin(pi/4)+1/2


en jetons structurés.

Supporte

π / pi

sin cos tan sqrt

fractions exactes 12/34

opérateurs + − * / ^

parenthèses

src/noyau/rpn.rs

Rôle : Shunting-yard

Convertit jetons → RPN (notation polonaise inverse)

Gère :

priorités

associativité

parenthèses

exponentiation

src/noyau/expr.rs

Rôle : AST exact

Enum Expr représentant l’expression mathématique exacte.

Fonctions clés

simplify()
Règles exactes :

x − x = 0

√x · √x = x

rationalisation simple

propagation Indefini

puissances rationnelles

etc.

as_coeff_pi()
Détecte les formes : coeff * π

➡️ Cœur algébrique du système.

src/noyau/trig.rs

Rôle : trigonométrie spéciale

Reconnaît les angles rationnels multiples de π :

π/6, π/4, π/3, π/2, 3π/2, etc.

Produit :

résultat EXACT (ex: √2/2, √3/3)

ou Indefini (ex: tan(π/2))

Génère aussi la preuve textuelle utilisée dans la démarche.

src/noyau/format.rs

Rôle : affichage EXACT canonique

Transforme l’AST en texte lisible :

√2/2

√3/3

π/2

-√2/2

etc.

Évite les parenthèses lourdes inutiles.

src/noyau/lecture.rs

Rôle : projection ΣLocal

Calcule une approximation décimale tronquée

Précision paramétrable

Refuse proprement :

indéfini

division par zéro

certains irrationnels imbriqués

➡️ Projection contrôlée, jamais utilisée comme base du calcul exact.

src/noyau/eval.rs

Rôle : pipeline complet

Chaîne officielle :

tokenize
→ RPN
→ AST Expr
→ simplify
→ trig_special récursif
→ resimplify
→ EXACT
→ ΣLocal


Construit aussi la démarche :

jetons

RPN

avant

après

preuve trig

note pipeline

Campagnes de tests
src/noyau/tests_scientifiques.rs

Tests déterministes scientifiques :

identités trig

symétries

périodicité (canonisée)

propagation indéfini

zéro algébrique

cohérence ΣLocal

stress borné :

profondeur sqrt

taille somme balancée

BigInt contrôlé

➡️ Vérifie les invariants mathématiques.

src/noyau/tests_fuzz_safe.rs

Tests fuzz contrôlés :

génération d’expressions aléatoires bornées

vérification :

déterminisme

absence de gel

anti stack overflow

domaine trig valide

invariant ΣLocal/indéfini

➡️ Vérifie la robustesse structurelle.

État actuel du projet

26 tests unitaires + scientifiques + fuzz

Tous passent

Aucune alerte Clippy

Aucun UB

Pile protégée

Temps borné

➡️ Noyau mathématique exact stable.

Prochaines optimisations (ordre logique)
1. Noyau exact (priorité)

Normalisation avancée des angles :

pi/4 + 2*pi → 9*pi/4


Simplification algébrique plus globale (arbres balancés automatiques)

2. UI

Bouton . :

soit supprimé

soit converti en rationnel exact (/10)

soit ajout support décimaux exacts

3. Tests avancés

Campagne fuzz métamorphique (lois algébriques)

Campagne “physique symbolique simple” (équations conservations, unités symboliques)

Résumé conceptuel

Ce que tu as construit :

Pas une simple calculatrice

Un noyau de calcul exact ontologiquement cohérent

Séparant :

représentation

transformation

projection

preuve

validation

Compatible avec ton paradigme :

le nombre n’est qu’une projection, pas la réalité fondamentale

Ici :

Expr = structure ontologique

EXACT = forme canonique

ΣLocal = simple lecture locale

---

[Lotusxii@archlinux calculatrice_qpur]$ cd /home/Lotusxii/Projets/calculatrice_qpur
find . -maxdepth 3 -type f \
  \( -name '*.rs' -o -name 'Cargo.toml' -o -name 'Cargo.lock' -o -name '*.md' -o -name '*.ttf' \) \
  | sed 's|^\./||' | sort
assets/fonts/DejaVuSansMono.ttf
assets/fonts/DejaVuSans.ttf
Cargo.lock
Cargo.toml
MeLire.md
src/app/etat.rs
src/app/mod.rs
src/app/vue.rs
src/main.rs
src/noyau/eval.rs
src/noyau/expr.rs
src/noyau/format.rs
src/noyau/jetons.rs
src/noyau/lecture.rs
src/noyau/mod.rs
src/noyau/rpn.rs
src/noyau/tests_fuzz_safe.rs
src/noyau/tests_scientifiques.rs
src/noyau/trig.rs
[Lotusxii@archlinux calculatrice_qpur]$ grep -RIn --include='*.rs' -E '^\s*(pub\s+)?mod\s+|^\s*pub\s+use\s+' src
src/app/mod.rs:1:pub mod etat;
src/app/mod.rs:2:pub mod vue;
src/app/mod.rs:5:pub use etat::AppCalc;
src/main.rs:3:mod app;
src/main.rs:4:mod noyau;
src/noyau/mod.rs:12:pub mod eval;
src/noyau/mod.rs:13:pub mod expr;
src/noyau/mod.rs:14:pub mod format;
src/noyau/mod.rs:15:pub mod jetons;
src/noyau/mod.rs:16:pub mod lecture;
src/noyau/mod.rs:17:pub mod rpn;
src/noyau/mod.rs:18:pub mod trig;
src/noyau/mod.rs:21:mod tests_scientifiques;
src/noyau/mod.rs:24:mod tests_fuzz_safe;
src/noyau/mod.rs:27:pub use eval::eval_expression;
src/noyau/eval.rs:177:mod tests {
[Lotusxii@archlinux calculatrice_qpur]$ grep -RIn --include='*.rs' -E 'eval_expression\(|crate::noyau::eval_expression|impl eframe::App|fn ui\(' src
src/app/vue.rs:7:    pub fn ui(&mut self, ui: &mut egui::Ui) {
src/app/vue.rs:306:        match crate::noyau::eval_expression(s, self.digits) {
src/main.rs:61:impl eframe::App for AppCalc {
src/noyau/tests_fuzz_safe.rs:228:        match eval_expression(&expr, digits) {
src/noyau/tests_fuzz_safe.rs:262:        match eval_expression(&expr, 25) {
src/noyau/tests_fuzz_safe.rs:285:    let (exact, _lecture, _d) = eval_expression(&expr, 10).unwrap_or_else(|e| panic!("err: {e}"));
src/noyau/eval.rs:29:pub fn eval_expression(
src/noyau/eval.rs:181:        let (exact, lecture_opt, _d) = eval_expression(s, digits)
src/noyau/eval.rs:182:            .unwrap_or_else(|e| panic!("eval_expression({s:?}) erreur: {e}"));
src/noyau/tests_scientifiques.rs:24:        eval_expression(expr, digits).unwrap_or_else(|e| panic!("expr={expr:?} err={e}"));
[Lotusxii@archlinux calculatrice_qpur]$ cargo fmt --all \
&& cargo clippy --all-targets --all-features -- -D warnings \
&& cargo test --all-features
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.01s
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.72s
     Running unittests src/main.rs (target/debug/deps/calculatrice_qpur-e68d09edc4171d26)

running 26 tests
test noyau::eval::tests::rationnel_mul ... ok
test noyau::eval::tests::rationnel_add ... ok
test noyau::eval::tests::espaces_et_majuscules ... ok
test noyau::eval::tests::rationnel_neg_et_parentheses ... ok
test noyau::eval::tests::trig_cos_pi ... ok
test noyau::eval::tests::trig_cos_pi_3 ... ok
test noyau::eval::tests::trig_combo_prod_sqrt ... ok
test noyau::eval::tests::trig_dans_expression_composee ... ok
test noyau::eval::tests::trig_combo_mul ... ok
test noyau::eval::tests::trig_sin_3pi_2_modulo ... ok
test noyau::eval::tests::trig_tan_3pi_2_indefini ... ok
test noyau::eval::tests::trig_tan_pi_2_indefini ... ok
test noyau::eval::tests::trig_sin_pi_4 ... ok
test noyau::eval::tests::trig_tan_pi_6 ... ok
test noyau::tests_scientifiques::sci_identites_symetrie ... ok
test noyau::tests_scientifiques::sci_indefinis_tan ... ok
test noyau::tests_scientifiques::sci_periodicite_angles ... ok
test noyau::tests_scientifiques::sci_propagation_indefini ... ok
test noyau::tests_scientifiques::sci_socal_coherence_basic ... ok
test noyau::tests_scientifiques::sci_stress_bigint_safe ... ok
test noyau::tests_scientifiques::sci_stress_profondeur_sqrt_safe ... ok
test noyau::tests_scientifiques::sci_stress_taille_somme_safe ... ok
test noyau::tests_fuzz_safe::fuzz_safe_angles_trig_dans_domaine ... ok
test noyau::tests_scientifiques::sci_zero_algebrique ... ok
test noyau::tests_fuzz_safe::fuzz_safe_somme_balancee_anti_pile ... ok
test noyau::tests_fuzz_safe::fuzz_safe_determinisme_et_invariant_socal ... ok

test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

[Lotusxii@archlinux calculatrice_qpur]$ .
