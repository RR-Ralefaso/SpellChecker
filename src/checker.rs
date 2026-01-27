use crate::dictionary::{Dictionary, DictionaryManager};
use crate::language::Language;
use crate::util::{sanitize_word, is_valid_word, is_code_file, is_likely_code};
use dashmap::DashMap;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
pub struct WordCheck {
    pub word: String,
    pub original: String,
    pub start: usize,
    pub end: usize,
    pub is_correct: bool,
    pub suggestions: Vec<String>,
    pub line: usize,
    pub column: usize,
    pub confidence: f32,
    pub word_type: WordType,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum WordType {
    Normal,
    CodeIdentifier,
    Acronym,
    ProperNoun,
    TechnicalTerm,
    Number,
    Symbol,
    ShortWord,
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
    pub likely_code: bool,
    pub file_type: Option<String>,
    pub unique_words: usize,
}

pub struct SpellChecker {
    dictionary_manager: DictionaryManager,
    current_language: Language,
    suggestions_enabled: bool,
    case_sensitive: bool,
    max_suggestions: usize,
    cache: Arc<DashMap<String, bool>>,
    ignore_list: HashSet<String>,
    user_dictionary: HashSet<String>,
    proper_nouns: HashSet<String>,
    acronyms: HashSet<String>,
    confidence_threshold: f32,
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
        
        let mut checker = Self {
            dictionary_manager,
            current_language: language,
            suggestions_enabled: true,
            case_sensitive: false,
            max_suggestions: 5,
            cache: Arc::new(DashMap::new()),
            ignore_list: HashSet::new(),
            user_dictionary: HashSet::new(),
            proper_nouns: HashSet::new(),
            acronyms: HashSet::new(),
            confidence_threshold: 0.7,
        };
        
        // Load user data
        checker.load_user_data();
        
        Ok(checker)
    }
    
    fn load_user_data(&mut self) {
        // Load user dictionary
        let user_dict_path = crate::language::LanguageManager::user_dict_dir()
            .join(format!("user_{}.txt", self.current_language.code()));
        
        if let Ok(content) = fs::read_to_string(&user_dict_path) {
            for line in content.lines() {
                let word = line.trim().to_lowercase();
                if !word.is_empty() {
                    self.user_dictionary.insert(word);
                }
            }
        }
        
        // Load proper nouns
        let proper_nouns_path = crate::language::LanguageManager::user_dict_dir()
            .join(format!("proper_{}.txt", self.current_language.code()));
        
        if let Ok(content) = fs::read_to_string(&proper_nouns_path) {
            for line in content.lines() {
                let word = line.trim().to_lowercase();
                if !word.is_empty() {
                    self.proper_nouns.insert(word);
                }
            }
        }
        
        // Load common acronyms
        self.acronyms.extend(vec![
            "api", "http", "https", "url", "uri", "html", "css", "js", "ts",
            "json", "xml", "sql", "nosql", "cpu", "gpu", "ram", "rom", "usb",
            "ssd", "hdd", "lan", "wan", "vpn", "dns", "ip", "tcp", "udp",
            "ftp", "ssh", "ssl", "tls", "csv", "pdf", "doc", "jpg", "png",
            "gif", "mp3", "mp4", "avi", "mkv", "zip", "rar", "tar", "gz",
            "exe", "dll", "so", "dylib", "bin", "iso", "img", "vm", "aws",
            "gcp", "azure", "api", "ui", "ux", "cli", "gui", "ide", "sdk",
        ].into_iter().map(String::from));
    }
    
    pub fn set_language(&mut self, language: Language) -> anyhow::Result<()> {
        if language != self.current_language {
            self.dictionary_manager.get_dictionary(&language)?;
            self.current_language = language;
            self.cache.clear();
            self.load_user_data();
        }
        Ok(())
    }
    
    pub fn current_language(&self) -> Language {
        self.current_language
    }
    
    pub fn get_current_dictionary(&self) -> anyhow::Result<Dictionary> {
        self.dictionary_manager.get_dictionary(&self.current_language)
    }
    
    pub fn check_document(&self, text: &str, filename: Option<&str>) -> DocumentAnalysis {
        let start_time = std::time::Instant::now();
        
        let dictionary = match self.get_current_dictionary() {
            Ok(dict) => dict,
            Err(_) => {
                return DocumentAnalysis {
                    total_words: 0,
                    misspelled_words: 0,
                    accuracy: 100.0,
                    words: Vec::new(),
                    suggestions_count: 0,
                    language: self.current_language,
                    lines_checked: 0,
                    check_duration_ms: 0,
                    likely_code: false,
                    file_type: filename.map(|f| f.to_string()),
                    unique_words: 0,
                };
            }
        };
        
        let is_cjk = matches!(self.current_language, Language::Chinese | Language::Japanese | Language::Korean);
        let is_code = filename.map(|f| is_code_file(f)).unwrap_or(false) || is_likely_code(text);
        
        let lines: Vec<&str> = text.lines().collect();
        let mut words = Vec::new();
        let mut suggestions_count = 0;
        let mut total_words = 0;
        let mut misspelled_words = 0;
        let mut unique_words = HashSet::new();
        
        for (line_idx, line) in lines.iter().enumerate() {
            let line_num = line_idx + 1;
            
            let word_pattern = if is_cjk {
                crate::util::CJK_WORD_REGEX.clone()
            } else if is_code {
                crate::util::CODE_WORD_REGEX.clone()
            } else {
                crate::util::WORD_REGEX.clone()
            };
            
            for mat in word_pattern.find_iter(line) {
                let original_word = mat.as_str();
                let start = mat.start();
                let end = mat.end();
                
                // Determine word type
                let word_type = self.determine_word_type(original_word, is_code);
                
                // Skip based on word type
                if self.should_skip_word(original_word, &word_type) {
                    words.push(WordCheck {
                        word: original_word.to_lowercase(),
                        original: original_word.to_string(),
                        start,
                        end,
                        is_correct: true,
                        suggestions: Vec::new(),
                        line: line_num,
                        column: start + 1,
                        confidence: 1.0,
                        word_type,
                    });
                    continue;
                }
                
                let word_lower = original_word.to_lowercase();
                unique_words.insert(word_lower.clone());
                
                // Check in various dictionaries and lists
                let is_correct = self.check_word_correctness(&word_lower, original_word, &word_type, &dictionary, is_code);
                let confidence = self.calculate_confidence(original_word, &word_type, is_correct, is_code);
                
                total_words += 1;
                if !is_correct && confidence >= self.confidence_threshold {
                    misspelled_words += 1;
                }
                
                let suggestions = if !is_correct && self.suggestions_enabled && confidence >= self.confidence_threshold {
                    let sugg = self.get_suggestions(&word_lower, &dictionary);
                    suggestions_count += sugg.len();
                    sugg
                } else {
                    Vec::new()
                };
                
                words.push(WordCheck {
                    word: word_lower.clone(),
                    original: original_word.to_string(),
                    start,
                    end,
                    is_correct: is_correct || confidence < self.confidence_threshold,
                    suggestions,
                    line: line_num,
                    column: start + 1,
                    confidence,
                    word_type,
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
            likely_code: is_code,
            file_type: filename.map(|f| f.to_string()),
            unique_words: unique_words.len(),
        }
    }
    
    fn determine_word_type(&self, word: &str, is_code: bool) -> WordType {
        // Check for numbers
        if word.chars().all(|c| c.is_numeric()) {
            return WordType::Number;
        }
        
        // Check for symbols
        if word.chars().all(|c| !c.is_alphabetic()) {
            return WordType::Symbol;
        }
        
        // Check for short words
        if word.len() <= 2 {
            return WordType::ShortWord;
        }
        
        // Check for acronyms (all caps or with numbers)
        if word.chars().all(|c| c.is_uppercase() || c.is_numeric() || c == '_') && word.len() <= 6 {
            return WordType::Acronym;
        }
        
        // Check for proper nouns (starts with capital, not at sentence start)
        if word.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) && word.len() > 2 {
            let common_caps = ["I", "A", "The", "And", "But", "Or", "For", "Nor", "Yet", "So"];
            if !common_caps.contains(&word) {
                return WordType::ProperNoun;
            }
        }
        
        // Check for code identifiers
        if is_code && (word.contains('_') || 
                      (word.chars().any(|c| c.is_uppercase()) && word.chars().any(|c| c.is_lowercase())) ||
                      word.starts_with("get_") || word.starts_with("set_") ||
                      word.ends_with("_t") || word.ends_with("_ptr") ||
                      word.ends_with("Handler") || word.ends_with("Service")) {
            return WordType::CodeIdentifier;
        }
        
        // Check for technical terms
        if word.contains('-') && word.len() > 5 {
            return WordType::TechnicalTerm;
        }
        
        WordType::Normal
    }
    
    fn should_skip_word(&self, word: &str, word_type: &WordType) -> bool {
        match word_type {
            WordType::Number | WordType::Symbol | WordType::ShortWord => true,
            WordType::Acronym => {
                self.acronyms.contains(&word.to_lowercase())
            }
            WordType::CodeIdentifier => {
                word.len() <= 3 || // Very short identifiers
                word.chars().all(|c| c.is_numeric()) || // Numbers
                word.starts_with("0x") || // Hex numbers
                word.contains("__") // Python dunders
            }
            WordType::ProperNoun => {
                self.proper_nouns.contains(&word.to_lowercase())
            }
            _ => false,
        }
    }
    
    fn check_word_correctness(&self, word_lower: &str, original_word: &str, word_type: &WordType, dictionary: &Dictionary, is_code: bool) -> bool {
        // Check ignore list
        if self.ignore_list.contains(word_lower) {
            return true;
        }
        
        // Check user dictionary
        if self.user_dictionary.contains(word_lower) {
            return true;
        }
        
        // Check cache
        let cache_key = format!("{}_{}", self.current_language.code(), word_lower);
        if let Some(cached) = self.cache.get(&cache_key) {
            return *cached;
        }
        
        // Check main dictionary
        let in_dictionary = dictionary.contains(original_word, self.case_sensitive, is_code);
        
        // For proper nouns and acronyms, be more lenient
        let is_correct = match word_type {
            WordType::ProperNoun | WordType::Acronym => {
                in_dictionary || self.looks_reasonable(original_word)
            }
            WordType::CodeIdentifier => {
                in_dictionary || original_word.len() <= 15
            }
            _ => in_dictionary,
        };
        
        self.cache.insert(cache_key, is_correct);
        is_correct
    }
    
    fn looks_reasonable(&self, word: &str) -> bool {
        if word.is_empty() || word.len() > 25 {
            return false;
        }
        
        let letters = word.chars().filter(|c| c.is_alphabetic()).count();
        let total = word.chars().count();
        
        if total == 0 {
            return false;
        }
        
        let letter_ratio = letters as f32 / total as f32;
        
        letter_ratio > 0.7 &&
        !has_repeated_characters(word, 4) &&
        (word.len() <= 4 || has_vowels(word))
    }
    
    fn calculate_confidence(&self, word: &str, word_type: &WordType, is_correct: bool, is_code: bool) -> f32 {
        if is_correct {
            return 1.0;
        }
        
        let mut confidence: f32 = 0.5;
        
        match word_type {
            WordType::Normal => confidence *= 1.2,
            WordType::CodeIdentifier => confidence *= if is_code { 0.3 } else { 0.8 },
            WordType::Acronym => confidence *= 0.4,
            WordType::ProperNoun => confidence *= 0.6,
            WordType::TechnicalTerm => confidence *= 0.8,
            _ => confidence *= 0.2,
        }
        
        if word.len() < 3 {
            confidence *= 0.3;
        } else if word.len() > 20 {
            confidence *= 0.7;
        }
        
        if word.contains('_') || word.contains('-') {
            confidence *= 1.1;
        }
        
        if has_common_typo_patterns(word) {
            confidence *= 1.3;
        }
        
        confidence.min(1.0).max(0.0)
    }
    
    fn get_suggestions(&self, word: &str, dictionary: &Dictionary) -> Vec<String> {
        if word.len() <= 1 {
            return Vec::new();
        }
        
        let dict_words = dictionary.get_words();
        let max_candidates = 2000;
        
        let candidates: Vec<&String> = dict_words.iter()
            .filter(|w| {
                let len_diff = (w.len() as isize - word.len() as isize).abs();
                len_diff <= 3
            })
            .take(max_candidates)
            .collect();
        
        let mut suggestions: Vec<(String, usize)> = candidates
            .par_iter()
            .map(|&dict_word| {
                let distance = self.edit_distance(word, dict_word);
                (dict_word.clone(), distance)
            })
            .filter(|(_, distance)| *distance <= 2)
            .collect();
        
        suggestions.sort_by_key(|(_, distance)| *distance);
        suggestions
            .into_iter()
            .take(self.max_suggestions)
            .map(|(word, _)| word)
            .collect()
    }
    
    fn edit_distance(&self, a: &str, b: &str) -> usize {
        crate::util::levenshtein_distance(a, b)
    }
    
    pub fn add_word_to_dictionary(&mut self, word: &str) -> anyhow::Result<()> {
        let sanitized = sanitize_word(word);
        if !is_valid_word(&sanitized) {
            return Ok(());
        }
        
        let word_lower = sanitized.to_lowercase();
        
        // Update cache
        let cache_key = format!("{}_{}", self.current_language.code(), word_lower);
        self.cache.insert(cache_key, true);
        
        // Update ignore list (remove if present)
        self.ignore_list.remove(&word_lower);
        
        // Update user dictionary
        self.user_dictionary.insert(word_lower.clone());
        
        // Update dictionary file
        let user_dict_path = crate::language::LanguageManager::user_dict_dir()
            .join(format!("user_{}.txt", self.current_language.code()));
        
        let mut content = String::new();
        if let Ok(existing) = fs::read_to_string(&user_dict_path) {
            content = existing;
        }
        
        if !content.lines().any(|line| line.trim() == word_lower) {
            content.push_str(&format!("{}\n", word_lower));
            fs::write(&user_dict_path, content)?;
        }
        
        // Update dictionary manager
        self.dictionary_manager.add_word_to_dictionary(&sanitized, self.current_language)?;
        
        Ok(())
    }
    
    pub fn ignore_word(&mut self, word: &str) -> anyhow::Result<()> {
        let sanitized = sanitize_word(word);
        if is_valid_word(&sanitized) {
            self.ignore_list.insert(sanitized.to_lowercase());
        }
        Ok(())
    }
    
    pub fn clear_ignored_words(&mut self) {
        self.ignore_list.clear();
        self.cache.clear();
    }
    
    pub fn import_dictionary(&mut self, path: &Path) -> anyhow::Result<()> {
        let content = fs::read_to_string(path)?;
        let detected_language = self.dictionary_manager.detect_language(&content);
        let language_to_use = if detected_language != Language::English {
            detected_language
        } else {
            self.current_language
        };
        
        self.dictionary_manager.import_dictionary(path.to_path_buf(), language_to_use)?;
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
        self.cache.clear();
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
    
    pub fn user_word_count(&self) -> usize {
        self.user_dictionary.len()
    }
}

fn has_repeated_characters(word: &str, max_repeats: usize) -> bool {
    let chars: Vec<char> = word.chars().collect();
    let mut current_char = ' ';
    let mut current_count = 0;
    
    for &c in &chars {
        if c == current_char {
            current_count += 1;
            if current_count > max_repeats {
                return true;
            }
        } else {
            current_char = c;
            current_count = 1;
        }
    }
    
    false
}

fn has_vowels(word: &str) -> bool {
    let vowels = ['a', 'e', 'i', 'o', 'u', 'y', 'A', 'E', 'I', 'O', 'U', 'Y'];
    word.chars().any(|c| vowels.contains(&c))
}

fn has_common_typo_patterns(word: &str) -> bool {
    let common_patterns = [
        "ie", "ei", "tion", "sion", "able", "ible", "ment", "ness", "ough"
    ];
    
    common_patterns.iter().any(|pattern| word.contains(pattern))
}