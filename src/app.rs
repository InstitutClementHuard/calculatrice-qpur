// src/app.rs
//
// Calculatrice Q-pur — module App (racine)
// ---------------------------------------
// Remplace l’ancien src/app/mod.rs.
//
// Rôle:
// - Déclarer les sous-modules (etat.rs + vue.rs)
// - Ré-exporter AppCalc (pour main.rs: use crate::app::AppCalc;)
// - Fournir l’impl eframe::App (compatible NATIF + WEB)
//
// Important:
// - La gestion Enter/Backspace est faite dans vue.rs (au bon endroit: quand le champ a le focus).
// - Ici, on évite d’appeler des méthodes privées de vue.rs.

pub mod etat;
pub mod vue;

// Ré-export pratique : `use crate::app::AppCalc;`
pub use etat::AppCalc;

use eframe::egui;

impl eframe::App for AppCalc {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Raccourci clavier global minimal (safe natif + web) :
        // ESC = effacer seulement l’entrée (comme bouton "C").
        //
        // On NE gère PAS Enter/Backspace ici:
        // - sur web/mobile, clavier incertain
        // - risque de double déclenchement
        // - la vue le fait déjà avec resp.has_focus()
        let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        if esc {
            self.clear_entree(); // méthode publique de etat.rs
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            self.ui(ui); // méthode publique (dans vue.rs)
        });
    }
}
