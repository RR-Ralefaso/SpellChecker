use regex::Regex;
use std::collections::HashMap;
use once_cell::sync::Lazy;

// Compile regex only once for better performance
static WORD_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b[\p{L}0-9'-]+\b").unwrap()
});

// New regex for CJK languages (Chinese, Japanese, Korean)
static CJK_WORD_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[\p{Han}\p{Hiragana}\p{Katakana}\p{Hangul}]+|[\p{L}0-9'-]+").unwrap()
});

/// Extract words from text based on language
pub fn extract_words(text: &str, is_cjk: bool) -> Vec<String> {
    let regex = if is_cjk { &CJK_WORD_REGEX } else { &WORD_REGEX };
    
    regex
        .find_iter(text)
        .map(|mat| {
            if is_cjk {
                mat.as_str().to_string() // CJK: keep original case
            } else {
                mat.as_str().to_lowercase()
            }
        })
        .collect()
}

/// Calculate word frequency
pub fn word_frequency(text: &str, is_cjk: bool) -> HashMap<String, usize> {
    let mut freq = HashMap::new();
    for word in extract_words(text, is_cjk) {
        *freq.entry(word).or_insert(0) += 1;
    }
    freq
}

/// Get most common words
pub fn most_common_words(freq: &HashMap<String, usize>, n: usize) -> Vec<(String, usize)> {
    let mut words: Vec<_> = freq.iter().map(|(w, c)| (w.clone(), *c)).collect();
    words.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    words.truncate(n);
    words
}

/// Calculate reading time
pub fn reading_time(text: &str) -> (usize, usize) {
    let words = extract_words(text, false).len();
    let minutes = words / 200;
    let seconds = ((words % 200) * 60) / 200;
    (minutes, seconds)
}

/// Calculate accuracy percentage
pub fn calculate_accuracy(correct: usize, total: usize) -> f32 {
    if total == 0 {
        100.0
    } else {
        (correct as f32 / total as f32 * 100.0).round()
    }
}

/// Sanitize word by removing invalid characters
pub fn sanitize_word(word: &str) -> String {
    word.trim()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '\'' || *c == '-')
        .collect()
}

/// Check if word is valid (contains at least one letter)
pub fn is_valid_word(word: &str) -> bool {
    !word.is_empty() && word.chars().any(|c| c.is_alphabetic())
}

/// Check if text contains CJK characters
pub fn is_cjk_text(text: &str) -> bool {
    text.chars().any(|c| {
        ('\u{4E00}' <= c && c <= '\u{9FFF}') || // Chinese
        ('\u{3040}' <= c && c <= '\u{309F}') || // Hiragana
        ('\u{30A0}' <= c && c <= '\u{30FF}') || // Katakana
        ('\u{AC00}' <= c && c <= '\u{D7AF}')    // Hangul
    })
}