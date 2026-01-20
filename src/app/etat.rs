//! src/app/etat.rs
//!
//! État UI (sans vue, sans noyau).
//!
//! Rôle : contenir l’état de la calculatrice (entrée, résultats, erreur, digits, démarche)
//! et offrir des opérations simples (C/CLR/AC) sans logique d’affichage.
//!
//! Contrats (Loi de Clément, version UI) :
//! - Aucune évaluation ici (pas de noyau, pas de parsing).
//! - Actions déterministes, sans effet de bord caché.
//! - Défense en profondeur : bornes sur ΣLocal (digits).

/// Précision ΣLocal par défaut (lecture décimale tronquée).
const DIGITS_DEFAUT: usize = 20;

/// Garde-fou : on borne la précision (anti-abus / anti-gel).
const DIGITS_MAX: usize = 200;

#[derive(Clone, Default, Debug)]
pub struct Demarche {
    pub jetons: String,
    pub rpn: String,
    pub avant: String,
    pub apres: String,
    pub note: String,
    pub preuve: String,
}

#[derive(Clone, Debug)]
pub struct AppCalc {
    // --- entrée utilisateur ---
    pub entree: String,

    // --- sorties ---
    pub exact: String,       // affichage EXACT (forme finie / symbolique)
    pub lecture: String,     // ΣLocal (décimal tronqué)
    pub erreur: String,      // message d’erreur (si parsing/éval échoue)
    pub lecture_dispo: bool, // false si indéfini / impossible / vide

    // --- démarche (panneau d’explication) ---
    pub demarche: Demarche,

    // --- paramètres ---
    pub digits: usize, // précision ΣLocal

    // --- UX ---
    // Permet à vue.rs de redonner le focus à l’entrée après un clic sur un bouton.
    pub focus_entree: bool,
}

impl Default for AppCalc {
    fn default() -> Self {
        Self {
            entree: String::new(),
            exact: String::new(),
            lecture: String::new(),
            erreur: String::new(),
            lecture_dispo: false, // au démarrage : rien à lire
            demarche: Demarche::default(),
            digits: DIGITS_DEFAUT,
            focus_entree: true, // au lancement, on veut pouvoir taper tout de suite
        }
    }
}

impl AppCalc {
    /* ------------------------ Actions “boutons” (état seulement) ------------------------ */

    /// AC : remise à zéro totale (entrée + résultats + digits par défaut).
    pub fn reset_total(&mut self) {
        self.entree.clear();
        self.clear_resultats();
        self.digits = DIGITS_DEFAUT;
        self.focus_entree = true;
    }

    /// C : effacer seulement l’entrée (sans toucher aux résultats).
    pub fn clear_entree(&mut self) {
        self.entree.clear();
        self.focus_entree = true;
    }

    fn clear_demarche(&mut self) {
        self.demarche = Demarche::default();
    }

    /// CLR : effacer résultats + erreur + démarche (sans toucher à l’entrée).
    pub fn clear_resultats(&mut self) {
        self.exact.clear();
        self.lecture.clear();
        self.erreur.clear();
        self.lecture_dispo = false; // clair : il n’y a rien à lire
        self.clear_demarche();
        self.focus_entree = true;
    }

    /// Utilitaire : placer une erreur.
    ///
    /// Choix UX :
    /// - On CONSERVE `exact` (dernier résultat) pour ne pas “effacer l’écran” sur une faute.
    /// - On coupe ΣLocal + démarche (non fiable si l’évaluation échoue).
    pub fn set_erreur(&mut self, msg: impl Into<String>) {
        self.erreur = msg.into();

        // ΣLocal indisponible en cas d’erreur
        self.lecture.clear();
        self.lecture_dispo = false;

        // pipeline “preuve” invalide => on efface la démarche
        self.clear_demarche();

        self.focus_entree = true;
    }

    /// Utilitaire : déposer un résultat complet (EXACT + lecture optionnelle + démarche).
    pub fn set_resultats(
        &mut self,
        exact: impl Into<String>,
        lecture: Option<String>,
        demarche: Demarche,
    ) {
        self.erreur.clear();
        self.exact = exact.into();
        self.demarche = demarche;

        if let Some(v) = lecture {
            self.lecture_dispo = true;
            self.lecture = v;
        } else {
            self.lecture_dispo = false;
            self.lecture.clear();
        }

        self.focus_entree = true;
    }

    /// Garde-fou : limite digits (évite abus / gel plus tard).
    pub fn set_digits(&mut self, digits: usize) {
        self.digits = digits.clamp(0, DIGITS_MAX);
        self.focus_entree = true;
    }
}
