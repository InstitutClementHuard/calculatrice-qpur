// src/app/vue.rs
//
// Vue (UI egui) — natif + web
// ---------------------------
// Objectifs :
// - Même AppCalc (etat.rs) pour natif + wasm
// - Clavier : Enter évalue, Backspace efface (quand le champ est focus)
// - Tactile : gros boutons, focus redonné après clic (focus_entree)
// - Boutons x/y (optionnel mais utile sur mobile)
//
// Note :
// - PAS de Key::NumEnter (n’existe pas dans egui 0.33.x)
// - Enter suffit (clavier PC + “Enter” virtuel mobile selon navigateur)

use eframe::egui;

use super::etat::{AppCalc, Demarche};

impl AppCalc {
    /// UI principale : à appeler depuis eframe::App::update(...)
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        // Densité “calc”
        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.heading("Calculatrice Q-pur");
                ui.add_space(6.0);

                self.ui_entree(ui);

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                self.ui_resultats(ui);

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                self.ui_demarche(ui);
            });
    }

    fn ui_entree(&mut self, ui: &mut egui::Ui) {
        ui.label("Entrée :");

        // IMPORTANT : id stable + focus contrôlé
        let resp = ui.add(
            egui::TextEdit::singleline(&mut self.entree)
                .desired_width(ui.available_width())
                .hint_text("Ex: (1/2)+sqrt(2)/2, sin(pi/4), tan(pi/2)")
                .id_source("entree_edit")
                .code_editor(),
        );

        // Si on a cliqué un bouton (pavé / fonctions / DEL / C / etc.), on redonne le focus
        if self.focus_entree {
            resp.request_focus();
            self.focus_entree = false;
        }

        // --- Clavier : Enter évalue (seulement si le champ est focus) ---
        // On évite les déclenchements “globaux” quand l’utilisateur clique ailleurs.
        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
        if resp.has_focus() && enter {
            self.eval_via_noyau();
            self.focus_entree = true;
        }

        // --- Clavier : Backspace (seulement si le champ est focus) ---
        // TextEdit gère déjà Backspace “normal”, mais notre backspace_entree()
        // est utile pour effacer des tokens complets ("sin(", "pi", etc.).
        let backspace = ui.input(|i| i.key_pressed(egui::Key::Backspace));
        if resp.has_focus() && backspace {
            self.backspace_entree();
            self.focus_entree = true;
        }

        ui.add_space(6.0);

        // Actions + ΣLocal
        ui.horizontal(|ui| {
            // Contrat: C = entrée seulement ; CLR = résultats seulement ; AC = tout
            self.bouton_action(ui, "C", "Efface seulement l’entrée", Action::ClearEntree);
            self.bouton_action(
                ui,
                "CLR",
                "Efface résultats + erreur + démarche",
                Action::ClearResultats,
            );
            self.bouton_action(ui, "AC", "Remise à zéro totale", Action::ResetTotal);

            ui.separator();

            ui.label("ΣLocal :");
            let mut d = self.digits as u32;
            let resp = ui.add(
                egui::DragValue::new(&mut d)
                    .speed(1)
                    .range(0..=200)
                    .suffix(" chiffres"),
            );
            if resp.changed() {
                self.set_digits(d as usize);
            }
        });

        ui.add_space(8.0);

        // Touches rapides + variables + "="
        ui.horizontal_wrapped(|ui| {
            self.bouton_insert(ui, "(", "(", InsertKind::OpenParen);
            self.bouton_insert(ui, ")", ")", InsertKind::CloseParen);

            self.bouton_insert(ui, "+", "+", InsertKind::Op);
            self.bouton_insert(ui, "-", "-", InsertKind::Op);
            self.bouton_insert(ui, "*", "*", InsertKind::Op);
            self.bouton_insert(ui, "/", "/", InsertKind::Op);
            self.bouton_insert(ui, "^", "^", InsertKind::Op);

            ui.separator();

            self.bouton_insert(ui, "pi", "pi", InsertKind::Word);
            self.bouton_insert(ui, "sqrt", "sqrt(", InsertKind::Func);
            self.bouton_insert(ui, "sin", "sin(", InsertKind::Func);
            self.bouton_insert(ui, "cos", "cos(", InsertKind::Func);
            self.bouton_insert(ui, "tan", "tan(", InsertKind::Func);

            ui.separator();

            // Variables (optionnel, mais super utile sur mobile)
            self.bouton_insert(ui, "x", "x", InsertKind::Word);
            self.bouton_insert(ui, "y", "y", InsertKind::Word);

            ui.add_space(10.0);

            let eq = ui.add_sized([64.0, 32.0], egui::Button::new("="));
            if eq.clicked() {
                self.eval_via_noyau();
                self.focus_entree = true;
            }
        });

        ui.add_space(8.0);

        // Pavé numérique
        self.ui_pave_numerique(ui);

        if !self.erreur.is_empty() {
            ui.add_space(6.0);
            ui.colored_label(ui.visuals().error_fg_color, &self.erreur);
        }
    }

    fn ui_pave_numerique(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("pave_numerique_qpur")
            .num_columns(4)
            .spacing([6.0, 6.0])
            .show(ui, |ui| {
                self.bouton_insert(ui, "7", "7", InsertKind::Digit);
                self.bouton_insert(ui, "8", "8", InsertKind::Digit);
                self.bouton_insert(ui, "9", "9", InsertKind::Digit);
                self.bouton_action(ui, "DEL", "Efface le dernier symbole", Action::Backspace);
                ui.end_row();

                self.bouton_insert(ui, "4", "4", InsertKind::Digit);
                self.bouton_insert(ui, "5", "5", InsertKind::Digit);
                self.bouton_insert(ui, "6", "6", InsertKind::Digit);
                self.bouton_insert(ui, "/", "/", InsertKind::Op);
                ui.end_row();

                self.bouton_insert(ui, "1", "1", InsertKind::Digit);
                self.bouton_insert(ui, "2", "2", InsertKind::Digit);
                self.bouton_insert(ui, "3", "3", InsertKind::Digit);
                self.bouton_insert(ui, ".", ".", InsertKind::Digit);
                ui.end_row();

                self.bouton_insert(ui, "0", "0", InsertKind::Digit);
                ui.label("");
                ui.label("");
                ui.label("");
                ui.end_row();
            });
    }

    /// Backspace “intelligent” : retire d’un coup les motifs utiles ("sin(", "pi", etc.).
    fn backspace_entree(&mut self) {
        if self.entree.is_empty() {
            return;
        }

        // Retire espaces finaux
        while self.entree.ends_with(' ') {
            self.entree.pop();
        }

        // Retire tokens connus
        for pat in ["sqrt(", "sin(", "cos(", "tan(", "pi"] {
            if self.entree.ends_with(pat) {
                for _ in 0..pat.chars().count() {
                    self.entree.pop();
                }
                while self.entree.ends_with(' ') {
                    self.entree.pop();
                }
                return;
            }
        }

        // Sinon : un caractère
        self.entree.pop();
        while self.entree.ends_with(' ') {
            self.entree.pop();
        }
    }

    fn ui_resultats(&mut self, ui: &mut egui::Ui) {
        ui.label("EXACT :");
        Self::champ_monospace(ui, "exact_out", &self.exact, 2);

        ui.add_space(6.0);

        ui.label("ΣLocal :");
        if self.lecture_dispo {
            Self::champ_monospace(ui, "socal_out", &self.lecture, 2);
        } else {
            ui.monospace("indisponible");
        }
    }

    fn ui_demarche(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Démarche")
            .default_open(true)
            .show(ui, |ui| {
                Self::champ_demarche(ui, "Jetons", "demarche_jetons", &self.demarche.jetons);
                Self::champ_demarche(ui, "RPN", "demarche_rpn", &self.demarche.rpn);
                Self::champ_demarche(ui, "Avant", "demarche_avant", &self.demarche.avant);
                Self::champ_demarche(ui, "Après", "demarche_apres", &self.demarche.apres);
                Self::champ_demarche(ui, "Note", "demarche_note", &self.demarche.note);
                Self::champ_demarche(ui, "Preuve", "demarche_preuve", &self.demarche.preuve);
            });
    }

    fn champ_demarche(ui: &mut egui::Ui, titre: &str, id: &str, contenu: &str) {
        ui.add_space(4.0);
        ui.label(format!("{titre} :"));
        Self::champ_monospace(ui, id, contenu, 2);
    }

    fn champ_monospace(ui: &mut egui::Ui, id: &str, contenu: &str, rows: usize) {
        // Affichage lecture seule “stable”, sans TextEdit interactif.
        // On garde un cadre visuel via Frame + Label monospace.
        egui::Frame::group(ui.style())
            .fill(ui.visuals().extreme_bg_color)
            .show(ui, |ui| {
                ui.push_id(id, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.set_min_height(
                        rows as f32 * ui.text_style_height(&egui::TextStyle::Monospace),
                    );
                    ui.monospace(contenu);
                });
            });
    }

    fn bouton_action(&mut self, ui: &mut egui::Ui, label: &str, tip: &str, action: Action) {
        let resp = ui
            .add_sized([56.0, 30.0], egui::Button::new(label))
            .on_hover_text(tip);

        if resp.clicked() {
            match action {
                Action::ClearEntree => self.clear_entree(),
                Action::ClearResultats => self.clear_resultats(),
                Action::ResetTotal => self.reset_total(),
                Action::Backspace => self.backspace_entree(),
            }
            self.focus_entree = true;
        }
    }

    fn bouton_insert(&mut self, ui: &mut egui::Ui, label: &str, to_insert: &str, kind: InsertKind) {
        let resp = ui.add_sized([46.0, 28.0], egui::Button::new(label));
        if !resp.clicked() || to_insert.is_empty() {
            return;
        }

        match kind {
            InsertKind::CloseParen => {
                while self.entree.ends_with(' ') {
                    self.entree.pop();
                }
                self.entree.push_str(to_insert);
            }
            InsertKind::OpenParen | InsertKind::Func => {
                if !self.entree.is_empty() {
                    let last = self.entree.chars().rev().find(|c| !c.is_whitespace());
                    if let Some(c) = last {
                        if c.is_ascii_digit() || c.is_ascii_alphabetic() || c == ')' {
                            self.entree.push(' ');
                        }
                    }
                }
                self.entree.push_str(to_insert);
            }
            InsertKind::Op => {
                while self.entree.ends_with(' ') {
                    self.entree.pop();
                }
                if !self.entree.is_empty() {
                    self.entree.push(' ');
                }
                self.entree.push_str(to_insert);
                self.entree.push(' ');
            }
            InsertKind::Digit => {
                // chiffres: pas d’espaces auto
                self.entree.push_str(to_insert);
            }
            InsertKind::Word => {
                // mots: espace si juste avant c’est un chiffre ou ')'
                if !self.entree.is_empty() && !self.entree.ends_with(char::is_whitespace) {
                    let last = self.entree.chars().rev().find(|c| !c.is_whitespace());
                    if let Some(c) = last {
                        if c.is_ascii_digit() || c == ')' {
                            self.entree.push(' ');
                        }
                    }
                }
                self.entree.push_str(to_insert);
            }
        }

        self.focus_entree = true;
    }

    /// Évalue l’expression via le noyau, puis dépose EXACT/ΣLocal/Démarche dans l’état UI.
    fn eval_via_noyau(&mut self) {
        let s = self.entree.trim();
        if s.is_empty() {
            self.set_erreur("Entrée vide");
            self.focus_entree = true;
            return;
        }

        match crate::noyau::eval_expression(s, self.digits) {
            Ok((exact, lecture_opt, d_noyau)) => {
                let d_ui = Demarche {
                    jetons: d_noyau.jetons,
                    rpn: d_noyau.rpn,
                    avant: d_noyau.avant,
                    apres: d_noyau.apres,
                    note: d_noyau.note,
                    preuve: d_noyau.preuve,
                };
                self.set_resultats(exact, lecture_opt, d_ui);
                self.focus_entree = true;
            }
            Err(msg) => {
                self.set_erreur(msg);
                self.focus_entree = true;
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Action {
    ClearEntree,
    ClearResultats,
    ResetTotal,
    Backspace,
}

#[derive(Clone, Copy, Debug)]
enum InsertKind {
    Digit,
    Word,
    Func,
    Op,
    OpenParen,
    CloseParen,
}
