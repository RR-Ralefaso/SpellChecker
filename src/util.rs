use regex::Regex;
use std::collections::HashMap;
use once_cell::sync::Lazy;

// Compile regex only once for better performance
pub static WORD_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b[\p{L}][\p{L}'-]*\b").unwrap()
});

// New regex for CJK languages (Chinese, Japanese, Korean)
pub static CJK_WORD_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[\p{Han}\p{Hiragana}\p{Katakana}\p{Hangul}]+|[\p{L}][\p{L}'-]*").unwrap()
});

// Regex for programming languages (ignores common code patterns)
pub static CODE_WORD_REGEX: Lazy<Regex> = Lazy::new(|| {
    // Match words but ignore common programming patterns
    Regex::new(r"\b([a-zA-Z][a-zA-Z'-]{2,})\b").unwrap()
});

// Regex to identify code-specific patterns to ignore
static CODE_IGNORE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[A-Z_][A-Z0-9_]*$|^[a-z_][a-z0-9_]*$|^\d+|^0x[0-9a-fA-F]+$|^\.\w+").unwrap()
});

// Common programming keywords to ignore
static CODE_KEYWORDS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "var", "val", "fn", "def", "func", "cls", "obj", "arr", "vec", "str",
        "int", "num", "bool", "float", "double", "char", "byte", "ptr", "ref",
        "mut", "const", "static", "pub", "priv", "prot", "async", "await",
        "try", "catch", "throw", "null", "nil", "none", "some", "ok", "err",
        "true", "false", "self", "this", "super", "new", "del", "inc", "dec",
        "let", "if", "else", "for", "while", "loop", "match", "case", "return",
        "break", "continue", "import", "export", "from", "as", "use", "mod",
        "struct", "enum", "impl", "trait", "where", "type", "interface", "class",
        "function", "const", "let", "var", "public", "private", "protected",
        "extends", "implements", "throws", "try", "catch", "finally", "throw",
        "new", "delete", "sizeof", "typedef", "namespace", "using", "template",
        "virtual", "override", "abstract", "sealed", "internal", "extern", "inline",
        "volatile", "register", "restrict", "atomic", "alignas", "alignof", "decltype",
        "noexcept", "static_assert", "thread_local", "concept", "requires", "co_await",
        "co_return", "co_yield", "module", "import", "export", "consteval", "constinit",
    ]
});

/// Extract words from text based on language and context
pub fn extract_words(text: &str, is_cjk: bool, is_code: bool) -> Vec<String> {
    if is_cjk {
        CJK_WORD_REGEX
            .find_iter(text)
            .map(|mat| mat.as_str().to_string())
            .collect()
    } else if is_code {
        // For code, we want to be more selective
        CODE_WORD_REGEX
            .find_iter(text)
            .map(|mat| mat.as_str())
            .filter(|word| {
                // Filter out common programming constructs
                !CODE_IGNORE_REGEX.is_match(word) &&
                !is_code_keyword(word) &&
                word.len() > 2 && // Ignore very short words
                !is_likely_code_symbol(word) &&
                !is_common_code_pattern(word)
            })
            .map(|word| word.to_lowercase())
            .collect()
    } else {
        WORD_REGEX
            .find_iter(text)
            .map(|mat| mat.as_str().to_lowercase())
            .collect()
    }
}

fn is_code_keyword(word: &str) -> bool {
    CODE_KEYWORDS.contains(&word.to_lowercase().as_str())
}

fn is_likely_code_symbol(word: &str) -> bool {
    // Words that are likely code symbols or short forms
    let code_symbols = [
        "arg", "args", "argv", "env", "envp", "ctx", "cfg", "config",
        "db", "dao", "dto", "vo", "bo", "po", "vo", "dto", "entity",
        "repo", "svc", "service", "ctrl", "controller", "hdl", "handler",
        "util", "utils", "helper", "helpers", "const", "consts", "def", "defs",
        "err", "error", "errors", "msg", "message", "messages", "val", "value",
        "values", "res", "result", "results", "opt", "option", "options",
        "buf", "buffer", "buffers", "mem", "memory", "ptr", "pointer", "ref",
        "reference", "tmp", "temp", "tempory", "info", "information", "stat",
        "status", "state", "states", "cfg", "config", "configuration",
        "init", "initialize", "cleanup", "destroy", "create", "delete",
        "update", "insert", "remove", "add", "get", "set", "is", "has",
        "can", "should", "will", "do", "make", "build", "run", "start",
        "stop", "pause", "resume", "load", "save", "read", "write",
    ];
    
    code_symbols.contains(&word.to_lowercase().as_str())
}

fn is_common_code_pattern(word: &str) -> bool {
    // Patterns common in code that we might want to ignore
    word.contains('_') && word.len() > 5 || // snake_case variables
    word.chars().any(|c| c.is_uppercase()) && word.len() > 3 // CamelCase
}

/// Calculate word frequency with context awareness
pub fn word_frequency(text: &str, is_cjk: bool, is_code: bool) -> HashMap<String, usize> {
    let mut freq = HashMap::new();
    for word in extract_words(text, is_cjk, is_code) {
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
    let words = extract_words(text, false, false).len();
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
    let trimmed = word.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    
    // Preserve apostrophes and hyphens in the middle of words
    let mut result = String::new();
    let chars: Vec<char> = trimmed.chars().collect();
    
    for (i, c) in chars.iter().enumerate() {
        if c.is_alphanumeric() {
            result.push(*c);
        } else if *c == '\'' || *c == '-' {
            // Only keep apostrophes/hyphens if they're in the middle of the word
            if i > 0 && i < chars.len() - 1 && chars[i-1].is_alphabetic() && chars[i+1].is_alphabetic() {
                result.push(*c);
            }
        }
    }
    
    result
}

/// Check if word is valid (contains at least one letter)
pub fn is_valid_word(word: &str) -> bool {
    let trimmed = word.trim();
    !trimmed.is_empty() && 
    trimmed.chars().any(|c| c.is_alphabetic()) &&
    trimmed.len() >= 2
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

/// Check if text looks like code based on common patterns
pub fn is_likely_code(text: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() < 3 {
        return false;
    }
    
    let mut code_indicators = 0;
    
    for line in lines.iter().take(10) {
        let trimmed = line.trim();
        
        if trimmed.contains('{') || trimmed.contains('}') ||
           trimmed.contains(';') && !trimmed.starts_with("//") ||
           trimmed.contains("->") || trimmed.contains("=>") ||
           trimmed.contains("fn ") || trimmed.contains("def ") ||
           trimmed.contains("function ") || trimmed.contains("class ") ||
           trimmed.contains("import ") || trimmed.contains("export ") ||
           trimmed.contains("#include") || trimmed.contains("pub ") ||
           trimmed.contains("let ") || trimmed.contains("const ") ||
           trimmed.contains("var ") || trimmed.contains("return ") {
            code_indicators += 1;
        }
        
        // Check for common code patterns
        if trimmed.contains("=") && !trimmed.contains("==") ||
           trimmed.contains("(") && trimmed.contains(")") ||
           trimmed.contains("[") && trimmed.contains("]") {
            code_indicators += 1;
        }
    }
    
    code_indicators >= 2
}

/// Check if file extension indicates code
pub fn is_code_file(filename: &str) -> bool {
    if let Some(ext) = filename.rsplit('.').next() {
        matches!(ext.to_lowercase().as_str(),
            "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "java" | "cpp" | "c" | "cc" | "cxx" |
            "go" | "rb" | "php" | "cs" | "swift" | "kt" | "scala" | "hs" | "lua" |
            "pl" | "r" | "m" | "f" | "f90" | "f95" | "f03" | "f08" | "v" | "sv" |
            "vhd" | "vhdl" | "asm" | "s" | "sh" | "bash" | "zsh" | "fish" |
            "ps1" | "bat" | "cmd" | "yml" | "yaml" | "toml" | "json" | "xml" | "html" |
            "htm" | "css" | "scss" | "less" | "md" | "markdown" | "tex" | "bib"
        )
    } else {
        false
    }
}

/// Calculate word similarity using Levenshtein distance
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    if a == b { return 0; }
    
    let a_len = a.chars().count();
    let b_len = b.chars().count();
    
    if a_len == 0 { return b_len; }
    if b_len == 0 { return a_len; }
    
    let mut prev_row: Vec<usize> = (0..=b_len).collect();
    let mut curr_row = vec![0; b_len + 1];
    
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    
    for i in 1..=a_len {
        curr_row[0] = i;
        
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            curr_row[j] = std::cmp::min(
                std::cmp::min(prev_row[j] + 1, curr_row[j - 1] + 1),
                prev_row[j - 1] + cost
            );
        }
        
        std::mem::swap(&mut prev_row, &mut curr_row);
    }
    
    prev_row[b_len]
}

/// Get suggestions for a misspelled word
pub fn get_suggestions(word: &str, dictionary_words: &std::collections::HashSet<String>) -> Vec<String> {
    if word.len() <= 1 {
        return Vec::new();
    }
    
    let mut suggestions: Vec<(String, usize)> = dictionary_words
        .iter()
        .filter(|dict_word| {
            // Quick filter: similar length
            let len_diff = (dict_word.len() as isize - word.len() as isize).abs();
            len_diff <= 3
        })
        .map(|dict_word| {
            let distance = levenshtein_distance(word, dict_word);
            (dict_word.clone(), distance)
        })
        .filter(|(_, distance)| *distance <= 2)
        .collect();
    
    suggestions.sort_by_key(|(_, distance)| *distance);
    suggestions.into_iter()
        .take(5)
        .map(|(word, _)| word)
        .collect()
}