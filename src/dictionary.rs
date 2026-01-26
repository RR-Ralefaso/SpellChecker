use crate::language::{Language, LanguageManager};
use dashmap::DashMap;
use rayon::prelude::*;
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use once_cell::sync::Lazy;

#[derive(Debug, Clone)]
pub struct Dictionary {
    words: HashSet<String>,
    word_pattern: Regex,
    ignore_pattern: Option<Regex>,
    min_word_length: usize,
    language: Language,
    is_loaded: bool,
    word_count_cache: usize,
}

impl Dictionary {
    pub fn new(language: Language) -> Self {
        let word_pattern = Self::get_word_pattern_for_language(&language);
        
        Self {
            words: HashSet::new(),
            word_pattern,
            ignore_pattern: None,
            min_word_length: 1,
            language,
            is_loaded: false,
            word_count_cache: 0,
        }
    }
    
    fn get_word_pattern_for_language(language: &Language) -> Regex {
        // Use lazy static regexes for common patterns
        static CHINESE_PATTERN: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"[\p{Han}\p{Hiragana}\p{Katakana}a-zA-Z0-9'-]+").unwrap()
        });
        
        static KOREAN_PATTERN: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"[\p{Hangul}a-zA-Z0-9'-]+").unwrap()
        });
        
        static RUSSIAN_PATTERN: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"[\p{Cyrillic}a-zA-Z0-9'-]+").unwrap()
        });
        
        static DEFAULT_PATTERN: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"[\p{L}0-9'-]+").unwrap()
        });
        
        match language {
            Language::Chinese | Language::Japanese => &CHINESE_PATTERN,
            Language::Korean => &KOREAN_PATTERN,
            Language::Russian => &RUSSIAN_PATTERN,
            _ => &DEFAULT_PATTERN,
        }.clone()
    }
    
    pub fn load(&mut self) -> anyhow::Result<()> {
        if self.is_loaded {
            return Ok(());
        }
        
        let language_manager = LanguageManager::new();
        
        if let Some(dict_path) = language_manager.get_dictionary_path(&self.language) {
            self.load_file(&dict_path)?;
            self.is_loaded = true;
            self.word_count_cache = self.words.len();
        } else if self.language != Language::English {
            // Try to load English as fallback
            let english_dict_path = language_manager.get_dictionary_path(&Language::English);
            if let Some(path) = english_dict_path {
                self.load_file(&path)?;
                self.is_loaded = true;
                self.word_count_cache = self.words.len();
            }
        }
        
        if !self.is_loaded {
            anyhow::bail!("Could not load dictionary for {}", self.language.name());
        }
        
        Ok(())
    }
    
    pub fn load_file(&mut self, path: &Path) -> anyhow::Result<()> {
        let bytes = fs::read(path)?;
        let (content, _, _) = encoding_rs::UTF_8.decode(&bytes);
        let content = content.into_owned();
        
        let new_words: HashSet<String> = content
            .par_lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .map(|word| {
                match self.language {
                    Language::Chinese | Language::Japanese | Language::Korean => {
                        word.to_string()
                    }
                    _ => {
                        word.to_lowercase()
                    }
                }
            })
            .filter(|word| word.len() >= self.min_word_length)
            .collect();
            
        self.words.extend(new_words);
        self.word_count_cache = self.words.len();
        
        Ok(())
    }
    
    pub fn contains(&self, word: &str, case_sensitive: bool) -> bool {
        let word = word.trim();
        
        if word.is_empty() || word.len() < self.min_word_length {
            return true;
        }
        
        if let Some(pattern) = &self.ignore_pattern {
            if pattern.is_match(word) {
                return true;
            }
        }
        
        // Skip words with numbers
        if word.chars().any(|c| c.is_ascii_digit()) {
            return true;
        }
        
        match self.language {
            Language::Chinese | Language::Japanese | Language::Korean => {
                // CJK languages are typically not case-sensitive
                self.words.contains(word)
            }
            _ => {
                if case_sensitive {
                    self.words.contains(word)
                } else {
                    self.words.contains(&word.to_lowercase())
                }
            }
        }
    }
    
    pub fn word_count(&self) -> usize {
        self.word_count_cache
    }
    
    pub fn get_words(&self) -> &HashSet<String> {
        &self.words
    }
    
    pub fn get_word_pattern(&self) -> &Regex {
        &self.word_pattern
    }
    
    pub fn language(&self) -> &Language {
        &self.language
    }
    
    pub fn is_loaded(&self) -> bool {
        self.is_loaded
    }
    
    pub fn add_word(&mut self, word: &str) {
        let normalized = match self.language {
            Language::Chinese | Language::Japanese | Language::Korean => word.trim().to_string(),
            _ => word.trim().to_lowercase(),
        };
        
        if !normalized.is_empty() {
            self.words.insert(normalized);
            self.word_count_cache = self.words.len();
        }
    }
    
    pub fn remove_word(&mut self, word: &str) -> bool {
        let removed = self.words.remove(word);
        if removed {
            self.word_count_cache = self.words.len();
        }
        removed
    }
}

#[derive(Clone)]
pub struct DictionaryManager {
    dictionaries: Arc<DashMap<Language, Dictionary>>,
    language_manager: LanguageManager,
}

impl DictionaryManager {
    pub fn new() -> Self {
        let manager = LanguageManager::new();
        let dictionaries = Arc::new(DashMap::new());
        
        // Pre-load English dictionary as default
        let mut english_dict = Dictionary::new(Language::English);
        if english_dict.load().is_ok() {
            dictionaries.insert(Language::English, english_dict);
        }
        
        Self {
            dictionaries,
            language_manager: manager,
        }
    }
    
    pub fn get_dictionary(&self, language: &Language) -> anyhow::Result<Dictionary> {
        if let Some(dict) = self.dictionaries.get(language) {
            return Ok(dict.clone());
        }
        
        let mut dict = Dictionary::new(*language);
        dict.load()?;
        
        self.dictionaries.insert(*language, dict.clone());
        
        Ok(dict)
    }
    
    pub fn reload_dictionary(&mut self, language: &Language) -> anyhow::Result<()> {
        let mut dict = Dictionary::new(*language);
        dict.load()?;
        self.dictionaries.insert(*language, dict);
        Ok(())
    }
    
    pub fn add_custom_dictionary(&mut self, path: PathBuf, language_code: String) -> anyhow::Result<()> {
        self.language_manager.add_custom_dictionary(path.clone(), language_code.clone());
        
        let language = Language::from_code(&language_code);
        let mut dict = Dictionary::new(language);
        dict.load_file(&path)?;
        
        self.dictionaries.insert(language, dict);
        
        Ok(())
    }
    
    pub fn get_available_languages(&self) -> Vec<Language> {
        self.language_manager.available_languages().to_vec()
    }
    
    pub fn detect_language(&self, text: &str) -> Language {
        self.language_manager.detect_language(text)
    }
    
    pub fn get_current_language(&self) -> &Language {
        self.language_manager.current_language()
    }
    
    pub fn set_current_language(&mut self, language: Language) {
        self.language_manager.set_language(language);
    }
    
    pub fn get_cached_dictionary(&self, language: &Language) -> Option<Dictionary> {
        self.dictionaries.get(language).map(|d| d.value().clone())
    }
}