use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AtomTheme {
    OneDark,
    OneLight,
    SolarizedDark,
    SolarizedLight,
    Monokai,
    Dracula,
}

impl AtomTheme {
    pub fn all() -> Vec<AtomTheme> {
        vec![
            AtomTheme::OneDark,
            AtomTheme::OneLight,
            AtomTheme::SolarizedDark,
            AtomTheme::SolarizedLight,
            AtomTheme::Monokai,
            AtomTheme::Dracula,
        ]
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            AtomTheme::OneDark => "One Dark",
            AtomTheme::OneLight => "One Light",
            AtomTheme::SolarizedDark => "Solarized Dark",
            AtomTheme::SolarizedLight => "Solarized Light",
            AtomTheme::Monokai => "Monokai",
            AtomTheme::Dracula => "Dracula",
        }
    }
    
    pub fn apply(&self, ctx: &egui::Context) {
        let visuals = match self {
            AtomTheme::OneDark => egui::Visuals::dark(),
            AtomTheme::OneLight => egui::Visuals::light(),
            AtomTheme::SolarizedDark => solarized_dark(),
            AtomTheme::SolarizedLight => solarized_light(),
            AtomTheme::Monokai => monokai(),
            AtomTheme::Dracula => dracula(),
        };
        ctx.set_visuals(visuals);
    }
}

fn solarized_dark() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();
    visuals.window_fill = egui::Color32::from_rgb(0, 43, 54); // base03
    visuals.panel_fill = egui::Color32::from_rgb(7, 54, 66);  // base02
    visuals.faint_bg_color = egui::Color32::from_rgb(88, 110, 117); // base01
    visuals.extreme_bg_color = egui::Color32::from_rgb(0, 43, 54); // base03
    visuals.code_bg_color = egui::Color32::from_rgb(7, 54, 66); // base02
    visuals.warn_fg_color = egui::Color32::from_rgb(181, 137, 0); // yellow
    visuals.error_fg_color = egui::Color32::from_rgb(220, 50, 47); // red
    visuals.hyperlink_color = egui::Color32::from_rgb(38, 139, 210); // blue
    visuals
}

fn solarized_light() -> egui::Visuals {
    let mut visuals = egui::Visuals::light();
    visuals.window_fill = egui::Color32::from_rgb(253, 246, 227); // base3
    visuals.panel_fill = egui::Color32::from_rgb(238, 232, 213); // base2
    visuals.faint_bg_color = egui::Color32::from_rgb(147, 161, 161); // base1
    visuals.extreme_bg_color = egui::Color32::from_rgb(253, 246, 227); // base3
    visuals.code_bg_color = egui::Color32::from_rgb(238, 232, 213); // base2
    visuals.warn_fg_color = egui::Color32::from_rgb(181, 137, 0); // yellow
    visuals.error_fg_color = egui::Color32::from_rgb(220, 50, 47); // red
    visuals.hyperlink_color = egui::Color32::from_rgb(38, 139, 210); // blue
    visuals
}

fn monokai() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();
    visuals.window_fill = egui::Color32::from_rgb(39, 40, 34);
    visuals.panel_fill = egui::Color32::from_rgb(39, 40, 34);
    visuals.faint_bg_color = egui::Color32::from_rgb(73, 72, 62);
    visuals.extreme_bg_color = egui::Color32::from_rgb(39, 40, 34);
    visuals.code_bg_color = egui::Color32::from_rgb(73, 72, 62);
    visuals.warn_fg_color = egui::Color32::from_rgb(249, 238, 152);
    visuals.error_fg_color = egui::Color32::from_rgb(249, 38, 114);
    visuals.hyperlink_color = egui::Color32::from_rgb(102, 217, 239);
    visuals
}

fn dracula() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();
    visuals.window_fill = egui::Color32::from_rgb(40, 42, 54);
    visuals.panel_fill = egui::Color32::from_rgb(40, 42, 54);
    visuals.faint_bg_color = egui::Color32::from_rgb(68, 71, 90);
    visuals.extreme_bg_color = egui::Color32::from_rgb(40, 42, 54);
    visuals.code_bg_color = egui::Color32::from_rgb(68, 71, 90);
    visuals.warn_fg_color = egui::Color32::from_rgb(241, 250, 140);
    visuals.error_fg_color = egui::Color32::from_rgb(255, 85, 85);
    visuals.hyperlink_color = egui::Color32::from_rgb(139, 233, 253);
    visuals
}