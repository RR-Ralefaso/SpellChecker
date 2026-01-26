use crate::checker::{DocumentAnalysis, SpellChecker};
use eframe::egui;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Sidebar {
    pub show_dictionary: bool,
    pub show_errors: bool,
    pub show_stats: bool,
    pub show_find: bool,
    pub show_replace: bool,
    pub selected_error_index: usize,
    pub find_text: String,
    pub replace_text: String,
    pub case_sensitive_find: bool,
    pub whole_word_find: bool,
    pub visible: bool,
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}

impl Sidebar {
    pub fn new() -> Self {
        Self {
            show_dictionary: true,
            show_errors: false,
            show_stats: false,
            show_find: false,
            show_replace: false,
            selected_error_index: 0,
            find_text: String::new(),
            replace_text: String::new(),
            case_sensitive_find: false,
            whole_word_find: false,
            visible: true,
        }
    }
    
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        spell_checker: &SpellChecker,
        analysis: &Option<DocumentAnalysis>,
        content: &str,
        on_add_word: &mut Option<String>,
        on_ignore_word: &mut Option<String>,
        on_replace: &mut Option<(String, String)>,
        on_import_dict: &mut bool,
        on_export_dict: &mut bool,
        on_clear_ignored: &mut bool,
    ) {
        ui.vertical(|ui| {
            // Tabs for different sidebar views
            ui.horizontal(|ui| {
                if ui.selectable_label(self.show_dictionary, "📚 Dictionary").clicked() {
                    self.reset_tabs();
                    self.show_dictionary = true;
                }
                
                if ui.selectable_label(self.show_errors, "❌ Errors").clicked() {
                    self.reset_tabs();
                    self.show_errors = true;
                }
                
                if ui.selectable_label(self.show_stats, "📊 Stats").clicked() {
                    self.reset_tabs();
                    self.show_stats = true;
                }
                
                if ui.selectable_label(self.show_find, "🔍 Find").clicked() {
                    self.reset_tabs();
                    self.show_find = true;
                }
                
                if ui.selectable_label(self.show_replace, "🔄 Replace").clicked() {
                    self.reset_tabs();
                    self.show_replace = true;
                }
            });
            
            ui.separator();
            
            // Show selected view
            if self.show_dictionary {
                self.show_dictionary_view(ui, spell_checker, on_add_word, on_ignore_word, 
                    on_import_dict, on_export_dict, on_clear_ignored);
            } else if self.show_errors {
                self.show_errors_view(ui, analysis, on_replace);
            } else if self.show_stats {
                self.show_stats_view(ui, analysis, spell_checker);
            } else if self.show_find {
                self.show_find_view(ui, content);
            } else if self.show_replace {
                self.show_replace_view(ui, content, on_replace);
            }
        });
    }
    
    fn reset_tabs(&mut self) {
        self.show_dictionary = false;
        self.show_errors = false;
        self.show_stats = false;
        self.show_find = false;
        self.show_replace = false;
    }
    
    fn show_dictionary_view(
        &mut self,
        ui: &mut egui::Ui,
        spell_checker: &SpellChecker,
        on_add_word: &mut Option<String>,
        on_ignore_word: &mut Option<String>,
        on_import_dict: &mut bool,
        on_export_dict: &mut bool,
        on_clear_ignored: &mut bool,
    ) {
        ui.heading("Dictionary");
        
        // Language info
        ui.horizontal(|ui| {
            ui.label("Language:");
            ui.label(spell_checker.current_language().name());
            ui.label(spell_checker.current_language().flag_emoji());
        });
        
        ui.horizontal(|ui| {
            ui.label("Words in dictionary:");
            ui.label(format!("{}", spell_checker.word_count()));
        });
        
        ui.separator();
        
        // Add word section
        ui.heading("Add Word to Dictionary");
        ui.horizontal(|ui| {
            let mut new_word = String::new();
            let response = ui.text_edit_singleline(&mut new_word);
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !new_word.is_empty() {
                *on_add_word = Some(new_word.clone());
            }
            if ui.button("Add").clicked() && !new_word.is_empty() {
                *on_add_word = Some(new_word.clone());
            }
        });
        ui.label("Adds word to user dictionary for current language");
        
        ui.separator();
        
        // Ignore word section
        ui.heading("Ignore Word (Session Only)");
        ui.horizontal(|ui| {
            let mut ignore_word = String::new();
            let response = ui.text_edit_singleline(&mut ignore_word);
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !ignore_word.is_empty() {
                *on_ignore_word = Some(ignore_word.clone());
            }
            if ui.button("Ignore").clicked() && !ignore_word.is_empty() {
                *on_ignore_word = Some(ignore_word.clone());
            }
        });
        ui.label("Ignores word only for current session");
        
        ui.separator();
        
        // Dictionary actions
        ui.heading("Dictionary Management");
        ui.horizontal_wrapped(|ui| {
            if ui.button("📥 Import Dictionary").clicked() {
                *on_import_dict = true;
            }
            if ui.button("📤 Export Dictionary").clicked() {
                *on_export_dict = true;
            }
            if ui.button("🗑️ Clear Ignored Words").clicked() {
                *on_clear_ignored = true;
            }
        });
        
        ui.separator();
        
        // Help text
        ui.label("ℹ️ Note: Added words are saved to user dictionary files.");
        ui.label("Ignored words are only for the current session.");
    }
    
    fn show_errors_view(
        &mut self,
        ui: &mut egui::Ui,
        analysis: &Option<DocumentAnalysis>,
        on_replace: &mut Option<(String, String)>,
    ) {
        ui.heading("Spelling Errors");
        
        if let Some(analysis) = analysis {
            if analysis.misspelled_words == 0 {
                ui.colored_label(egui::Color32::GREEN, "✓ No spelling errors found!");
                return;
            }
            
            // Error list
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (idx, word) in analysis.words.iter().filter(|w| !w.is_correct).enumerate() {
                    let is_selected = idx == self.selected_error_index;
                    
                    ui.horizontal(|ui| {
                        // Error indicator
                        ui.colored_label(egui::Color32::RED, "✗");
                        
                        // Error word
                        if ui.selectable_label(is_selected, &word.word).clicked() {
                            self.selected_error_index = idx;
                        }
                        
                        // Line info
                        ui.label(format!("(L{}:C{})", word.line, word.column));
                    });
                    
                    // Suggestions
                    if !word.suggestions.is_empty() {
                        ui.indent("suggestions", |ui| {
                            ui.label("Suggestions:");
                            for suggestion in &word.suggestions {
                                ui.horizontal(|ui| {
                                    if ui.button("Use").clicked() {
                                        *on_replace = Some((word.word.clone(), suggestion.clone()));
                                    }
                                    ui.label(suggestion);
                                });
                            }
                        });
                    }
                    
                    ui.separator();
                }
            });
            
            // Error count
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(format!("Total errors: {}", analysis.misspelled_words));
                if analysis.misspelled_words > 0 {
                    if ui.button("▶️ Fix All with First Suggestion").clicked() {
                        // TODO: Implement fix all
                        ui.label("Feature coming soon...");
                    }
                }
            });
        } else {
            ui.label("No document loaded or checked.");
        }
    }
    
    fn show_stats_view(
        &mut self,
        ui: &mut egui::Ui,
        analysis: &Option<DocumentAnalysis>,
        spell_checker: &SpellChecker,
    ) {
        ui.heading("Document Statistics");
        
        if let Some(analysis) = analysis {
            // Accuracy gauge
            ui.horizontal(|ui| {
                ui.label("Accuracy:");
                let gauge = egui::widgets::ProgressBar::new(analysis.accuracy / 100.0)
                    .show_percentage()
                    .desired_width(150.0);
                ui.add(gauge);
            });
            
            ui.separator();
            
            // Stats grid
            egui::Grid::new("stats_grid")
                .num_columns(2)
                .spacing([10.0, 5.0])
                .show(ui, |ui| {
                    ui.label("Total words:");
                    ui.label(format!("{}", analysis.total_words));
                    ui.end_row();
                    
                    ui.label("Misspelled:");
                    ui.colored_label(egui::Color32::RED, format!("{}", analysis.misspelled_words));
                    ui.end_row();
                    
                    ui.label("Accuracy:");
                    ui.label(format!("{:.1}%", analysis.accuracy));
                    ui.end_row();
                    
                    ui.label("Suggestions:");
                    ui.label(format!("{}", analysis.suggestions_count));
                    ui.end_row();
                    
                    ui.label("Lines checked:");
                    ui.label(format!("{}", analysis.lines_checked));
                    ui.end_row();
                    
                    ui.label("Language:");
                    ui.label(format!("{} {}", 
                        analysis.language.flag_emoji(),
                        analysis.language.name()
                    ));
                    ui.end_row();
                    
                    ui.label("Dictionary size:");
                    ui.label(format!("{} words", spell_checker.word_count()));
                    ui.end_row();
                });
            
            // Reading time
            if analysis.total_words > 0 {
                ui.separator();
                let minutes = analysis.total_words / 200;
                let seconds = ((analysis.total_words % 200) * 60) / 200;
                ui.label(format!("Estimated reading time: {} min {} sec", minutes, seconds));
            }
        } else {
            ui.label("No statistics available. Load a document first.");
        }
    }
    
    fn show_find_view(&mut self, ui: &mut egui::Ui, content: &str) {
        ui.heading("Find in Document");
        
        ui.horizontal(|ui| {
            ui.label("Find:");
            ui.text_edit_singleline(&mut self.find_text);
            
            if ui.button("🔍").clicked() && !self.find_text.is_empty() {
                // Find functionality would be implemented in the editor
            }
        });
        
        ui.checkbox(&mut self.case_sensitive_find, "Case sensitive");
        ui.checkbox(&mut self.whole_word_find, "Whole word");
        
        if !self.find_text.is_empty() {
            let count = if self.case_sensitive_find {
                content.matches(&self.find_text).count()
            } else {
                content.to_lowercase().matches(&self.find_text.to_lowercase()).count()
            };
            
            if count > 0 {
                ui.colored_label(egui::Color32::GREEN, format!("Found {} occurrences", count));
            } else {
                ui.colored_label(egui::Color32::RED, "No matches found");
            }
        }
    }
    
    fn show_replace_view(&mut self, ui: &mut egui::Ui, content: &str, on_replace: &mut Option<(String, String)>) {
        ui.heading("Find and Replace");
        
        ui.horizontal(|ui| {
            ui.label("Find:");
            ui.text_edit_singleline(&mut self.find_text);
        });
        
        ui.horizontal(|ui| {
            ui.label("Replace:");
            ui.text_edit_singleline(&mut self.replace_text);
        });
        
        ui.checkbox(&mut self.case_sensitive_find, "Case sensitive");
        ui.checkbox(&mut self.whole_word_find, "Whole word");
        
        ui.horizontal(|ui| {
            if ui.button("Replace").clicked() && !self.find_text.is_empty() {
                *on_replace = Some((self.find_text.clone(), self.replace_text.clone()));
            }
            
            if ui.button("Replace All").clicked() && !self.find_text.is_empty() {
                // TODO: Implement replace all
                ui.label("Replace All coming soon...");
            }
        });
        
        if !self.find_text.is_empty() {
            let count = if self.case_sensitive_find {
                content.matches(&self.find_text).count()
            } else {
                content.to_lowercase().matches(&self.find_text.to_lowercase()).count()
            };
            ui.label(format!("Found {} occurrences", count));
        }
    }
    
    pub fn visible(&self) -> bool {
        self.visible
    }
    
    pub fn toggle_visible(&mut self) {
        self.visible = !self.visible;
    }
    
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }
}