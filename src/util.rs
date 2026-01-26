use regex::Regex;
use std::collections::HashMap;
use once_cell::sync::Lazy;

// Compile regex only once for better performance
static WORD_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b[\p{L}0-9'-]+\b").unwrap()
});

pub fn extract_words(text: &str) -> Vec<String> {
    WORD_REGEX
        .find_iter(text)
        .map(|mat| mat.as_str().to_lowercase())
        .collect()
}

pub fn word_frequency(text: &str) -> HashMap<String, usize> {
    let mut freq = HashMap::new();
    for word in extract_words(text) {
        *freq.entry(word).or_insert(0) += 1;
    }
    freq
}

pub fn most_common_words(freq: &HashMap<String, usize>, n: usize) -> Vec<(String, usize)> {
    let mut words: Vec<_> = freq.iter().map(|(w, c)| (w.clone(), *c)).collect();
    words.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    words.truncate(n);
    words
}

pub fn reading_time(text: &str) -> (usize, usize) {
    let words = extract_words(text).len();
    let minutes = words / 200; // Average reading speed: 200 words per minute
    let seconds = ((words % 200) * 60) / 200;
    (minutes, seconds)
}

pub fn calculate_accuracy(correct: usize, total: usize) -> f32 {
    if total == 0 {
        100.0
    } else {
        (correct as f32 / total as f32 * 100.0).round()
    }
}

pub fn sanitize_word(word: &str) -> String {
    word.trim()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '\'' || *c == '-')
        .collect()
}

pub fn is_valid_word(word: &str) -> bool {
    !word.is_empty() && word.chars().any(|c| c.is_alphabetic())
}