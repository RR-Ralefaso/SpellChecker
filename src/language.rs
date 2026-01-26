use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use once_cell::sync::Lazy;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    English,
    Afrikaans,
    French,
    Spanish,
    German,
    Chinese,
    Italian,
    Portuguese,
    Russian,
    Japanese,
    Korean,
    AutoDetect,
    Custom(&'static str),
}

// Manual Serialize implementation to handle Custom variant
impl Serialize for Language {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Language::Custom(code) => serializer.serialize_str(&format!("custom:{}", code)),
            _ => serializer.serialize_str(self.code()),
        }
    }
}

// Manual Deserialize implementation
impl<'de> Deserialize<'de> for Language {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s.starts_with("custom:") {
            let code = s.trim_start_matches("custom:");
            let static_str: &'static str = Box::leak(code.to_string().into_boxed_str());
            Ok(Language::Custom(static_str))
        } else {
            Ok(Language::from_code(&s))
        }
    }
}

impl Language {
    pub fn all() -> Vec<Language> {
        vec![
            Language::English,
            Language::Afrikaans,
            Language::French,
            Language::Spanish,
            Language::German,
            Language::Chinese,
            Language::Italian,
            Language::Portuguese,
            Language::Russian,
            Language::Japanese,
            Language::Korean,
            Language::AutoDetect,
        ]
    }
    
    pub fn code(&self) -> &str {
        match self {
            Language::English => "eng",
            Language::Afrikaans => "afr",
            Language::French => "fra",
            Language::Spanish => "spa",
            Language::German => "deu",
            Language::Chinese => "zho",
            Language::Italian => "ita",
            Language::Portuguese => "por",
            Language::Russian => "rus",
            Language::Japanese => "jpn",
            Language::Korean => "kor",
            Language::AutoDetect => "auto",
            Language::Custom(code) => code,
        }
    }
    
    pub fn name(&self) -> &str {
        match self {
            Language::English => "English",
            Language::Afrikaans => "Afrikaans",
            Language::French => "French",
            Language::Spanish => "Spanish",
            Language::German => "German",
            Language::Chinese => "Chinese",
            Language::Italian => "Italian",
            Language::Portuguese => "Portuguese",
            Language::Russian => "Russian",
            Language::Japanese => "Japanese",
            Language::Korean => "Korean",
            Language::AutoDetect => "Auto-detect",
            Language::Custom(code) => code,
        }
    }
    
    pub fn flag_emoji(&self) -> &str {
        match self {
            Language::English => "🇬🇧",
            Language::Afrikaans => "🇿🇦",
            Language::French => "🇫🇷",
            Language::Spanish => "🇪🇸",
            Language::German => "🇩🇪",
            Language::Chinese => "🇨🇳",
            Language::Italian => "🇮🇹",
            Language::Portuguese => "🇵🇹",
            Language::Russian => "🇷🇺",
            Language::Japanese => "🇯🇵",
            Language::Korean => "🇰🇷",
            Language::AutoDetect => "🌐",
            Language::Custom(_) => "⚙️",
        }
    }
    
    pub fn dictionary_filename(&self) -> Option<String> {
        match self {
            Language::AutoDetect => None,
            Language::Custom(_) => None,
            //HIGHLIGHT
            _ => Some(format!("dictionary/dictionary({}).txt", self.code())),
        }
    }
    
    pub fn from_code(code: &str) -> Self {
        match code.to_lowercase().as_str() {
            "eng" | "en" => Language::English,
            "afr" | "af" => Language::Afrikaans,
            "fra" | "fr" => Language::French,
            "spa" | "es" => Language::Spanish,
            "deu" | "de" => Language::German,
            "zho" | "zh" => Language::Chinese,
            "ita" | "it" => Language::Italian,
            "por" | "pt" => Language::Portuguese,
            "rus" | "ru" => Language::Russian,
            "jpn" | "ja" => Language::Japanese,
            "kor" | "ko" => Language::Korean,
            "auto" => Language::AutoDetect,
            custom => {
                // Create a static string for custom language codes
                let static_str: &'static str = Box::leak(custom.to_string().into_boxed_str());
                Language::Custom(static_str)
            }
        }
    }
    
    pub fn detect_from_text(text: &str) -> Vec<(Language, f32)> {
        static COMMON_WORDS: Lazy<HashMap<Language, Vec<&'static str>>> = Lazy::new(|| {
            let mut map = HashMap::new();
            map.insert(Language::English, vec![
                "the", "and", "that", "have", "for", "with", "this", "from", "they", "would",
            ]);
            map.insert(Language::Afrikaans, vec![
                "die", "en", "het", "vir", "om", "wat", "in", "is", "jy", "ek",
            ]);
            map.insert(Language::French, vec![
                "le", "la", "et", "que", "dans", "un", "est", "pour", "des", "les",
            ]);
            map.insert(Language::Spanish, vec![
                "el", "la", "que", "y", "en", "los", "se", "del", "las", "un",
            ]);
            map.insert(Language::German, vec![
                "der", "die", "das", "und", "den", "dem", "des", "ein", "eine", "einer",
            ]);
            map.insert(Language::Chinese, vec![
                "的", "是", "在", "了", "和", "有", "人", "这", "中", "大",
            ]);
            map.insert(Language::Italian, vec![
                "il", "la", "che", "e", "di", "in", "un", "una", "per", "con",
            ]);
            map.insert(Language::Portuguese, vec![
                "o", "a", "e", "que", "do", "da", "em", "um", "para", "com",
            ]);
            map.insert(Language::Russian, vec![
                "и", "в", "не", "на", "я", "что", "он", "с", "как", "все",
            ]);
            map.insert(Language::Japanese, vec![
                "の", "に", "を", "は", "が", "と", "で", "た", "し", "て",
            ]);
            map.insert(Language::Korean, vec![
                "이", "가", "을", "를", "은", "는", "에", "의", "고", "하다",
            ]);
            map
        });
        
        let text_lower = text.to_lowercase();
        let words: Vec<&str> = text_lower.split_whitespace().collect();
        let total_words = words.len().max(1);
        
        let mut scores = HashMap::new();
        
        for (language, common_words) in COMMON_WORDS.iter() {
            let mut score = 0.0;
            
            // Check for common words
            for word in common_words {
                if text_lower.contains(word) {
                    score += 5.0;
                }
            }
            
            // Count occurrences of common words
            let mut matches = 0;
            for word in words.iter().take(100) { // Check first 100 words
                if common_words.contains(word) {
                    matches += 1;
                }
            }
            
            score += (matches as f32 / total_words as f32) * 50.0;
            
            if score > 0.0 {
                scores.insert(*language, score);
            }
        }
        
        // Character-based detection for CJK
        let cjk_count = text.chars().filter(|c| {
            let c = *c;
            ('\u{4E00}' <= c && c <= '\u{9FFF}') || // Chinese
            ('\u{3040}' <= c && c <= '\u{309F}') || // Hiragana
            ('\u{30A0}' <= c && c <= '\u{30FF}') || // Katakana
            ('\u{AC00}' <= c && c <= '\u{D7AF}')    // Hangul
        }).count();
        
        let cjk_ratio = cjk_count as f32 / text.chars().count().max(1) as f32;
        
        if cjk_ratio > 0.3 {
            if text.contains('\u{4E00}') { // Chinese character range
                scores.insert(Language::Chinese, 100.0);
            } else if text.contains('\u{3040}') { // Hiragana
                scores.insert(Language::Japanese, 100.0);
            } else if text.contains('\u{AC00}') { // Hangul
                scores.insert(Language::Korean, 100.0);
            }
        }
        
        // Sort by score
        let mut sorted_scores: Vec<(Language, f32)> = scores.into_iter().collect();
        sorted_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        sorted_scores.truncate(3);
        sorted_scores
    }
}

#[derive(Debug, Clone)]
pub struct LanguageManager {
    available_languages: Vec<Language>,
    current_language: Language,
    custom_dictionaries: HashMap<String, PathBuf>,
}

impl LanguageManager {
    pub fn new() -> Self {
        let mut manager = Self {
            available_languages: Language::all(),
            current_language: Language::English,
            custom_dictionaries: HashMap::new(),
        };
        
        // Scan for available dictionary files
        manager.scan_dictionaries();
        
        manager
    }
    
    fn scan_dictionaries(&mut self) {
        let dict_dir = Self::dictionary_dir();
        
        if let Ok(entries) = std::fs::read_dir(&dict_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        // Check for dictionary(lang).txt pattern
                        if let Some(lang_code) = filename
                            .strip_prefix("dictionary(")
                            .and_then(|s| s.strip_suffix(").txt"))
                        {
                            let language = Language::from_code(lang_code);
                            self.custom_dictionaries.insert(lang_code.to_string(), path);
                            
                            // Add to available languages if not already there
                            if !self.available_languages.contains(&language) {
                                self.available_languages.push(language);
                            }
                        }
                    }
                }
            }
        }
        
        // Sort languages by name
        self.available_languages.sort_by_key(|l| l.name().to_string());
    }
    
    pub fn dictionary_dir() -> PathBuf {
        let mut path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        path.push("src");
        path.push("dictionary");
        path
    }
    
    pub fn get_dictionary_path(&self, language: &Language) -> Option<PathBuf> {
        match language {
            Language::Custom(code) => self.custom_dictionaries.get(*code).cloned(),
            lang => {
                if let Some(filename) = lang.dictionary_filename() {
                    let mut path = Self::dictionary_dir();
                    path.push(filename);
                    if path.exists() {
                        Some(path)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        }
    }
    
    pub fn set_language(&mut self, language: Language) {
        self.current_language = language;
    }
    
    pub fn current_language(&self) -> &Language {
        &self.current_language
    }
    
    pub fn available_languages(&self) -> &[Language] {
        &self.available_languages
    }
    
    pub fn add_custom_dictionary(&mut self, path: PathBuf, language_code: String) {
        self.custom_dictionaries.insert(language_code.clone(), path);
        let language = Language::from_code(&language_code);
        
        if !self.available_languages.contains(&language) {
            self.available_languages.push(language);
        }
    }
    
    pub fn detect_language(&self, text: &str) -> Language {
        if text.trim().is_empty() {
            return Language::English;
        }
        
        let scores = Language::detect_from_text(text);
        
        if let Some((detected_lang, score)) = scores.first() {
            if *score > 20.0 {
                return *detected_lang;
            }
        }
        
        Language::English // Default fallback
    }
}