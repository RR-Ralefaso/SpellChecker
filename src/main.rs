#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use spellchecker::SpellCheckerApp;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("AtomSpell - Atom IDE Inspired Spell Checker")
            .with_icon(
                // Note: You'll need to create an icon file or remove this line
                // eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icons/icon.png")[..])
                //     .unwrap(),
            ),
        ..Default::default()
    };
    
    eframe::run_native(
        "AtomSpell",
        options,
        Box::new(|cc| {
            // Configure visuals
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            
            // Load custom font for Atom-like typography
            let mut fonts = egui::FontDefinitions::default();
            
            // Try to load FiraCode, fall back to default if not available
            if let Ok(font_data) = std::fs::read("assets/fonts/FiraCode-Regular.ttf") {
                fonts.font_data.insert(
                    "FiraCode".to_owned(),
                    egui::FontData::from_owned(font_data),
                );
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, "FiraCode".to_owned());
            }
            
            cc.egui_ctx.set_fonts(fonts);
            
            Box::new(SpellCheckerApp::new(cc))
        }),
    )
}