use crate::checker::{DocumentAnalysis, WordCheck};
use eframe::egui;

#[derive(Clone)]
pub struct TextEditor {
    line_height: f32,
    font_size: f32,
    show_whitespace: bool,
    wrap_lines: bool,
    char_width_cache: std::collections::HashMap<char, f32>,
    error_cache: std::collections::HashMap<usize, WordCheck>,
    last_analysis: Option<DocumentAnalysis>,
}

impl Default for TextEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl TextEditor {
    pub fn new() -> Self {
        Self {
            line_height: 24.0,
            font_size: 14.0,
            show_whitespace: false,
            wrap_lines: true,
            char_width_cache: std::collections::HashMap::new(),
            error_cache: std::collections::HashMap::new(),
            last_analysis: None,
        }
    }
    
    pub fn set_analysis(&mut self, analysis: DocumentAnalysis) {
        self.last_analysis = Some(analysis.clone());
        
        // Build error cache for quick lookup
        self.error_cache.clear();
        for word in &analysis.words {
            if !word.is_correct {
                self.error_cache.insert(word.start, word.clone());
            }
        }
    }
    
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        content: &mut String,
        modified: &mut bool,
        show_line_numbers: bool,
        analysis: &Option<DocumentAnalysis>,
    ) -> egui::Response {
        // Update analysis if provided
        if let Some(analysis) = analysis {
            self.set_analysis(analysis.clone());
        }
        
        let available_rect = ui.available_rect_before_wrap();
        
        // Calculate line numbers width if needed
        let line_numbers_width = if show_line_numbers {
            let line_count = content.lines().count().max(1);
            let max_digits = line_count.to_string().len();
            (max_digits as f32 * self.font_size * 0.6) + 20.0
        } else {
            0.0
        };
        
        // Create a custom widget for the editor
        let (rect, _response) = ui.allocate_exact_size(
            egui::vec2(available_rect.width(), available_rect.height()),
            egui::Sense::click_and_drag(),
        );
        
        // Draw the editor background
        ui.painter().rect_filled(
            rect,
            egui::Rounding::same(4.0),
            ui.visuals().extreme_bg_color,
        );
        
        // Draw line numbers if enabled
        if show_line_numbers {
            self.draw_line_numbers(ui, rect, content);
        }
        
        // Draw text content with error highlighting
        self.draw_text_with_errors(ui, rect, content, line_numbers_width);
        
        // Add a text edit widget for editing (transparent overlay)
        let text_edit_rect = egui::Rect::from_min_size(
            egui::pos2(rect.left() + line_numbers_width, rect.top()),
            egui::vec2(rect.width() - line_numbers_width, rect.height()),
        );
        
        ui.allocate_ui_at_rect(text_edit_rect, |ui| {
            let mut text_edit = egui::TextEdit::multiline(content)
                .desired_width(f32::INFINITY)
                .font(egui::FontId::monospace(self.font_size))
                .frame(false);
            
            if self.wrap_lines {
                text_edit = text_edit.desired_rows(10);
            }
            
            let edit_response = ui.add(text_edit);
            if edit_response.changed() {
                *modified = true;
            }
            edit_response
        }).inner
    }
    
    fn draw_line_numbers(&self, ui: &egui::Ui, rect: egui::Rect, content: &str) {
        let painter = ui.painter();
        let line_count = content.lines().count().max(1);
        let line_number_color = ui.visuals().weak_text_color();
        
        for i in 0..line_count {
            let line_y = rect.top() + (i as f32 * self.line_height) + (self.line_height * 0.75);
            let line_num = (i + 1).to_string();
            
            painter.text(
                egui::pos2(rect.left() + 5.0, line_y),
                egui::Align2::LEFT_CENTER,
                line_num,
                egui::FontId::monospace(self.font_size),
                line_number_color,
            );
        }
    }
    
    fn draw_text_with_errors(&self, ui: &egui::Ui, rect: egui::Rect, content: &str, line_numbers_width: f32) {
        let painter = ui.painter();
        let lines: Vec<&str> = content.lines().collect();
        let text_color = ui.visuals().text_color();
        let error_color = ui.visuals().error_fg_color;
        
        // Calculate character width for positioning
        let char_width = self.font_size * 0.6;
        
        for (line_idx, line) in lines.iter().enumerate() {
            let line_y = rect.top() + (line_idx as f32 * self.line_height);
            let text_x = rect.left() + line_numbers_width + 5.0;
            
            // Draw the line text
            painter.text(
                egui::pos2(text_x, line_y + (self.line_height * 0.75)),
                egui::Align2::LEFT_CENTER,
                *line,
                egui::FontId::monospace(self.font_size),
                text_color,
            );
            
            // Draw error highlights for this line
            if let Some(analysis) = &self.last_analysis {
                let line_num = line_idx + 1;
                let line_errors: Vec<&WordCheck> = analysis.words
                    .iter()
                    .filter(|w| !w.is_correct && w.line == line_num)
                    .collect();
                
                for error in line_errors {
                    // Calculate position of error in the line
                    let error_start_in_line = error.column.saturating_sub(1);
                    let error_x = text_x + (error_start_in_line as f32 * char_width);
                    let error_width = error.word.len() as f32 * char_width;
                    
                    // Draw wavy underline for the error
                    self.draw_wavy_underline(
                        painter,
                        error_x,
                        line_y + self.line_height - 4.0,
                        error_width,
                        error_color,
                    );
                    
                    // Draw subtle background highlight
                    painter.rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(error_x, line_y + 2.0),
                            egui::vec2(error_width, self.line_height - 6.0),
                        ),
                        0.0,
                        egui::Color32::from_rgba_unmultiplied(255, 0, 0, 15),
                    );
                }
            }
        }
    }
    
    fn draw_wavy_underline(
        &self,
        painter: &egui::Painter,
        x: f32,
        y: f32,
        width: f32,
        color: egui::Color32,
    ) {
        let wave_height = 2.0;
        let wave_length = 6.0;
        let segments = (width / wave_length).ceil() as usize;
        
        for i in 0..segments {
            let segment_x = x + i as f32 * wave_length;
            let segment_end_x = (segment_x + wave_length).min(x + width);
            
            let y_offset = if i % 2 == 0 { 0.0 } else { wave_height };
            
            painter.line_segment(
                [
                    egui::pos2(segment_x, y + y_offset),
                    egui::pos2(segment_end_x, y + y_offset),
                ],
                egui::Stroke::new(1.5, color),
            );
        }
    }
    
    pub fn set_font_size(&mut self, size: f32) {
        self.font_size = size;
        self.line_height = size * 1.8;
    }
    
    pub fn set_wrap_lines(&mut self, wrap: bool) {
        self.wrap_lines = wrap;
    }
    
    pub fn set_show_whitespace(&mut self, show: bool) {
        self.show_whitespace = show;
    }
    
    pub fn get_error_at_position(&self, line: usize, column: usize) -> Option<&WordCheck> {
        if let Some(analysis) = &self.last_analysis {
            analysis.words.iter()
                .find(|w| !w.is_correct && w.line == line && w.column <= column && column <= w.column + w.word.len())
        } else {
            None
        }
    }
}