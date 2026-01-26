#[cfg(feature = "cli")]
use clap::{Parser, Subcommand};
#[cfg(feature = "cli")]
use colored::*;
#[cfg(feature = "cli")]
use indicatif::{ProgressBar, ProgressStyle};
#[cfg(feature = "cli")]
use spellchecker::{checker::SpellChecker, language::Language, util::*};
#[cfg(feature = "cli")]
use std::path::PathBuf;

#[cfg(feature = "cli")]
#[derive(Parser)]
#[command(name = "spellchecker-cli")]
#[command(about = "Command-line spell checker", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[cfg(feature = "cli")]
#[derive(Subcommand)]
enum Commands {
    /// Check spelling in a file
    Check {
        /// Input file to check
        file: PathBuf,
        
        /// Language to use
        #[arg(short, long, default_value = "eng")]
        language: String,
        
        /// Output suggestions
        #[arg(short, long)]
        suggest: bool,
        
        /// Show statistics
        #[arg(long)]
        stats: bool,
    },
    
    /// Analyze word frequency
    Frequency {
        /// Input file to analyze
        file: PathBuf,
        
        /// Number of top words to show
        #[arg(short, long, default_value_t = 10)]
        top: usize,
    },
    
    /// Create a dictionary from a text file
    CreateDict {
        /// Input text file
        input: PathBuf,
        
        /// Output dictionary file
        output: PathBuf,
        
        /// Language code
        #[arg(short, long)]
        lang: String,
    },
}

#[cfg(feature = "cli")]
fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Check { file, language, suggest, stats } => {
            let content = std::fs::read_to_string(&file)?;
            let language = Language::from_code(&language);
            
            println!("Checking '{}' in {}...", file.display(), language.name());
            
            let mut checker = SpellChecker::new(language)?;
            checker.enable_suggestions(suggest);
            
            let analysis = checker.check_document(&content);
            
            println!("\n{}", "Results:".bold());
            println!("  Total words: {}", analysis.total_words);
            println!("  Misspelled: {}", analysis.misspelled_words);
            println!("  Accuracy: {:.1}%", analysis.accuracy);
            
            if analysis.misspelled_words > 0 {
                println!("\n{}", "Errors found:".red().bold());
                for word in analysis.words.iter().filter(|w| !w.is_correct) {
                    println!("  Line {}: '{}'", word.line, word.word.red());
                    if suggest && !word.suggestions.is_empty() {
                        println!("    Suggestions: {}", word.suggestions.join(", ").green());
                    }
                }
            } else {
                println!("\n{}", "✓ No spelling errors found!".green().bold());
            }
            
            if stats {
                let reading_time = reading_time(&content);
                let freq = word_frequency(&content);
                let common = most_common_words(&freq, 5);
                
                println!("\n{}", "Additional statistics:".bold());
                println!("  Reading time: {} min {} sec", reading_time.0, reading_time.1);
                println!("  Unique words: {}", freq.len());
                println!("  Most common words:");
                for (word, count) in common {
                    println!("    {}: {}", word, count);
                }
            }
        }
        
        Commands::Frequency { file, top } => {
            let content = std::fs::read_to_string(&file)?;
            let freq = word_frequency(&content);
            let common = most_common_words(&freq, top);
            
            println!("Top {} words in '{}':", top, file.display());
            println!("{:-^40}", "");
            println!("{:<20} {:>10}", "Word", "Frequency");
            println!("{:-^40}", "");
            
            for (word, count) in common {
                println!("{:<20} {:>10}", word, count);
            }
            
            let total_words: usize = freq.values().sum();
            println!("{:-^40}", "");
            println!("Total unique words: {}", freq.len());
            println!("Total word count: {}", total_words);
        }
        
        Commands::CreateDict { input, output, lang } => {
            let content = std::fs::read_to_string(&input)?;
            let words = extract_words(&content);
            let unique_words: std::collections::HashSet<_> = words.into_iter().collect();
            
            let pb = ProgressBar::new(unique_words.len() as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                    .unwrap()
                    .progress_chars("#>-"),
            );
            
            let mut dict_content = String::new();
            for word in unique_words {
                dict_content.push_str(&word);
                dict_content.push('\n');
                pb.inc(1);
            }
            
            std::fs::write(&output, dict_content)?;
            pb.finish_with_message("Dictionary created!");
            
            println!("Created dictionary '{}' with {} words", output.display(), unique_words.len());
        }
    }
    
    Ok(())
}

#[cfg(not(feature = "cli"))]
fn main() {
    println!("CLI feature not enabled. Build with --features cli");
}