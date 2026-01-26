use crate::dictionary::{Dictionary, DictionaryManager};
use crate::language::Language;
use crate::util::sanitize_word;
use dashmap::DashMap;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::cmp::min;

#[derive(Debug, Clone, Serialize)]
pub struct WordCheck {
    pub word: String,
    pub start: usize,
    pub end: usize,
    pub is_correct: bool,
    pub suggestions: Vec<String>,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DocumentAnalysis {
    pub total_words: usize,
    pub misspelled_words: usize,
    pub accuracy: f32,
    pub words: Vec<WordCheck>,
    pub suggestions_count: usize,
    pub language: Language,
    pub lines_checked: usize,
    pub check_duration_ms: u128,
}

pub struct SpellChecker {
    dictionary_manager: DictionaryManager,
    current_language: Language,
    suggestions_enabled: bool,
    case_sensitive: bool,
    max_suggestions: usize,
    cache: Arc<DashMap<String, bool>>,
    ignore_list: HashSet<String>,
}

impl SpellChecker {
    pub fn new(language: Language) -> anyhow::Result<Self> {
        let dictionary_manager = DictionaryManager::new();
        
        // Try to load dictionary
        let dict_result = dictionary_manager.get_dictionary(&language);
        if let Err(e) = dict_result {
            eprintln!("Warning: Could not load dictionary for {}: {}", language.name(), e);
            // Continue with empty dictionary
        }
        
        Ok(Self {
            dictionary_manager,
            current_language: language,
            suggestions_enabled: true,
            case_sensitive: false,
            max_suggestions: 5,
            cache: Arc::new(DashMap::new()),
            ignore_list: HashSet::new(),
        })
    }
    
    pub fn set_language(&mut self, language: Language) -> anyhow::Result<()> {
        if language != self.current_language {
            self.dictionary_manager.get_dictionary(&language)?;
            self.current_language = language;
            self.cache.clear(); // Clear cache when language changes
        }
        Ok(())
    }
    
    pub fn current_language(&self) -> Language {
        self.current_language
    }
    
    pub fn get_current_dictionary(&self) -> anyhow::Result<Dictionary> {
        self.dictionary_manager.get_dictionary(&self.current_language)
    }
    
    pub fn check_document(&self, text: &str) -> DocumentAnalysis {
        let start_time = std::time::Instant::now();
        
        let dictionary = match self.get_current_dictionary() {
            Ok(dict) => dict,
            Err(_) => {
                // Return empty analysis if no dictionary
                return DocumentAnalysis {
                    total_words: 0,
                    misspelled_words: 0,
                    accuracy: 100.0,
                    words: Vec::new(),
                    suggestions_count: 0,
                    language: self.current_language,
                    lines_checked: 0,
                    check_duration_ms: 0,
                };
            }
        };
        
        let word_pattern = dictionary.get_word_pattern();
        let lines: Vec<&str> = text.lines().collect();
        let mut words = Vec::new();
        let mut suggestions_count = 0;
        let mut total_words = 0;
        let mut misspelled_words = 0;
        
        let is_cjk = matches!(self.current_language, Language::Chinese | Language::Japanese | Language::Korean);
        
        for (line_idx, line) in lines.iter().enumerate() {
            let line_num = line_idx + 1;
            
            for mat in word_pattern.find_iter(line) {
                let word = mat.as_str();
                let start = mat.start();
                let end = mat.end();
                
                // Skip if word is in ignore list
                let word_lower = if is_cjk { word.to_string() } else { word.to_lowercase() };
                if self.ignore_list.contains(&word_lower) {
                    words.push(WordCheck {
                        word: word.to_string(),
                        start,
                        end,
                        is_correct: true,
                        suggestions: Vec::new(),
                        line: line_num,
                        column: start + 1,
                    });
                    continue;
                }
                
                // Check cache first
                let cache_key = format!("{}_{}", self.current_language.code(), word_lower);
                let is_correct = if let Some(cached) = self.cache.get(&cache_key) {
                    *cached
                } else {
                    let correct = dictionary.contains(word, self.case_sensitive);
                    self.cache.insert(cache_key.clone(), correct);
                    correct
                };
                
                total_words += 1;
                if !is_correct {
                    misspelled_words += 1;
                }
                
                let suggestions = if !is_correct && self.suggestions_enabled {
                    let sugg = self.get_suggestions(word, &dictionary);
                    suggestions_count += sugg.len();
                    sugg
                } else {
                    Vec::new()
                };
                
                words.push(WordCheck {
                    word: word.to_string(),
                    start,
                    end,
                    is_correct,
                    suggestions,
                    line: line_num,
                    column: start + 1,
                });
            }
        }
        
        let accuracy = if total_words > 0 {
            ((total_words - misspelled_words) as f32 / total_words as f32 * 100.0).round()
        } else {
            100.0
        };
        
        let check_duration = start_time.elapsed();
        
        DocumentAnalysis {
            total_words,
            misspelled_words,
            accuracy,
            words,
            suggestions_count,
            language: self.current_language,
            lines_checked: lines.len(),
            check_duration_ms: check_duration.as_millis(),
        }
    }
    
    fn get_suggestions(&self, word: &str, dictionary: &Dictionary) -> Vec<String> {
        // Don't suggest for very short words or numbers
        if word.len() <= 1 || word.chars().any(|c| c.is_numeric()) {
            return Vec::new();
        }
        
        let word_lower = word.to_lowercase();
        let dict_words = dictionary.get_words();
        
        // Early exit if word is already in dictionary
        if dict_words.contains(&word_lower) {
            return Vec::new();
        }
        
        // Limit the number of dictionary words to check for performance
        let max_candidates = 2000;
        let candidates: Vec<&String> = dict_words.iter()
            .filter(|w| {
                // Quick filter: similar length
                let len_diff = (w.len() as isize - word_lower.len() as isize).abs();
                len_diff <= 3
            })
            .take(max_candidates)
            .collect();
        
        let mut suggestions: Vec<(String, usize)> = candidates
            .par_iter()
            .map(|&dict_word| {
                let distance = self.edit_distance(&word_lower, dict_word);
                (dict_word.clone(), distance)
            })
            .filter(|(_, distance)| *distance <= 2) // Only suggest words with edit distance <= 2
            .collect();
        
        suggestions.sort_by_key(|(_, distance)| *distance);
        suggestions
            .into_iter()
            .take(self.max_suggestions)
            .map(|(word, _)| word)
            .collect()
    }
    
    // Optimized edit distance (Levenshtein) calculation
    fn edit_distance(&self, a: &str, b: &str) -> usize {
        if a == b { return 0; }
        
        let a_len = a.chars().count();
        let b_len = b.chars().count();
        
        if a_len == 0 { return b_len; }
        if b_len == 0 { return a_len; }
        
        // Use small vectors for common cases
        let mut prev_row: Vec<usize> = (0..=b_len).collect();
        let mut curr_row = vec![0; b_len + 1];
        
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();
        
        for i in 1..=a_len {
            curr_row[0] = i;
            
            for j in 1..=b_len {
                let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
                curr_row[j] = min(
                    min(prev_row[j] + 1, curr_row[j - 1] + 1),
                    prev_row[j - 1] + cost
                );
            }
            
            std::mem::swap(&mut prev_row, &mut curr_row);
        }
        
        prev_row[b_len]
    }
    
    pub fn add_word_to_dictionary(&mut self, word: &str) -> anyhow::Result<()> {
        let sanitized = sanitize_word(word);
        if sanitized.is_empty() {
            return Ok(());
        }
        
        // Update cache
        let cache_key = format!("{}_{}", self.current_language.code(), sanitized.to_lowercase());
        self.cache.insert(cache_key, true);
        
        // Update ignore list (remove if present)
        self.ignore_list.remove(&sanitized.to_lowercase());
        
        // Update dictionary
        self.dictionary_manager.add_word_to_dictionary(&sanitized, self.current_language)?;
        
        Ok(())
    }
    
    pub fn ignore_word(&mut self, word: &str) -> anyhow::Result<()> {
        let sanitized = sanitize_word(word);
        if !sanitized.is_empty() {
            self.ignore_list.insert(sanitized.to_lowercase());
        }
        Ok(())
    }
    
    pub fn clear_ignored_words(&mut self) {
        self.ignore_list.clear();
        self.cache.clear(); // Clear cache since ignore status changed
    }
    
    pub fn import_dictionary(&mut self, path: &Path) -> anyhow::Result<()> {
        // Read the file to detect language or use current language
        let content = fs::read_to_string(path)?;
        
        // Try to detect language from content
        let detected_language = self.dictionary_manager.detect_language(&content);
        let language_to_use = if detected_language != Language::English {
            detected_language
        } else {
            self.current_language
        };
        
        // Import dictionary
        self.dictionary_manager.import_dictionary(path.to_path_buf(), language_to_use)?;
        
        // Clear cache since dictionary changed
        self.cache.clear();
        
        Ok(())
    }
    
    pub fn export_dictionary(&self, path: &Path) -> anyhow::Result<()> {
        self.dictionary_manager.export_dictionary(&self.current_language, path)
    }
    
    pub fn enable_suggestions(&mut self, enabled: bool) {
        self.suggestions_enabled = enabled;
    }
    
    pub fn set_case_sensitive(&mut self, sensitive: bool) {
        self.case_sensitive = sensitive;
        self.cache.clear(); // Clear cache when case sensitivity changes
    }
    
    pub fn word_count(&self) -> usize {
        match self.get_current_dictionary() {
            Ok(dict) => dict.word_count(),
            Err(_) => 0,
        }
    }
    
    pub fn ignored_word_count(&self) -> usize {
        self.ignore_list.len()
    }
}