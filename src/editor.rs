use crate::checker::{DocumentAnalysis, WordCheck};
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
        for word in &analysis.words {
            if !word.is_correct {
                self.error_positions.insert(word.start, word.clone());
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
        let available_rect = ui.available_rect_before_wrap();
        
        // Use egui's TextEdit widget for editing
        let mut text_edit = egui::TextEdit::multiline(content)
            .desired_width(available_rect.width())
            .font(egui::FontId::monospace(self.font_size))
            .code_editor();
        
        // Configure based on settings
        if self.wrap_lines {
            text_edit = text_edit.desired_rows((available_rect.height() / self.line_height) as usize);
        } else {
            text_edit = text_edit.desired_width(f32::INFINITY);
        }
        
        let response = ui.add(text_edit.frame(false));
        
        if response.changed() {
            *modified = true;
        }
        
        // Update cursor position from text edit state
        if response.has_focus() {
            if let Some(state) = ui.memory(|mem| 
                mem.data.get_temp::<egui::text_edit::TextEditState>(response.id)
            ) {
                // Get cursor position from the text edit state
                let cursor_range = state.cursor_range();
                if let Some(range) = cursor_range {
                    self.cursor_pos = range.primary.index;
                }
            }
        }
        
        // Draw custom error highlighting on top
        self.draw_error_highlights(ui, &response.rect, show_line_numbers);
        
        response
    }
    
    fn draw_error_highlights(
        &self,
        ui: &mut egui::Ui,
        rect: &egui::Rect,
        show_line_numbers: bool,
    ) {
        let painter = ui.painter();
        
        if let Some(analysis) = &self.analysis {
            for word in &analysis.words {
                if !word.is_correct {
                    // Draw error underline
                    let line_y = rect.top() + (word.line - 1) as f32 * self.line_height;
                    let x_offset = if show_line_numbers { 55.0 } else { 5.0 };
                    let char_width = self.font_size * 0.6;
                    
                    // Simple positioning - for exact positioning we'd need character-level layout
                    let underline_x = x_offset + (word.column - 1) as f32 * char_width;
                    let underline_width = word.word.len() as f32 * char_width;
                    
                    // Draw wavy underline for errors
                    self.draw_wavy_underline(
                        painter,
                        underline_x,
                        line_y + self.line_height - 2.0,
                        underline_width,
                        ui.visuals().error_fg_color,
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
        let wave_length = 4.0;
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
                (1.0, color),
            );
        }
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