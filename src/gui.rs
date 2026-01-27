use crate::checker::{DocumentAnalysis, SpellChecker};
use crate::editor::TextEditor;
use crate::language::{Language, LanguageManager};
use crate::sidebar::Sidebar;
use crate::theme::AtomTheme;
use crate::{open_repository, open_sponsor_page};
use eframe::egui;
use rfd::FileDialog;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct AppState {
    pub current_file: Option<PathBuf>,
    pub document_content: String,
    pub is_document_modified: bool,
    pub auto_check: bool,
    pub show_line_numbers: bool,
    pub sidebar_width: f32,
    pub theme: AtomTheme,
    pub recent_files: Vec<PathBuf>,
    pub selected_language: Language,
    pub auto_detect_language: bool,
    pub font_size: f32,
    pub wrap_text: bool,
    pub show_whitespace: bool,
    pub last_directory: Option<PathBuf>,
    pub sidebar_state: Sidebar,
    pub show_about: bool,
    pub show_settings: bool,
    pub enable_syntax_highlighting: bool,
    pub check_interval_ms: u64,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_file: None,
            document_content: String::new(),
            is_document_modified: false,
            auto_check: true,
            show_line_numbers: true,
            sidebar_width: 300.0,
            theme: AtomTheme::OneDark,
            recent_files: Vec::new(),
            selected_language: Language::English,
            auto_detect_language: true,
            font_size: 14.0,
            wrap_text: true,
            show_whitespace: false,
            last_directory: None,
            sidebar_state: Sidebar::new(),
            show_about: false,
            show_settings: false,
            enable_syntax_highlighting: true,
            check_interval_ms: 1500,
        }
    }
}

pub struct SpellCheckerApp {
    state: AppState,
    text_editor: TextEditor,
    spell_checker: Arc<std::sync::Mutex<SpellChecker>>,
    last_check_time: Instant,
    check_interval: std::time::Duration,
    is_dragging_file: bool,
    drop_highlight: bool,
    stats: CheckStats,
    language_manager: LanguageManager,
    analysis: Option<DocumentAnalysis>,
    pending_add_word: Option<String>,
    pending_ignore_word: Option<String>,
    pending_replace: Option<(String, String)>,
    pending_import_dict: bool,
    pending_export_dict: bool,
    pending_clear_ignored: bool,
    last_spell_check: Option<DocumentAnalysis>,
    show_notification: Option<(String, egui::Color32)>,
    notification_timer: Instant,
}

#[derive(Default)]
struct CheckStats {
    total_words: usize,
    errors: usize,
    last_check_duration: std::time::Duration,
    detected_language: Option<Language>,
    check_count: usize,
    total_characters: usize,
    total_lines: usize,
}

impl SpellCheckerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let state: AppState = cc.storage
            .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY))
            .unwrap_or_default();
        
        state.theme.apply(&cc.egui_ctx);
        
        let language_manager = LanguageManager::new();
        
        let spell_checker = match SpellChecker::new(state.selected_language) {
            Ok(checker) => Arc::new(std::sync::Mutex::new(checker)),
            Err(e) => {
                eprintln!("Failed to create spell checker: {}", e);
                Arc::new(std::sync::Mutex::new(
                    SpellChecker::new(Language::English).unwrap()
                ))
            }
        };
        
        let mut text_editor = TextEditor::new();
        text_editor.set_font_size(state.font_size);
        text_editor.set_wrap_lines(state.wrap_text);
        text_editor.set_show_whitespace(state.show_whitespace);
        
        Self {
            state: state.clone(),
            text_editor,
            spell_checker,
            last_check_time: Instant::now(),
            check_interval: std::time::Duration::from_millis(state.check_interval_ms),
            is_dragging_file: false,
            drop_highlight: false,
            stats: CheckStats::default(),
            language_manager,
            analysis: None,
            pending_add_word: None,
            pending_ignore_word: None,
            pending_replace: None,
            pending_import_dict: false,
            pending_export_dict: false,
            pending_clear_ignored: false,
            last_spell_check: None,
            show_notification: None,
            notification_timer: Instant::now(),
        }
    }
    
    fn check_spelling(&mut self) {
        if !self.state.auto_check || self.state.document_content.trim().is_empty() {
            return;
        }
        
        let start_time = Instant::now();
        
        let language_to_use = if self.state.auto_detect_language {
            let detected = self.language_manager.detect_language(&self.state.document_content);
            self.stats.detected_language = Some(detected);
            detected
        } else {
            self.state.selected_language
        };
        
        if language_to_use != self.spell_checker.lock().unwrap().current_language() {
            if let Ok(mut checker) = self.spell_checker.lock() {
                if checker.set_language(language_to_use).is_ok() {
                    self.state.selected_language = language_to_use;
                }
            }
        }
        
        let filename = self.state.current_file
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str());
        
        let analysis = {
            let checker = self.spell_checker.lock().unwrap();
            checker.check_document(&self.state.document_content, filename)
        };
        
        self.analysis = Some(analysis.clone());
        self.stats.total_words = analysis.total_words;
        self.stats.errors = analysis.misspelled_words;
        self.stats.last_check_duration = start_time.elapsed();
        self.stats.check_count += 1;
        self.stats.total_characters = self.state.document_content.chars().count();
        self.stats.total_lines = self.state.document_content.lines().count();
        
        self.text_editor.set_analysis(analysis.clone());
        self.last_spell_check = Some(analysis);
        self.last_check_time = Instant::now();
    }
    
    fn open_file(&mut self, path: PathBuf) -> anyhow::Result<()> {
        let content = std::fs::read_to_string(&path)?;
        self.state.current_file = Some(path.clone());
        self.state.document_content = content;
        self.state.is_document_modified = false;
        
        if let Some(parent) = path.parent() {
            self.state.last_directory = Some(parent.to_path_buf());
        }
        
        if !self.state.recent_files.contains(&path) {
            self.state.recent_files.insert(0, path);
            if self.state.recent_files.len() > 10 {
                self.state.recent_files.pop();
            }
        }
        
        if self.state.auto_detect_language {
            let detected = self.language_manager.detect_language(&self.state.document_content);
            self.state.selected_language = detected;
            if let Ok(mut checker) = self.spell_checker.lock() {
                let _ = checker.set_language(detected);
            }
        }
        
        self.check_spelling();
        
        Ok(())
    }
    
    fn save_file(&mut self) -> anyhow::Result<()> {
        if let Some(path) = &self.state.current_file {
            std::fs::write(path, &self.state.document_content)?;
            self.state.is_document_modified = false;
            self.show_notification("File saved successfully".to_string(), egui::Color32::GREEN);
        } else {
            self.save_as()?;
        }
        Ok(())
    }
    
    fn save_as(&mut self) -> anyhow::Result<()> {
        let default_name = self.state
            .current_file
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("document.txt");
        
        if let Some(path) = FileDialog::new()
            .add_filter("Text files", &["txt", "md", "rs", "py", "js", "html", "css"])
            .set_file_name(default_name)
            .set_directory(self.state.last_directory.clone().unwrap_or_else(|| PathBuf::from(".")))
            .save_file()
        {
            std::fs::write(&path, &self.state.document_content)?;
            self.state.current_file = Some(path);
            self.state.is_document_modified = false;
            self.show_notification("File saved successfully".to_string(), egui::Color32::GREEN);
        }
        Ok(())
    }
    
    fn show_notification(&mut self, message: String, color: egui::Color32) {
        self.show_notification = Some((message, color));
        self.notification_timer = Instant::now();
    }
    
    fn handle_file_drop(&mut self, ctx: &egui::Context) {
        if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
            self.is_dragging_file = true;
        } else {
            self.is_dragging_file = false;
        }
        
        if ctx.input(|i| i.raw.dropped_files.len() > 0) {
            if let Some(file) = ctx.input(|i| i.raw.dropped_files[0].path.clone()) {
                if let Err(e) = self.open_file(file) {
                    self.show_notification(format!("Failed to open file: {}", e), egui::Color32::RED);
                }
            }
            self.drop_highlight = false;
        }
        
        if ctx.input(|i| i.pointer.any_down()) && self.is_dragging_file {
            self.drop_highlight = true;
        } else {
            self.drop_highlight = false;
        }
    }
    
    fn handle_pending_actions(&mut self) {
        // Create a scope to drop the mutex guard before showing notifications
        if let Some(word) = self.pending_add_word.take() {
            let result = {
                let mut checker = self.spell_checker.lock().unwrap();
                checker.add_word_to_dictionary(&word)
            };
            
            if result.is_ok() {
                self.show_notification(format!("Added '{}' to dictionary", word), egui::Color32::GREEN);
            }
            self.check_spelling();
        }
        
        if let Some(word) = self.pending_ignore_word.take() {
            let result = {
                let mut checker = self.spell_checker.lock().unwrap();
                checker.ignore_word(&word)
            };
            
            if result.is_ok() {
                self.show_notification(format!("Ignored '{}' for this session", word), egui::Color32::YELLOW);
            }
            self.check_spelling();
        }
        
        if let Some((find, replace)) = self.pending_replace.take() {
            if !find.is_empty() {
                self.state.document_content = self.state.document_content.replace(&find, &replace);
                self.state.is_document_modified = true;
                self.check_spelling();
                self.show_notification(format!("Replaced '{}' with '{}'", find, replace), egui::Color32::GREEN);
            }
        }
        
        if self.pending_import_dict {
            self.pending_import_dict = false;
            if let Some(path) = FileDialog::new()
                .add_filter("Dictionary files", &["txt", "dict"])
                .set_directory(self.state.last_directory.clone().unwrap_or_else(|| PathBuf::from(".")))
                .pick_file()
            {
                let result = {
                    let mut checker = self.spell_checker.lock().unwrap();
                    checker.import_dictionary(&path)
                };
                
                if let Err(e) = result {
                    self.show_notification(format!("Failed to import: {}", e), egui::Color32::RED);
                } else {
                    self.show_notification("Dictionary imported successfully".to_string(), egui::Color32::GREEN);
                }
                self.check_spelling();
            }
        }
        
        if self.pending_export_dict {
            self.pending_export_dict = false;
            let default_name = format!("dictionary_{}.txt", self.state.selected_language.code());
            if let Some(path) = FileDialog::new()
                .add_filter("Text files", &["txt"])
                .set_file_name(&default_name)
                .set_directory(self.state.last_directory.clone().unwrap_or_else(|| PathBuf::from(".")))
                .save_file()
            {
                let result = {
                    let checker = self.spell_checker.lock().unwrap();
                    checker.export_dictionary(&path)
                };
                
                if let Err(e) = result {
                    self.show_notification(format!("Failed to export: {}", e), egui::Color32::RED);
                } else {
                    self.show_notification("Dictionary exported successfully".to_string(), egui::Color32::GREEN);
                }
            }
        }
        
        if self.pending_clear_ignored {
            self.pending_clear_ignored = false;
            {
                let mut checker = self.spell_checker.lock().unwrap();
                checker.clear_ignored_words();
            }
            self.check_spelling();
            self.show_notification("Cleared ignored words".to_string(), egui::Color32::GREEN);
        }
    }
    
    fn show_about_dialog(&mut self, ctx: &egui::Context) {
        let mut show_about = self.state.show_about;
        
        egui::Window::new("About AtomSpell")
            .open(&mut show_about)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("🔤 AtomSpell Spell Checker");
                    ui.label("An Atom IDE-inspired multilingual spell checker");
                    ui.separator();
                    
                    ui.label(format!("Version: {}", env!("CARGO_PKG_VERSION")));
                    ui.label("Author: Rothang Ralph Ralefaso");
                    ui.label("Email: rrralefaso@outlook.com");
                    ui.label("GitHub: https://github.com/RR-Ralefaso/SpellChecker");
                    ui.separator();
                    
                    ui.label("License: MIT");
                    ui.separator();
                    
                    ui.horizontal(|ui| {
                        ui.label("❤️ Support this project:");
                        if ui.button("Become a Sponsor").clicked() {
                            let _ = open_sponsor_page();
                        }
                    });
                    
                    ui.horizontal(|ui| {
                        if ui.button("View on GitHub").clicked() {
                            let _ = open_repository();
                        }
                        
                        if ui.button("Close").clicked() {
                            self.state.show_about = false;
                        }
                    });
                });
            });
            
        self.state.show_about = show_about;
    }
    
    fn show_settings_dialog(&mut self, ctx: &egui::Context) {
        let mut show_settings = self.state.show_settings;
        
        egui::Window::new("Settings")
            .open(&mut show_settings)
            .resizable(true)
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.heading("Application Settings");
                ui.separator();
                
                egui::Grid::new("settings_grid")
                    .num_columns(2)
                    .spacing([20.0, 10.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Auto-check:");
                        ui.checkbox(&mut self.state.auto_check, "");
                        ui.end_row();
                        
                        ui.label("Check interval (ms):");
                        ui.add(egui::DragValue::new(&mut self.state.check_interval_ms)
                            .clamp_range(500..=10000)
                            .suffix("ms"));
                        ui.end_row();
                        
                        ui.label("Auto-detect language:");
                        ui.checkbox(&mut self.state.auto_detect_language, "");
                        ui.end_row();
                        
                        ui.label("Show line numbers:");
                        ui.checkbox(&mut self.state.show_line_numbers, "");
                        ui.end_row();
                        
                        ui.label("Wrap text:");
                        ui.checkbox(&mut self.state.wrap_text, "");
                        ui.end_row();
                        
                        ui.label("Show whitespace:");
                        ui.checkbox(&mut self.state.show_whitespace, "");
                        ui.end_row();
                        
                        ui.label("Syntax highlighting:");
                        ui.checkbox(&mut self.state.enable_syntax_highlighting, "");
                        ui.end_row();
                        
                        ui.label("Font size:");
                        ui.add(egui::DragValue::new(&mut self.state.font_size)
                            .clamp_range(8.0..=36.0)
                            .speed(0.5));
                        ui.end_row();
                    });
                
                ui.separator();
                
                ui.horizontal(|ui| {
                    if ui.button("Reset to Defaults").clicked() {
                        // Create a default state instead of trying to reconstruct the app
                        self.state = AppState::default();
                        self.text_editor.set_font_size(self.state.font_size);
                        self.text_editor.set_wrap_lines(self.state.wrap_text);
                        self.check_interval = std::time::Duration::from_millis(self.state.check_interval_ms);
                    }
                    
                    if ui.button("Save").clicked() {
                        self.state.show_settings = false;
                        self.text_editor.set_font_size(self.state.font_size);
                        self.text_editor.set_wrap_lines(self.state.wrap_text);
                        self.check_interval = std::time::Duration::from_millis(self.state.check_interval_ms);
                    }
                    
                    if ui.button("Cancel").clicked() {
                        self.state.show_settings = false;
                    }
                });
            });
            
        self.state.show_settings = show_settings;
    }
    
    fn show_menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("📂 Open File...").clicked() {
                    if let Some(path) = FileDialog::new()
                        .add_filter("Text files", &["txt", "md", "rs", "py", "js", "html", "css"])
                        .set_directory(self.state.last_directory.clone().unwrap_or_else(|| PathBuf::from(".")))
                        .pick_file()
                    {
                        if let Err(e) = self.open_file(path) {
                            self.show_notification(format!("Failed to open file: {}", e), egui::Color32::RED);
                        }
                    }
                    ui.close_menu();
                }
                
                ui.separator();
                
                if ui.button("💾 Save").clicked() {
                    if let Err(e) = self.save_file() {
                        self.show_notification(format!("Failed to save: {}", e), egui::Color32::RED);
                    }
                    ui.close_menu();
                }
                
                if ui.button("💾 Save As...").clicked() {
                    if let Err(e) = self.save_as() {
                        self.show_notification(format!("Failed to save: {}", e), egui::Color32::RED);
                    }
                    ui.close_menu();
                }
                
                ui.separator();
                
                if !self.state.recent_files.is_empty() {
                    ui.menu_button("Recent Files", |ui| {
                        let recent_files = self.state.recent_files.clone();
                        for path in &recent_files {
                            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                                if ui.button(format!("📄 {}", filename)).clicked() {
                                    let path_clone = path.clone();
                                    if let Err(e) = self.open_file(path_clone) {
                                        self.show_notification(format!("Failed to open: {}", e), egui::Color32::RED);
                                    }
                                    ui.close_menu();
                                }
                            }
                        }
                    });
                }
                
                ui.separator();
                
                if ui.button("🚪 Exit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            
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
                    {
                        let mut checker = self.spell_checker.lock().unwrap();
                        let _ = checker.set_language(detected);
                    }
                    self.check_spelling();
                    ui.close_menu();
                }
                
                ui.separator();
                
                if ui.button("🧹 Clear Document").clicked() {
                    self.state.document_content.clear();
                    self.state.is_document_modified = true;
                    self.analysis = None;
                    ui.close_menu();
                }
            });
            
            ui.menu_button("View", |ui| {
                ui.menu_button("Theme", |ui| {
                    for theme in AtomTheme::all() {
                        if ui.selectable_value(&mut self.state.theme, theme, theme.name()).clicked() {
                            self.state.theme.apply(ui.ctx());
                            ui.close_menu();
                        }
                    }
                });
                
                ui.separator();
                
                ui.checkbox(&mut self.state.sidebar_state.visible, "📁 Sidebar");
                ui.checkbox(&mut self.state.show_line_numbers, "🔢 Line Numbers");
                ui.checkbox(&mut self.state.wrap_text, "📝 Wrap Text");
                ui.checkbox(&mut self.state.show_whitespace, "␣ Show Whitespace");
                
                ui.separator();
                
                ui.menu_button("Text Size", |ui| {
                    if ui.button("Smaller").clicked() && self.state.font_size > 8.0 {
                        self.state.font_size -= 1.0;
                        self.text_editor.set_font_size(self.state.font_size);
                    }
                    if ui.button("Reset").clicked() {
                        self.state.font_size = 14.0;
                        self.text_editor.set_font_size(self.state.font_size);
                    }
                    if ui.button("Larger").clicked() && self.state.font_size < 36.0 {
                        self.state.font_size += 1.0;
                        self.text_editor.set_font_size(self.state.font_size);
                    }
                });
            });
            
            ui.menu_button("Language", |ui| {
                let available_languages = self.language_manager.available_languages().to_vec();
                let mut selected_language = self.state.selected_language;
                
                for lang in &available_languages {
                    if ui.selectable_value(
                        &mut selected_language,
                        *lang,
                        format!("{} {}", lang.flag_emoji(), lang.name()),
                    ).clicked() {
                        self.state.selected_language = selected_language;
                        self.state.auto_detect_language = false;
                        {
                            let mut checker = self.spell_checker.lock().unwrap();
                            let _ = checker.set_language(*lang);
                        }
                        self.check_spelling();
                        ui.close_menu();
                    }
                }
                
                ui.separator();
                
                ui.checkbox(&mut self.state.auto_detect_language, "🌐 Auto-detect language");
            });
            
            ui.menu_button("Tools", |ui| {
                if ui.button("⚙️ Settings").clicked() {
                    self.state.show_settings = true;
                    ui.close_menu();
                }
                
                ui.separator();
                
                if ui.button("📥 Import Dictionary...").clicked() {
                    self.pending_import_dict = true;
                    ui.close_menu();
                }
                
                if ui.button("📤 Export Dictionary...").clicked() {
                    self.pending_export_dict = true;
                    ui.close_menu();
                }
                
                ui.separator();
                
                if ui.button("📊 Statistics Report").clicked() {
                    // TODO: Implement statistics report export
                    ui.close_menu();
                }
            });
            
            ui.menu_button("Help", |ui| {
                if ui.button("ℹ️ About AtomSpell").clicked() {
                    self.state.show_about = true;
                    ui.close_menu();
                }
                
                if ui.button("📖 Documentation").clicked() {
                    let _ = open_repository();
                    ui.close_menu();
                }
                
                ui.separator();
                
                if ui.button("❤️ Sponsor This Project").clicked() {
                    let _ = open_sponsor_page();
                    ui.close_menu();
                }
            });
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.show_language_selection(ui);
                
                if let Some(path) = &self.state.current_file {
                    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    ui.label(egui::RichText::new(filename).color(egui::Color32::GRAY));
                    
                    if self.state.is_document_modified {
                        ui.colored_label(egui::Color32::YELLOW, "●");
                    }
                }
            });
        });
    }
    
    fn show_language_selection(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("🌍");
            
            egui::ComboBox::from_id_source("language_combo")
                .selected_text(format!(
                    "{} {}",
                    self.state.selected_language.flag_emoji(),
                    self.state.selected_language.name()
                ))
                .show_ui(ui, |ui| {
                    let available_languages = self.language_manager.available_languages().to_vec();
                    let mut selected_language = self.state.selected_language;
                    
                    for lang in &available_languages {
                        if ui.selectable_value(
                            &mut selected_language,
                            *lang,
                            format!("{} {}", lang.flag_emoji(), lang.name()),
                        ).clicked() {
                            self.state.selected_language = selected_language;
                            self.state.auto_detect_language = false;
                            {
                                let mut checker = self.spell_checker.lock().unwrap();
                                let _ = checker.set_language(*lang);
                            }
                            self.check_spelling();
                        }
                    }
                });
            
            ui.checkbox(&mut self.state.auto_detect_language, "Auto");
        });
    }
    
    fn show_status_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(path) = &self.state.current_file {
                let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("Untitled");
                ui.label(format!("📄 {}", filename));
            } else {
                ui.label("📄 Untitled");
            }
            
            if self.state.is_document_modified {
                ui.colored_label(egui::Color32::YELLOW, "(modified)");
            }
        });
        
        ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("📝 Words: {}", self.stats.total_words));
                ui.label(format!("📊 Lines: {}", self.stats.total_lines));
                ui.label(format!("🔤 Chars: {}", self.stats.total_characters));
                
                if self.stats.errors > 0 {
                    ui.colored_label(
                        egui::Color32::RED,
                        format!("❌ Errors: {}", self.stats.errors),
                    );
                } else if self.stats.total_words > 0 {
                    ui.colored_label(
                        egui::Color32::GREEN,
                        "✅ No errors",
                    );
                }
                
                if self.stats.last_check_duration.as_millis() > 0 {
                    ui.label(format!("⚡ {}ms", self.stats.last_check_duration.as_millis()));
                }
            });
        });
        
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if self.state.auto_check {
                ui.colored_label(egui::Color32::GREEN, "🔄 Auto");
            }
            
            let word_count = {
                let checker = self.spell_checker.lock().unwrap();
                checker.word_count()
            };
            ui.label(format!("📚 Dict: {}", word_count));
            
            if self.state.auto_detect_language {
                if let Some(detected) = self.stats.detected_language {
                    if detected != self.state.selected_language {
                        ui.colored_label(
                            egui::Color32::LIGHT_BLUE,
                            format!("🌐 ({})", detected.name()),
                        );
                    }
                }
            }
        });
    }
    
    fn show_notification_overlay(&self, ui: &mut egui::Ui) {
        if let Some((message, color)) = &self.show_notification {
            if self.notification_timer.elapsed() < std::time::Duration::from_secs(3) {
                let rect = ui.max_rect();
                let notification_rect = egui::Rect::from_center_size(
                    egui::pos2(rect.center().x, rect.top() + 50.0),
                    egui::vec2(300.0, 40.0),
                );
                
                ui.painter().rect_filled(
                    notification_rect,
                    10.0,
                    ui.visuals().panel_fill,
                );
                
                ui.painter().rect_stroke(
                    notification_rect,
                    10.0,
                    egui::Stroke::new(1.0, *color),
                );
                
                ui.painter().text(
                    notification_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    message,
                    egui::FontId::proportional(14.0),
                    *color,
                );
            }
        }
    }
    
    fn show_main_content(&mut self, ui: &mut egui::Ui) {
        if self.state.sidebar_state.visible {
            egui::SidePanel::left("sidebar")
                .resizable(true)
                .default_width(self.state.sidebar_width)
                .width_range(200.0..=500.0)
                .show_inside(ui, |ui| {
                    let checker = self.spell_checker.lock().unwrap();
                    self.state.sidebar_state.show(
                        ui,
                        &checker,
                        &self.analysis,
                        &self.state.document_content,
                        &mut self.pending_add_word,
                        &mut self.pending_ignore_word,
                        &mut self.pending_replace,
                        &mut self.pending_import_dict,
                        &mut self.pending_export_dict,
                        &mut self.pending_clear_ignored,
                    );
                });
        }
        
        egui::CentralPanel::default().show_inside(ui, |ui| {
            if self.drop_highlight {
                let rect = ui.available_rect_before_wrap();
                ui.painter().rect_filled(
                    rect,
                    10.0,
                    egui::Color32::from_rgba_unmultiplied(100, 149, 237, 50),
                );
                
                ui.centered_and_justified(|ui| {
                    ui.heading("📂 Drop file here");
                    ui.label("Release to open the file");
                });
            }
            
            let editor_response = self.text_editor.show(
                ui,
                &mut self.state.document_content,
                &mut self.state.is_document_modified,
                self.state.show_line_numbers,
                &self.analysis,
            );
            
            if editor_response.changed && self.state.auto_check {
                self.check_spelling();
            }
            
            self.show_notification_overlay(ui);
        });
    }
}

impl eframe::App for SpellCheckerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_pending_actions();
        self.handle_file_drop(ctx);
        self.state.theme.apply(ctx);
        
        if self.state.show_about {
            self.show_about_dialog(ctx);
        }
        
        if self.state.show_settings {
            self.show_settings_dialog(ctx);
        }
        
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            self.show_menu_bar(ui);
        });
        
        // Show status bar in a bottom panel
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            self.show_status_bar(ui);
        });
        
        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_main_content(ui);
        });
        
        if self.state.auto_check && self.last_check_time.elapsed() > self.check_interval {
            self.check_spelling();
        }
        
        ctx.request_repaint();
    }
    
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.state);
    }
    
    fn auto_save_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(30)
    }
    
    fn persist_egui_memory(&self) -> bool {
        true
    }
}