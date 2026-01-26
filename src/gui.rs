use crate::checker::{DocumentAnalysis, SpellChecker};
use crate::editor::TextEditor;
use crate::language::{Language, LanguageManager};
use crate::sidebar::Sidebar;
use crate::theme::AtomTheme;
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
    pub show_dictionary_manager: bool,
    pub font_size: f32,
    pub wrap_text: bool,
    pub show_whitespace: bool,
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
            show_dictionary_manager: false,
            font_size: 14.0,
            wrap_text: true,
            show_whitespace: false,
        }
    }
}

pub struct SpellCheckerApp {
    state: AppState,
    text_editor: TextEditor,
    sidebar: Sidebar,
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
}

#[derive(Default)]
struct CheckStats {
    total_words: usize,
    errors: usize,
    last_check_duration: std::time::Duration,
    detected_language: Option<Language>,
}

impl SpellCheckerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let state = AppState::default();
        let language_manager = LanguageManager::new();
        
        let spell_checker = Arc::new(
            std::sync::Mutex::new(
                SpellChecker::new(state.selected_language)
                    .unwrap_or_else(|e| {
                        eprintln!("Failed to create spell checker: {}", e);
                        SpellChecker::new(Language::English).unwrap()
                    })
            )
        );
        
        let mut text_editor = TextEditor::new();
        text_editor.set_font_size(state.font_size);
        
        Self {
            state: state.clone(),
            text_editor,
            sidebar: Sidebar::new(),
            spell_checker,
            last_check_time: Instant::now(),
            check_interval: std::time::Duration::from_millis(1000),
            is_dragging_file: false,
            drop_highlight: false,
            stats: CheckStats::default(),
            language_manager,
            analysis: None,
            pending_add_word: None,
            pending_ignore_word: None,
            pending_replace: None,
        }
    }
    
    fn check_spelling(&mut self) {
        if !self.state.auto_check || self.state.document_content.is_empty() {
            return;
        }
        
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
        if language_to_use != self.spell_checker.lock().unwrap().current_language() {
            if let Ok(mut checker) = self.spell_checker.lock() {
                if let Err(e) = checker.set_language(language_to_use) {
                    eprintln!("Failed to change language: {}", e);
                }
            }
        }
        
        let checker = self.spell_checker.lock().unwrap();
        self.analysis = Some(checker.check_document(&self.state.document_content));
        if let Some(analysis) = &self.analysis {
            self.stats.total_words = analysis.total_words;
            self.stats.errors = analysis.misspelled_words;
            self.stats.last_check_duration = start_time.elapsed();
            
            self.text_editor.set_analysis(analysis.clone());
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
            if let Ok(mut checker) = self.spell_checker.lock() {
                if let Err(e) = checker.set_language(detected) {
                    eprintln!("Failed to set language: {}", e);
                }
            }
        }
        
        // Trigger spell check
        self.check_spelling();
        
        Ok(())
    }
    
    fn save_file(&mut self) -> anyhow::Result<()> {
        if let Some(path) = &self.state.current_file {
            std::fs::write(path, &self.state.document_content)?;
            self.state.is_document_modified = false;
        }
        Ok(())
    }
    
    fn save_as(&mut self) -> anyhow::Result<()> {
        if let Some(path) = FileDialog::new()
            .add_filter("Text files", &["txt", "md", "rs", "py", "js", "html", "css"])
            .set_file_name(
                self.state
                    .current_file
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("document.txt"),
            )
            .save_file()
        {
            std::fs::write(&path, &self.state.document_content)?;
            self.state.current_file = Some(path);
            self.state.is_document_modified = false;
        }
        Ok(())
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
                    eprintln!("Failed to open dropped file: {}", e);
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
    
    fn show_menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
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
                
                if ui.button("📁 Open Folder...").clicked() {
                    if let Some(path) = FileDialog::new().pick_folder() {
                        println!("Selected folder: {:?}", path);
                    }
                    ui.close_menu();
                }
                
                ui.separator();
                
                if ui.button("💾 Save").clicked() {
                    if let Err(e) = self.save_file() {
                        eprintln!("Failed to save file: {}", e);
                    }
                    ui.close_menu();
                }
                
                if ui.button("💾 Save As...").clicked() {
                    if let Err(e) = self.save_as() {
                        eprintln!("Failed to save file: {}", e);
                    }
                    ui.close_menu();
                }
                
                ui.separator();
                
                // Make a copy of recent files to avoid borrowing issues
                let recent_files = self.state.recent_files.clone();
                if !recent_files.is_empty() {
                    ui.menu_button("Recent Files", |ui| {
                        for path in &recent_files {
                            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                                if ui.button(format!("📄 {}", filename)).clicked() {
                                    if let Err(e) = self.open_file(path.clone()) {
                                        eprintln!("Failed to open file: {}", e);
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
                    if let Ok(mut checker) = self.spell_checker.lock() {
                        if let Err(e) = checker.set_language(detected) {
                            eprintln!("Failed to set language: {}", e);
                        }
                    }
                    self.check_spelling();
                    ui.close_menu();
                }
                
                ui.separator();
                
                if ui.button("🧹 Clear Document").clicked() {
                    self.state.document_content.clear();
                    self.state.is_document_modified = true;
                    ui.close_menu();
                }
            });
            
            ui.menu_button("View", |ui| {
                ui.menu_button("Theme", |ui| {
                    for theme in AtomTheme::all() {
                        if ui
                            .selectable_value(&mut self.state.theme, theme, theme.name())
                            .clicked()
                        {
                            self.state.theme.apply(ui.ctx());
                            ui.close_menu();
                        }
                    }
                });
                
                ui.separator();
                
                ui.checkbox(&mut self.sidebar.visible(), "📁 Sidebar");
                
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
                
                ui.checkbox(&mut self.state.wrap_text, "Wrap Text");
                ui.checkbox(&mut self.state.show_whitespace, "Show Whitespace");
            });
            
            ui.menu_button("Language", |ui| {
                // Make a copy of available languages to avoid borrowing issues
                let available_languages = self.state.available_languages.clone();
                let mut selected_language = self.state.selected_language;
                let mut auto_detect = self.state.auto_detect_language;
                
                for lang in &available_languages {
                    if ui
                        .selectable_value(
                            &mut selected_language,
                            *lang,
                            format!("{} {}", lang.flag_emoji(), lang.name()),
                        )
                        .clicked()
                    {
                        self.state.selected_language = selected_language;
                        self.state.auto_detect_language = false;
                        if let Ok(mut checker) = self.spell_checker.lock() {
                            if let Err(e) = checker.set_language(*lang) {
                                eprintln!("Failed to change language: {}", e);
                            }
                        }
                        self.check_spelling();
                        ui.close_menu();
                    }
                }
                
                ui.separator();
                
                if ui.checkbox(&mut auto_detect, "Auto-detect language").changed() {
                    self.state.auto_detect_language = auto_detect;
                }
            });
            
            ui.menu_button("Tools", |ui| {
                if ui.button("⚙️ Add Dictionary...").clicked() {
                    if let Some(path) = FileDialog::new()
                        .add_filter("Dictionary files", &["txt", "dict"])
                        .pick_file()
                    {
                        // Ask for language code
                        // TODO: Implement dialog for language code input
                        println!("Would add dictionary: {:?}", path);
                    }
                    ui.close_menu();
                }
                
                ui.separator();
                
                if ui.button("📊 Statistics Report").clicked() {
                    // TODO: Implement statistics report
                    ui.close_menu();
                }
                
                if ui.button("📤 Export Errors...").clicked() {
                    // TODO: Implement error export
                    ui.close_menu();
                }
            });
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.show_language_selection(ui);
                
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
    
    fn show_language_selection(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("🌍");
            
            egui::ComboBox::from_id_source("language_combo")
                .selected_text(format!(
                    "{} {}",
                    self.state.selected_language.flag_emoji(),
                    self.state.selected_language.name()
                ))
                .show_ui(ui, |ui: &mut egui::Ui| {
                    // Make a copy to avoid borrowing issues
                    let available_languages = self.state.available_languages.clone();
                    let mut selected_language = self.state.selected_language;
                    
                    for lang in &available_languages {
                        if ui.selectable_value(
                            &mut selected_language,
                            *lang,
                            format!("{} {}", lang.flag_emoji(), lang.name()),
                        ).clicked() {
                            self.state.selected_language = selected_language;
                            self.state.auto_detect_language = false;
                            if let Ok(mut checker) = self.spell_checker.lock() {
                                if let Err(e) = checker.set_language(*lang) {
                                    eprintln!("Failed to change language: {}", e);
                                }
                            }
                            self.check_spelling();
                        }
                    }
                });
            
            ui.checkbox(&mut self.state.auto_detect_language, "Auto");
        });
    }
    
    fn show_status_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
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
                    
                    if self.stats.last_check_duration.as_millis() > 0 {
                        ui.label(format!("({}ms)", self.stats.last_check_duration.as_millis()));
                    }
                });
            });
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.state.auto_check {
                    ui.colored_label(egui::Color32::GREEN, "🔄 Auto");
                }
                
                let word_count = self.spell_checker.lock().unwrap().word_count();
                ui.label(format!("Dict: {}", word_count));
                
                if self.state.auto_detect_language {
                    if let Some(detected) = self.stats.detected_language {
                        if detected != self.state.selected_language {
                            ui.colored_label(
                                egui::Color32::LIGHT_BLUE,
                                format!("({})", detected.name()),
                            );
                        }
                    }
                }
            });
        });
    }
    
    fn show_main_content(&mut self, ui: &mut egui::Ui) {
        if self.sidebar.visible() {
            egui::SidePanel::left("sidebar")
                .resizable(true)
                .default_width(self.state.sidebar_width)
                .width_range(200.0..=500.0)
                .show_inside(ui, |ui| {
                    let mut pending_add_word = None;
                    let mut pending_ignore_word = None;
                    let mut pending_replace = None;
                    
                    self.sidebar.show(
                        ui,
                        &self.spell_checker.lock().unwrap(),
                        &self.analysis,
                        &self.state.document_content,
                        &mut pending_add_word,
                        &mut pending_ignore_word,
                        &mut pending_replace,
                    );
                    
                    // Handle pending actions
                    if let Some(word) = pending_add_word {
                        if let Ok(mut checker) = self.spell_checker.lock() {
                            if let Err(e) = checker.add_word_to_dictionary(&word) {
                                eprintln!("Failed to add word: {}", e);
                            }
                        }
                        self.check_spelling();
                    }
                    
                    if let Some(word) = pending_ignore_word {
                        // TODO: Implement ignore word functionality
                        println!("Ignore word: {}", word);
                    }
                    
                    if let Some((find, replace)) = pending_replace {
                        // TODO: Implement replace
                        println!("Replace: {} -> {}", find, replace);
                    }
                });
        }
        
        egui::CentralPanel::default().show_inside(ui, |ui| {
            // Drop zone highlight
            if self.drop_highlight {
                let rect = ui.available_rect_before_wrap();
                ui.painter().rect_filled(
                    rect,
                    5.0,
                    egui::Color32::from_rgba_unmultiplied(100, 149, 237, 50),
                );
                
                ui.centered_and_justified(|ui| {
                    ui.heading("📂 Drop file here");
                    ui.label("Release to open the file");
                });
            }
            
            // Text editor
            let editor_response = self.text_editor.show(
                ui,
                &mut self.state.document_content,
                &mut self.state.is_document_modified,
                self.state.show_line_numbers,
            );
            
            // Check spelling if content changed
            if editor_response.changed && self.state.auto_check {
                self.check_spelling();
            }
        });
    }
}

impl eframe::App for SpellCheckerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle file drop
        self.handle_file_drop(ctx);
        
        // Apply theme
        self.state.theme.apply(ctx);
        
        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            self.show_menu_bar(ui);
        });
        
        // Status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            self.show_status_bar(ui);
        });
        
        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_main_content(ui);
        });
        
        // Check for auto-check interval
        if self.state.auto_check && self.last_check_time.elapsed() > self.check_interval {
            self.check_spelling();
        }
        
        // Request repaint if needed
        ctx.request_repaint();
    }
    
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        // Save application state if needed
    }
}