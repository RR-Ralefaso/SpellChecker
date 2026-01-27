#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use spellchecker::gui::SpellCheckerApp;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("AtomSpell - IDE-Inspired Multilingual Spell Checker"),
        centered: true,
        ..Default::default()
    };
    
    eframe::run_native(
        "AtomSpell",
        options,
        Box::new(|cc| {
            // Set dark theme by default
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            
            // Try to load custom font
            let mut fonts = egui::FontDefinitions::default();
            
            // Try common font locations
            let font_paths = [
                "assets/fonts/FiraCode-Regular.ttf",
                "./assets/fonts/FiraCode-Regular.ttf",
                "../assets/fonts/FiraCode-Regular.ttf",
                "FiraCode-Regular.ttf"
            ];
            
            for font_path in font_paths {
                if let Ok(font_data) = std::fs::read(font_path) {
                    fonts.font_data.insert(
                        "FiraCode".to_owned(),
                        egui::FontData::from_owned(font_data),
                    );
                    
                    fonts.families.insert(
                        egui::FontFamily::Monospace,
                        vec!["FiraCode".to_owned()]
                    );
                    
                    println!("Loaded FiraCode font from: {}", font_path);
                    break;
                }
            }
            
            cc.egui_ctx.set_fonts(fonts);
            
            Box::new(SpellCheckerApp::new(cc))
        }),
    )
}