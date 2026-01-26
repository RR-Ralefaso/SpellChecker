use crate::editor::TextEditor;
use crate::sidebar::Sidebar;
use crate::theme::AtomTheme;
use crate::checker::SpellChecker;
use crate::language::{Language, LanguageManager};
use eframe::egui;
use rfd::FileDialog;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct AppState {
    pub current_file: Option<PathBuf>,
    pub document_content: String,
    pub is_document_modified: bool,
    pub auto_check: bool,
    pub show_line_numbers: bool,
    pub sidebar_width: f32,
    pub theme: AtomTheme,
    pub recent_files: Vec<PathBuf>,
    pub dictionary_paths: Vec<PathBuf>,
    pub selected_language: Language,
    pub auto_detect_language: bool,
    pub available_languages: Vec<Language>,
}

impl Default for AppState {
    fn default() -> Self {
        let language_manager = LanguageManager::new();
        
        Self {
            current_file: None,
            document_content: String::new(),
            is_document_modified: false,
            auto_check: true,
            show_line_numbers: true,
            sidebar_width: 300.0,
            theme: AtomTheme::OneDark,
            recent_files: Vec::new(),
            dictionary_paths: Vec::new(),
            selected_language: Language::English,
            auto_detect_language: true,
            available_languages: language_manager.available_languages().to_vec(),
        }
    }
}

pub struct SpellCheckerApp {
    state: AppState,
    text_editor: TextEditor,
    sidebar: Sidebar,
    spell_checker: Arc<SpellChecker>,
    last_check_time: Instant,
    check_interval: std::time::Duration,
    is_dragging_file: bool,
    drop_highlight: bool,
    stats: CheckStats,
    language_manager: LanguageManager,
}

#[derive(Default)]
struct CheckStats {
    total_words: usize,
    errors: usize,
    last_check_duration: std::time::Duration,
    detected_language: Option<Language>,
}

impl SpellCheckerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let state = AppState::default();
        let language_manager = LanguageManager::new();
        
        // Initialize spell checker with selected language
        let spell_checker = Arc::new(
            SpellChecker::new(state.selected_language)
                .expect("Failed to create spell checker"),
        );
        
        Self {
            state: state.clone(),
            text_editor: TextEditor::new(state.clone()),
            sidebar: Sidebar::new(state.clone(), Arc::clone(&spell_checker)),
            spell_checker,
            last_check_time: Instant::now(),
            check_interval: std::time::Duration::from_millis(1000),
            is_dragging_file: false,
            drop_highlight: false,
            stats: CheckStats::default(),
            language_manager,
        }
    }
    
    fn check_spelling(&mut self) {
        if self.state.auto_check && !self.state.document_content.is_empty() {
            let start_time = Instant::now();
            
            // Detect language if auto-detect is enabled
            let language_to_use = if self.state.auto_detect_language {
                let detected = self.language_manager.detect_language(&self.state.document_content);
                self.stats.detected_language = Some(detected);
                detected
            } else {
                self.state.selected_language
            };
            
            // Update spell checker language if changed
            if language_to_use != self.spell_checker.current_language() {
                if let Err(e) = self.spell_checker.set_language(language_to_use) {
                    eprintln!("Failed to change language: {}", e);
                }
            }
            
            let analysis = self.spell_checker.check_document(&self.state.document_content);
            self.stats.total_words = analysis.total_words;
            self.stats.errors = analysis.misspelled_words;
            self.stats.last_check_duration = start_time.elapsed();
            
            self.text_editor.set_analysis(analysis);
            self.last_check_time = Instant::now();
        }
    }
    
    fn open_file(&mut self, path: PathBuf) -> anyhow::Result<()> {
        let content = std::fs::read_to_string(&path)?;
        self.state.current_file = Some(path.clone());
        self.state.document_content = content;
        self.state.is_document_modified = false;
        
        // Add to recent files
        if !self.state.recent_files.contains(&path) {
            self.state.recent_files.insert(0, path);
            if self.state.recent_files.len() > 10 {
                self.state.recent_files.pop();
            }
        }
        
        // Auto-detect language from file content
        if self.state.auto_detect_language {
            let detected = self.language_manager.detect_language(&self.state.document_content);
            self.state.selected_language = detected;
            if let Err(e) = self.spell_checker.set_language(detected) {
                eprintln!("Failed to set language: {}", e);
            }
        }
        
        // Trigger spell check
        self.check_spelling();
        
        Ok(())
    }
    
    // ... rest of the existing methods remain the same ...
    
    fn show_language_selection(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Language selection combo box
            ui.label("🌍 Language:");
            
            egui::ComboBox::from_id_salt("language_combo")
                .selected_text(format!(
                    "{} {}",
                    self.state.selected_language.flag_emoji(),
                    self.state.selected_language.name()
                ))
                .show_ui(ui, |ui| {
                    for lang in &self.state.available_languages {
                        ui.selectable_value(
                            &mut self.state.selected_language,
                            *lang,
                            format!("{} {}", lang.flag_emoji(), lang.name()),
                        );
                    }
                });
            
            // Auto-detect checkbox
            ui.checkbox(&mut self.state.auto_detect_language, "Auto-detect");
            
            // Show detected language if auto-detect is on
            if self.state.auto_detect_language {
                if let Some(detected) = self.stats.detected_language {
                    ui.colored_label(
                        egui::Color32::LIGHT_BLUE,
                        format!("Detected: {}", detected.name()),
                    );
                }
            }
            
            // Dictionary info
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Ok(dict) = self.spell_checker.get_current_dictionary() {
                    ui.label(format!("📚 {} words", dict.word_count()));
                }
            });
        });
    }
    
    fn show_menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            // File menu
            ui.menu_button("File", |ui| {
                if ui.button("📂 Open File...").clicked() {
                    if let Some(path) = FileDialog::new()
                        .add_filter("Text files", &["txt", "md", "rs", "py", "js", "html", "css"])
                        .pick_file()
                    {
                        if let Err(e) = self.open_file(path) {
                            eprintln!("Failed to open file: {}", e);
                        }
                    }
                    ui.close_menu();
                }
                
                ui.separator();
                
                // Language submenu
                ui.menu_button("Language", |ui| {
                    for lang in &self.state.available_languages {
                        if ui
                            .selectable_value(
                                &mut self.state.selected_language,
                                *lang,
                                format!("{} {}", lang.flag_emoji(), lang.name()),
                            )
                            .clicked()
                        {
                            if let Err(e) = self.spell_checker.set_language(*lang) {
                                eprintln!("Failed to change language: {}", e);
                            }
                            self.state.auto_detect_language = false;
                            self.check_spelling();
                            ui.close_menu();
                        }
                    }
                    
                    ui.separator();
                    
                    ui.checkbox(&mut self.state.auto_detect_language, "Auto-detect language");
                });
                
                ui.separator();
                
                if ui.button("⚙️ Manage Dictionaries...").clicked() {
                    self.show_dictionary_manager = true;
                    ui.close_menu();
                }
                
                ui.separator();
                
                if ui.button("🚪 Exit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            
            // Edit menu
            ui.menu_button("Edit", |ui| {
                if ui.button("✏️ Check Spelling Now").clicked() {
                    self.check_spelling();
                    ui.close_menu();
                }
                
                ui.checkbox(&mut self.state.auto_check, "🔄 Auto-check");
                ui.checkbox(&mut self.state.show_line_numbers, "🔢 Show Line Numbers");
                
                ui.separator();
                
                if ui.button("🌐 Detect Language").clicked() {
                    let detected = self.language_manager.detect_language(&self.state.document_content);
                    self.state.selected_language = detected;
                    self.state.auto_detect_language = false;
                    if let Err(e) = self.spell_checker.set_language(detected) {
                        eprintln!("Failed to set language: {}", e);
                    }
                    self.check_spelling();
                    ui.close_menu();
                }
            });
            
            // Spacer
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Language selection in menu bar
                self.show_language_selection(ui);
                
                // Current file indicator
                if let Some(path) = &self.state.current_file {
                    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    ui.label(egui::RichText::new(filename).color(egui::Color32::GRAY));
                    
                    if self.state.is_document_modified {
                        ui.label(egui::RichText::new("●").color(egui::Color32::YELLOW));
                    }
                }
            });
        });
    }
    
    fn show_status_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            // Left side: Language info
            ui.horizontal(|ui| {
                ui.label(format!(
                    "{} {}",
                    self.state.selected_language.flag_emoji(),
                    self.state.selected_language.name()
                ));
                
                if self.state.auto_detect_language {
                    if let Some(detected) = self.stats.detected_language {
                        if detected != self.state.selected_language {
                            ui.colored_label(
                                egui::Color32::LIGHT_BLUE,
                                format!("(Detected: {})", detected.name()),
                            );
                        }
                    }
                }
            });
            
            // Center: Spell check stats
            ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("Words: {}", self.stats.total_words));
                    
                    if self.stats.errors > 0 {
                        ui.colored_label(
                            egui::Color32::RED,
                            format!("Errors: {}", self.stats.errors),
                        );
                    } else if self.stats.total_words > 0 {
                        ui.colored_label(
                            egui::Color32::GREEN,
                            "✓ No errors",
                        );
                    }
                    
                    ui.label(format!("({:.2}ms)", self.stats.last_check_duration.as_secs_f64() * 1000.0));
                });
            });
            
            // Right side: Status indicators
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.state.auto_check {
                    ui.colored_label(egui::Color32::GREEN, "🔄 Auto");
                }
                
                if let Ok(dict) = self.spell_checker.get_current_dictionary() {
                    ui.label(format!("📚 {} words", dict.word_count()));
                }
            });
        });
    }
    
    // ... rest of the file remains the same ...
}