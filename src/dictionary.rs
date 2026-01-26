use crate::language::{Language, LanguageManager};
use dashmap::DashMap;
use rayon::prelude::*;
use regex::Regex;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use once_cell::sync::Lazy;

#[derive(Debug, Clone)]
pub struct Dictionary {
    words: HashSet<String>,
    ignored_words: HashSet<String>,
    word_pattern: Regex,
    min_word_length: usize,
    language: Language,
    is_loaded: bool,
    word_count_cache: usize,
    ignored_count_cache: usize,
    file_path: Option<PathBuf>,
}

impl Dictionary {
    pub fn new(language: Language) -> Self {
        let word_pattern = Self::get_word_pattern_for_language(&language);
        
        Self {
            words: HashSet::new(),
            ignored_words: HashSet::new(),
            word_pattern,
            min_word_length: 2,
            language,
            is_loaded: false,
            word_count_cache: 0,
            ignored_count_cache: 0,
            file_path: None,
        }
    }
    
    fn get_word_pattern_for_language(language: &Language) -> Regex {
        match language {
            Language::Chinese | Language::Japanese => {
                Regex::new(r"[\p{Han}\p{Hiragana}\p{Katakana}a-zA-Z0-9'-]+").unwrap()
            }
            Language::Korean => {
                Regex::new(r"[\p{Hangul}a-zA-Z0-9'-]+").unwrap()
            }
            Language::Russian => {
                Regex::new(r"[\p{Cyrillic}a-zA-Z0-9'-]+").unwrap()
            }
            _ => {
                Regex::new(r"[\p{L}0-9'-]+").unwrap()
            }
        }
    }
    
    pub fn load(&mut self) -> anyhow::Result<()> {
        if self.is_loaded {
            return Ok(());
        }
        
        let language_manager = LanguageManager::new();
        
        // Try to load main dictionary
        if let Some(dict_path) = language_manager.get_dictionary_path(&self.language) {
            println!("Loading dictionary for {} from: {:?}", self.language.name(), dict_path);
            self.load_file(&dict_path)?;
            self.file_path = Some(dict_path);
        } else {
            println!("No dictionary file found for {}. Creating empty dictionary.", self.language.name());
            // Create empty dictionary
            self.words = HashSet::new();
        }
        
        // Load user-added words
        self.load_user_words();
        
        // Load ignored words
        self.load_ignored_words();
        
        self.is_loaded = true;
        self.word_count_cache = self.words.len();
        self.ignored_count_cache = self.ignored_words.len();
        
        println!("Loaded {} words ({} ignored) for {}", 
            self.word_count_cache, self.ignored_count_cache, self.language.name());
        
        Ok(())
    }
    
    pub fn load_file(&mut self, path: &Path) -> anyhow::Result<()> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        
        let mut new_words = HashSet::new();
        
        for line in reader.lines() {
            let line = line?;
            let word = line.trim();
            
            if !word.is_empty() && word.len() >= self.min_word_length {
                let normalized = self.normalize_word(word);
                new_words.insert(normalized);
            }
        }
        
        self.words.extend(new_words);
        self.word_count_cache = self.words.len();
        
        Ok(())
    }
    
    fn normalize_word(&self, word: &str) -> String {
        match self.language {
            Language::Chinese | Language::Japanese | Language::Korean => {
                word.to_string() // Keep original for CJK
            }
            _ => {
                word.to_lowercase()
            }
        }
    }
    
    fn load_user_words(&mut self) {
        let mut path = LanguageManager::user_dict_dir();
        path.push(format!("user_{}.txt", self.language.code()));
        
        if let Ok(file) = File::open(&path) {
            let reader = BufReader::new(file);
            for line in reader.lines() {
                if let Ok(word) = line {
                    let word = word.trim().to_string();
                    if !word.is_empty() {
                        self.words.insert(self.normalize_word(&word));
                    }
                }
            }
        }
    }
    
    fn save_user_words(&self) -> anyhow::Result<()> {
        let mut path = LanguageManager::user_dict_dir();
        path.push(format!("user_{}.txt", self.language.code()));
        
        let mut file = File::create(&path)?;
        
        // Get user-added words (words not in original dictionary)
        // For simplicity, we'll save all words
        let mut sorted_words: Vec<&String> = self.words.iter().collect();
        sorted_words.sort();
        
        for word in sorted_words {
            writeln!(file, "{}", word)?;
        }
        
        Ok(())
    }
    
    fn load_ignored_words(&mut self) {
        let mut path = LanguageManager::user_dict_dir();
        path.push(format!("ignored_{}.txt", self.language.code()));
        
        if let Ok(file) = File::open(&path) {
            let reader = BufReader::new(file);
            for line in reader.lines() {
                if let Ok(word) = line {
                    let word = word.trim().to_string();
                    if !word.is_empty() {
                        self.ignored_words.insert(self.normalize_word(&word));
                    }
                }
            }
            self.ignored_count_cache = self.ignored_words.len();
        }
    }
    
    fn save_ignored_words(&self) -> anyhow::Result<()> {
        let mut path = LanguageManager::user_dict_dir();
        path.push(format!("ignored_{}.txt", self.language.code()));
        
        let mut file = File::create(&path)?;
        
        let mut sorted_words: Vec<&String> = self.ignored_words.iter().collect();
        sorted_words.sort();
        
        for word in sorted_words {
            writeln!(file, "{}", word)?;
        }
        
        Ok(())
    }
    
    pub fn contains(&self, word: &str, case_sensitive: bool) -> bool {
        let word = word.trim();
        
        if word.is_empty() || word.len() < self.min_word_length {
            return true;
        }
        
        // Check if word is ignored
        let normalized = self.normalize_word(word);
        if self.ignored_words.contains(&normalized) {
            return true;
        }
        
        // Skip words with numbers (except in CJK)
        if !matches!(self.language, Language::Chinese | Language::Japanese | Language::Korean) {
            if word.chars().any(|c| c.is_ascii_digit()) {
                return true;
            }
        }
        
        // Check in dictionary
        match self.language {
            Language::Chinese | Language::Japanese | Language::Korean => {
                // CJK languages: check exact match
                self.words.contains(&normalized)
            }
            _ => {
                if case_sensitive {
                    self.words.contains(word)
                } else {
                    self.words.contains(&normalized)
                }
            }
        }
    }
    
    pub fn word_count(&self) -> usize {
        self.word_count_cache
    }
    
    pub fn ignored_word_count(&self) -> usize {
        self.ignored_count_cache
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
    
    pub fn add_word(&mut self, word: &str) -> anyhow::Result<()> {
        let normalized = self.normalize_word(word.trim());
        
        if !normalized.is_empty() && normalized.len() >= self.min_word_length {
            self.words.insert(normalized.clone());
            self.word_count_cache = self.words.len();
            
            // Remove from ignored words if it was there
            self.ignored_words.remove(&normalized);
            self.ignored_count_cache = self.ignored_words.len();
            
            // Save to user dictionary file
            self.save_user_words()?;
        }
        
        Ok(())
    }
    
    pub fn ignore_word(&mut self, word: &str) -> anyhow::Result<()> {
        let normalized = self.normalize_word(word.trim());
        
        if !normalized.is_empty() {
            self.ignored_words.insert(normalized);
            self.ignored_count_cache = self.ignored_words.len();
            
            // Save ignored words
            self.save_ignored_words()?;
        }
        
        Ok(())
    }
    
    pub fn clear_ignored_words(&mut self) -> anyhow::Result<()> {
        self.ignored_words.clear();
        self.ignored_count_cache = 0;
        
        // Save empty ignored words file
        self.save_ignored_words()?;
        
        Ok(())
    }
    
    pub fn remove_word(&mut self, word: &str) -> bool {
        let removed = self.words.remove(word);
        if removed {
            self.word_count_cache = self.words.len();
            // Note: We don't remove from file, just from memory
        }
        removed
    }
    
    pub fn save_to_file(&self, path: &Path) -> anyhow::Result<()> {
        let mut file = File::create(path)?;
        
        let mut sorted_words: Vec<&String> = self.words.iter().collect();
        sorted_words.sort();
        
        for word in sorted_words {
            writeln!(file, "{}", word)?;
        }
        
        Ok(())
    }
    
    pub fn import_from_file(&mut self, path: &Path) -> anyhow::Result<()> {
        self.load_file(path)
    }
    
    pub fn export_to_file(&self, path: &Path) -> anyhow::Result<()> {
        self.save_to_file(path)
    }
}

#[derive(Clone)]
pub struct DictionaryManager {
    dictionaries: Arc<DashMap<Language, Dictionary>>,
    language_manager: LanguageManager,
}

impl Default for DictionaryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DictionaryManager {
    pub fn new() -> Self {
        let manager = LanguageManager::new();
        let dictionaries = Arc::new(DashMap::new());
        
        Self {
            dictionaries,
            language_manager: manager,
        }
    }
    
    pub fn get_dictionary(&self, language: &Language) -> anyhow::Result<Dictionary> {
        // Check if dictionary is already loaded
        if let Some(dict) = self.dictionaries.get(language) {
            return Ok(dict.clone());
        }
        
        // Load dictionary
        let mut dict = Dictionary::new(*language);
        dict.load()?;
        
        // Cache it
        self.dictionaries.insert(*language, dict.clone());
        
        Ok(dict)
    }
    
    pub fn reload_dictionary(&mut self, language: &Language) -> anyhow::Result<()> {
        let mut dict = Dictionary::new(*language);
        dict.load()?;
        self.dictionaries.insert(*language, dict);
        Ok(())
    }
    
    pub fn add_custom_dictionary(&mut self, path: PathBuf, language: Language) -> anyhow::Result<()> {
        let mut dict = Dictionary::new(language);
        dict.import_from_file(&path)?;
        self.dictionaries.insert(language, dict);
        Ok(())
    }
    
    pub fn add_word_to_dictionary(&mut self, word: &str, language: Language) -> anyhow::Result<()> {
        if let Some(mut dict) = self.dictionaries.get_mut(&language) {
            dict.add_word(word)
        } else {
            let mut dict = Dictionary::new(language);
            dict.load()?;
            dict.add_word(word)?;
            self.dictionaries.insert(language, dict);
            Ok(())
        }
    }
    
    pub fn ignore_word(&mut self, word: &str, language: Language) -> anyhow::Result<()> {
        if let Some(mut dict) = self.dictionaries.get_mut(&language) {
            dict.ignore_word(word)
        } else {
            let mut dict = Dictionary::new(language);
            dict.load()?;
            dict.ignore_word(word)?;
            self.dictionaries.insert(language, dict);
            Ok(())
        }
    }
    
    pub fn clear_ignored_words(&mut self, language: Language) -> anyhow::Result<()> {
        if let Some(mut dict) = self.dictionaries.get_mut(&language) {
            dict.clear_ignored_words()
        } else {
            let mut dict = Dictionary::new(language);
            dict.load()?;
            dict.clear_ignored_words()?;
            self.dictionaries.insert(language, dict);
            Ok(())
        }
    }
    
    pub fn import_dictionary(&mut self, path: &Path, language: Language) -> anyhow::Result<()> {
        self.add_custom_dictionary(path.to_path_buf(), language)
    }
    
    pub fn export_dictionary(&self, language: &Language, path: &Path) -> anyhow::Result<()> {
        let dict = self.get_dictionary(language)?;
        dict.export_to_file(path)
    }
    
    pub fn get_available_languages(&self) -> Vec<Language> {
        self.language_manager.available_languages().to_vec()
    }
    
    pub fn detect_language(&self, text: &str) -> Language {
        self.language_manager.detect_language(text)
    }
    
    pub fn get_current_language(&self) -> Language {
        self.language_manager.current_language()
    }
    
    pub fn set_current_language(&mut self, language: Language) {
        self.language_manager.set_language(language);
    }
    
    pub fn get_cached_dictionary(&self, language: &Language) -> Option<Dictionary> {
        self.dictionaries.get(language).map(|d| d.value().clone())
    }
}