// src/main.rs
//
// Calculatrice Q-pur — point d’entrée NATIF + WEB (WASM)
// ------------------------------------------------------
// But:
// - NATIF (Linux/Windows/macOS) : eframe::run_native + NativeOptions
// - WEB  (wasm32)              : eframe::WebRunner + WebOptions + <canvas>
// - Polices embarquées         : anti “carrés” (Unicode / symboles)
//
// Côté WEB (WASM) : ton index.html doit contenir un canvas :
//   <canvas id="the_canvas_id"></canvas>
//
// IMPORTANT (structure projet):
// - `impl eframe::App for AppCalc` doit vivre dans src/app.rs (recommandé)
// - Ici: point d’entrée seulement (natif + web)

#![cfg_attr(target_arch = "wasm32", allow(unused_imports))]

use eframe::egui;

mod app;
mod noyau;

use app::AppCalc;

/// Titre unique (natif + web).
const TITRE_APP: &str = "Calculatrice Q-pur";

/* ------------------------ Polices (natif + web) ------------------------ */

fn installer_polices(ctx: &egui::Context) {
    use egui::{FontData, FontDefinitions, FontFamily};

    let mut fonts = FontDefinitions::default();

    // Polices embarquées (anti-“carrés” garanti)
    fonts.font_data.insert(
        "dejavu_sans".to_string(),
        FontData::from_static(include_bytes!("../assets/fonts/DejaVuSans.ttf")).into(),
    );
    fonts.font_data.insert(
        "dejavu_mono".to_string(),
        FontData::from_static(include_bytes!("../assets/fonts/DejaVuSansMono.ttf")).into(),
    );

    // Proportional (titres/labels)
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "dejavu_sans".to_string());

    // Monospace (zones EXACT/ΣLocal/Démarche)
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "dejavu_mono".to_string());

    ctx.set_fonts(fonts);
}

/* ------------------------ Entrée NATIF (PC) ------------------------ */

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(TITRE_APP)
            .with_inner_size([520.0, 740.0])
            .with_min_inner_size([420.0, 620.0]),
        ..Default::default()
    };

    eframe::run_native(
        TITRE_APP,
        options,
        Box::new(|cc| {
            // Contexte egui prêt => polices avant la première frame.
            installer_polices(&cc.egui_ctx);
            Ok(Box::<AppCalc>::default())
        }),
    )
}

/* ------------------------ Entrée WEB (WASM) ------------------------ */

#[cfg(target_arch = "wasm32")]
fn main() {
    // En wasm32, le démarrage réel passe par `start()` (wasm_bindgen).
    // On laisse main() vide pour rester clair.
}

#[cfg(target_arch = "wasm32")]
mod web {
    use super::{installer_polices, AppCalc, TITRE_APP};

    use wasm_bindgen::JsCast;
    use web_sys::{window, HtmlCanvasElement};

    /// ID du canvas attendu dans index.html.
    const CANVAS_ID: &str = "the_canvas_id";

    /// Point d’entrée automatique au chargement de la page.
    /// - Fixe le titre de l’onglet (document.title)
    /// - Récupère le <canvas id="the_canvas_id">
    /// - Démarre eframe WebRunner dessus
    #[wasm_bindgen::prelude::wasm_bindgen(start)]
    pub async fn start() -> Result<(), wasm_bindgen::JsValue> {
        // 1) window/document
        let w = window().ok_or_else(|| js_err("window() indisponible"))?;
        let d = w
            .document()
            .ok_or_else(|| js_err("document() indisponible"))?;

        // 1.5) Titre onglet (utilise TITRE_APP => plus de warning)
        d.set_title(TITRE_APP);

        // 2) element by id
        let el = d
            .get_element_by_id(CANVAS_ID)
            .ok_or_else(|| js_err("canvas introuvable (id incorrect dans index.html)"))?;

        // 3) cast -> HtmlCanvasElement
        let canvas: HtmlCanvasElement = el
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| js_err("l’élément trouvé n’est pas un <canvas>"))?;

        // 4) run web
        let web_options = eframe::WebOptions::default();

        eframe::WebRunner::new()
            .start(
                canvas, // ✅ canvas DOM réel
                web_options,
                Box::new(|cc| {
                    installer_polices(&cc.egui_ctx);
                    Ok(Box::<AppCalc>::default())
                }),
            )
            .await
    }

    fn js_err(msg: &str) -> wasm_bindgen::JsValue {
        wasm_bindgen::JsValue::from_str(msg)
    }
}

