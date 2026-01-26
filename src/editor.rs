use crate::checker::{DocumentAnalysis, WordCheck};
use crate::theme::AtomTheme;
use eframe::egui;
use std::collections::HashMap;

#[derive(Clone)]
pub struct TextEditor {
    analysis: Option<DocumentAnalysis>,
    scroll_offset: f32,
    cursor_pos: usize,
    selection: Option<(usize, usize)>,
    line_height: f32,
    font_size: f32,
    show_whitespace: bool,
    wrap_lines: bool,
    error_positions: HashMap<usize, WordCheck>,
}

impl TextEditor {
    pub fn new() -> Self {
        Self {
            analysis: None,
            scroll_offset: 0.0,
            cursor_pos: 0,
            selection: None,
            line_height: 24.0,
            font_size: 14.0,
            show_whitespace: false,
            wrap_lines: true,
            error_positions: HashMap::new(),
        }
    }
    
    pub fn set_analysis(&mut self, analysis: DocumentAnalysis) {
        self.analysis = Some(analysis.clone());
        
        // Build error positions map for quick lookup
        self.error_positions.clear();
        for word in analysis.words {
            if !word.is_correct {
                self.error_positions.insert(word.start, word);
            }
        }
    }
    
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        content: &mut String,
        modified: &mut bool,
        show_line_numbers: bool,
    ) -> egui::Response {
        let available_width = ui.available_width();
        let available_height = ui.available_height();
        
        // Create scroll area
        let scroll_area = egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .scroll_offset(vec2(0.0, self.scroll_offset))
            .max_height(available_height);
        
        scroll_area.show(ui, |ui| {
            // Calculate line count
            let line_count = content.lines().count().max(1);
            
            // Create painter for custom rendering
            let painter = ui.painter();
            let rect = ui.available_rect_before_wrap();
            
            // Draw background
            painter.rect_filled(rect, 0.0, ui.visuals().window_fill);
            
            // Calculate text layout
            let font_id = egui::FontId::new(self.font_size, egui::FontFamily::Proportional);
            let galley = ui.fonts(|f| {
                f.layout_no_wrap(
                    content.clone(),
                    font_id.clone(),
                    ui.visuals().text_color(),
                    available_width,
                )
            });
            
            // Draw line numbers if enabled
            if show_line_numbers {
                let line_number_width = 60.0;
                let line_number_rect = rect.split_left_right_at_width(line_number_width).0;
                
                painter.rect_filled(
                    line_number_rect,
                    0.0,
                    ui.visuals().faint_bg_color,
                );
                
                for line in 1..=line_count {
                    let y_pos = (line - 1) as f32 * self.line_height;
                    let line_number_text = format!("{:>4}", line);
                    
                    painter.text(
                        pos2(line_number_rect.left() + 10.0, line_number_rect.top() + y_pos + self.line_height * 0.75),
                        egui::Align2::LEFT_CENTER,
                        line_number_text,
                        font_id.clone(),
                        ui.visuals().weak_text_color(),
                    );
                }
                
                // Draw separator
                painter.line_segment(
                    [
                        pos2(line_number_rect.right(), rect.top()),
                        pos2(line_number_rect.right(), rect.bottom()),
                    ],
                    (1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
                );
            }
            
            // Draw text content with highlighting
            let text_start_x = if show_line_numbers { 70.0 } else { 10.0 };
            let mut char_pos = 0;
            
            for (line_idx, line) in content.lines().enumerate() {
                let y_pos = line_idx as f32 * self.line_height;
                let mut x_pos = text_start_x;
                
                for (word_start, word) in self.extract_words_with_positions(line) {
                    let word_end = word_start + word.len();
                    let word_text = &line[word_start..word_end];
                    
                    // Check if this word has an error
                    let is_error = if let Some(analysis) = &self.analysis {
                        analysis.words.iter().any(|w| 
                            w.line == line_idx + 1 && 
                            w.word == word_text
                        )
                    } else {
                        false
                    };
                    
                    // Draw word with appropriate color
                    let color = if is_error {
                        ui.visuals().error_fg_color
                    } else {
                        ui.visuals().text_color()
                    };
                    
                    painter.text(
                        pos2(x_pos, rect.top() + y_pos + self.line_height * 0.75),
                        egui::Align2::LEFT_CENTER,
                        word_text,
                        font_id.clone(),
                        color,
                    );
                    
                    // Underline errors
                    if is_error {
                        let word_width = self.measure_text_width(word_text, &font_id, ui);
                        painter.line_segment(
                            [
                                pos2(x_pos, rect.top() + y_pos + self.line_height),
                                pos2(x_pos + word_width, rect.top() + y_pos + self.line_height),
                            ],
                            (2.0, ui.visuals().error_fg_color),
                        );
                    }
                    
                    x_pos += self.measure_text_width(word_text, &font_id, ui) + 
                           self.measure_text_width(" ", &font_id, ui);
                }
                
                // Draw line break if not last line
                if line_idx < line_count - 1 {
                    painter.text(
                        pos2(x_pos, rect.top() + y_pos + self.line_height * 0.75),
                        egui::Align2::LEFT_CENTER,
                        "",
                        font_id.clone(),
                        ui.visuals().text_color(),
                    );
                }
                
                char_pos += line.len() + 1; // +1 for newline
            }
            
            // Draw cursor
            let cursor_y = (self.get_cursor_line(content) as f32) * self.line_height;
            let cursor_x = self.get_cursor_x(content, &font_id, ui, text_start_x);
            
            painter.line_segment(
                [
                    pos2(cursor_x, rect.top() + cursor_y),
                    pos2(cursor_x, rect.top() + cursor_y + self.line_height),
                ],
                (2.0, ui.visuals().text_color()),
            );
            
            // Handle text input
            let response = ui.add(
                egui::TextEdit::multiline(content)
                    .desired_width(f32::INFINITY)
                    .desired_rows((available_height / self.line_height) as usize)
                    .font(font_id)
                    .frame(false),
            );
            
            if response.changed() {
                *modified = true;
            }
            
            // Update cursor position based on interaction
            if response.has_focus() {
                if let Some(mut state) = response.ctx.memory(|mem| mem.data.get_temp::<egui::text_edit::TextEditState>(response.id)) {
                    self.cursor_pos = state.cursor.primary.ccursor.index;
                    state.store(ui.ctx(), response.id);
                }
            }
            
            response
        }).inner
    }
    
    fn extract_words_with_positions(&self, text: &str) -> Vec<(usize, String)> {
        let mut words = Vec::new();
        let mut current_word = String::new();
        let mut current_pos = 0;
        let mut word_start = 0;
        
        for (i, ch) in text.char_indices() {
            if ch.is_alphanumeric() || ch == '\'' || ch == '-' {
                if current_word.is_empty() {
                    word_start = i;
                }
                current_word.push(ch);
            } else if !current_word.is_empty() {
                words.push((word_start, current_word.clone()));
                current_word.clear();
            }
        }
        
        if !current_word.is_empty() {
            words.push((word_start, current_word));
        }
        
        words
    }
    
    fn measure_text_width(&self, text: &str, font_id: &egui::FontId, ui: &egui::Ui) -> f32 {
        ui.fonts(|f| f.glyph_width(font_id, text))
    }
    
    fn get_cursor_line(&self, content: &str) -> usize {
        let chars_before = content.chars().take(self.cursor_pos).count();
        let line = content.chars().take(chars_before).filter(|&c| c == '\n').count();
        line
    }
    
    fn get_cursor_x(&self, content: &str, font_id: &egui::FontId, ui: &egui::Ui, text_start_x: f32) -> f32 {
        let line_start = content
            .chars()
            .take(self.cursor_pos)
            .collect::<String>()
            .rfind('\n')
            .map(|pos| pos + 1)
            .unwrap_or(0);
        
        let line_text = content
            .chars()
            .skip(line_start)
            .take(self.cursor_pos - line_start)
            .collect::<String>();
        
        text_start_x + self.measure_text_width(&line_text, font_id, ui)
    }
    
    pub fn set_font_size(&mut self, size: f32) {
        self.font_size = size;
        self.line_height = size * 1.5;
    }
    
    pub fn toggle_wrap_lines(&mut self) {
        self.wrap_lines = !self.wrap_lines;
    }
    
    pub fn toggle_whitespace(&mut self) {
        self.show_whitespace = !self.show_whitespace;
    }
    
    pub fn get_error_at_cursor(&self) -> Option<&WordCheck> {
        if let Some(analysis) = &self.analysis {
            analysis.words.iter().find(|w| !w.is_correct && w.start <= self.cursor_pos && w.end >= self.cursor_pos)
        } else {
            None
        }
    }
}

// Helper functions
fn pos2(x: f32, y: f32) -> egui::Pos2 {
    egui::pos2(x, y)
}

fn vec2(x: f32, y: f32) -> egui::Vec2 {
    egui::vec2(x, y)
}