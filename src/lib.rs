pub mod checker;
pub mod dictionary;
pub mod editor;
pub mod gui;
pub mod language;
pub mod sidebar;
pub mod theme;
pub mod utils;

pub use checker::{DocumentAnalysis, SpellChecker, WordCheck};
pub use dictionary::DictionaryManager;
pub use language::{Language, LanguageManager};

#[derive(Debug, thiserror::Error)]
pub enum SpellCheckerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Invalid dictionary path: {0}")]
    InvalidDictionaryPath(String),
    
    #[error("Empty dictionary")]
    EmptyDictionary,
    
    #[error("Invalid document encoding")]
    InvalidEncoding,
    
    #[error("Language error: {0}")]
    Language(String),
    
    #[error("GUI error: {0}")]
    Gui(String),
}

pub type Result<T> = std::result::Result<T, SpellCheckerError>;