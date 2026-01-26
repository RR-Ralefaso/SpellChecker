use clap::Parser;           // CLI argument parsing
use colored::*;             // Colored terminal output
use rayon::prelude::*;      // Parallel processing  
use regex::Regex;           // Pattern matching
use std::collections::HashSet;  // Duplicate tracking
use std::fs;                // File operations
use std::io::{self, Write}; // I/O handling
use std::path::Path;        // Path manipulation
use walkdir::WalkDir;       // Recursive directory traversal


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the document file
    document: String,
    
    /// Path to custom dictionary file or directory (optional)
    #[arg(short, long)]
    dictionary: Option<String>,
    
    /// Show suggestions for misspelled words
    #[arg(short, long)]
    suggestions: bool,
    
    /// Case-sensitive checking
    #[arg(short, long)]
    case_sensitive: bool,
    
    /// Include built-in dictionary
    #[arg(short = 'b', long = "builtin", default_value_t = true)]
    include_builtin: bool,
}


