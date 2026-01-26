#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use spell_checker_gui::SpellCheckerApp;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("AtomSpell - Atom IDE Inspired Spell Checker")
            .with_icon(
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icons/icon.png")[..])
                    .unwrap(),
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
            fonts.font_data.insert(
                "FiraCode".to_owned(),
                egui::FontData::from_static(include_bytes!("../assets/fonts/FiraCode-Regular.ttf")),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "FiraCode".to_owned());
            cc.egui_ctx.set_fonts(fonts);
            
            Box::new(SpellCheckerApp::new(cc))
        }),
    )
}