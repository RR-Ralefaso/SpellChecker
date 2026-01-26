use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use unic_langid::LanguageIdentifier;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    Custom(String),
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
            _ => Some(format!("dictionary({}).txt", self.code())),
        }
    }
    
    pub fn from_code(code: &str) -> Self {
        match code {
            "eng" => Language::English,
            "afr" => Language::Afrikaans,
            "fra" => Language::French,
            "spa" => Language::Spanish,
            "deu" => Language::German,
            "zho" => Language::Chinese,
            "ita" => Language::Italian,
            "por" => Language::Portuguese,
            "rus" => Language::Russian,
            "jpn" => Language::Japanese,
            "kor" => Language::Korean,
            "auto" => Language::AutoDetect,
            custom => Language::Custom(custom.to_string()),
        }
    }
    
    pub fn detect_from_text(text: &str) -> Vec<(Language, f32)> {
        let mut scores = HashMap::new();
        
        // Common word detection for each language
        let language_patterns = [
            (Language::English, vec!["the", "and", "that", "have", "for"]),
            (Language::Afrikaans, vec!["die", "en", "het", "vir", "om"]),
            (Language::French, vec!["le", "la", "et", "que", "dans"]),
            (Language::Spanish, vec!["el", "la", "que", "y", "en"]),
            (Language::German, vec!["der", "die", "das", "und", "den"]),
            (Language::Chinese, vec!["的", "是", "在", "了", "和"]),
            (Language::Italian, vec!["il", "la", "che", "e", "di"]),
            (Language::Portuguese, vec!["o", "a", "e", "que", "do"]),
            (Language::Russian, vec!["и", "в", "не", "на", "я"]),
            (Language::Japanese, vec!["の", "に", "を", "は", "が"]),
            (Language::Korean, vec!["이", "가", "을", "를", "은"]),
        ];
        
        let text_lower = text.to_lowercase();
        let total_chars = text.chars().count().max(1);
        
        for (lang, patterns) in language_patterns {
            let mut score = 0.0;
            
            // Check for common words
            for pattern in patterns {
                if text_lower.contains(pattern) {
                    score += 10.0;
                }
            }
            
            // Character frequency analysis
            let char_freq: HashMap<char, f32> = match lang {
                Language::English => [('e', 12.7), ('t', 9.1), ('a', 8.2), ('o', 7.5), ('i', 7.0)]
                    .iter()
                    .map(|(c, freq)| (*c, *freq))
                    .collect(),
                Language::Afrikaans => [('e', 18.9), ('a', 7.2), ('i', 7.0), ('n', 6.9), ('r', 6.6)]
                    .iter()
                    .map(|(c, freq)| (*c, *freq))
                    .collect(),
                Language::French => [('e', 15.9), ('a', 8.4), ('i', 7.3), ('s', 7.3), ('n', 7.1)]
                    .iter()
                    .map(|(c, freq)| (*c, *freq))
                    .collect(),
                Language::German => [('e', 17.4), ('n', 10.5), ('i', 8.0), ('s', 7.6), ('r', 7.3)]
                    .iter()
                    .map(|(c, freq)| (*c, *freq))
                    .collect(),
                _ => HashMap::new(),
            };
            
            for (ch, expected_freq) in char_freq {
                let actual_count = text_lower.chars().filter(|&c| c == ch).count() as f32;
                let actual_freq = (actual_count / total_chars as f32) * 100.0;
                let freq_diff = (expected_freq - actual_freq).abs();
                score += 5.0 / (freq_diff + 1.0);
            }
            
            if score > 0.0 {
                scores.insert(lang, score);
            }
        }
        
        // Sort by score
        let mut sorted_scores: Vec<(Language, f32)> = scores.into_iter().collect();
        sorted_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
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
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("src");
        path.push("dictionary");
        path
    }
    
    pub fn get_dictionary_path(&self, language: &Language) -> Option<PathBuf> {
        match language {
            Language::Custom(code) => self.custom_dictionaries.get(code).cloned(),
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
        let language = Language::Custom(language_code);
        
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
            if *score > 15.0 {
                return *detected_lang;
            }
        }
        
        Language::English // Default fallback
    }
}