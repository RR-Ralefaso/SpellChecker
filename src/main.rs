#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use spellchecker::gui::SpellCheckerApp;

fn main() -> Result<(), eframe::Error> {
    // Set up native options
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("AtomSpell - Atom IDE Inspired Spell Checker")
            .with_icon(
                // Load icon from assets
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icons/icon.png")[..])
                    .unwrap_or_else(|_e| {
                        // Fallback to a simple icon if file not found
                        eprintln!("Warning: Could not load icon.png");
                        eframe::icon_data::from_png_bytes(&[])
                            .unwrap_or_default()
                    })
            ),
        centered: true,
        ..Default::default()
    };
    
    eframe::run_native(
        "AtomSpell",
        options,
        Box::new(|cc| {
            // Restore previous state if available
            let _storage = cc.storage.unwrap();
            
            // Configure visuals with default dark theme
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
                    
                // Also add to proportional font for UI elements
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .push("FiraCode".to_owned());
            } else {
                eprintln!("Warning: Could not load FiraCode font. Using default font.");
            }
            
            cc.egui_ctx.set_fonts(fonts);
            
            // Create and return the app
            Box::new(SpellCheckerApp::new(cc))
        }),
    )
}