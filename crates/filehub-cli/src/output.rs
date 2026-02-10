//! Table and JSON output formatting for CLI commands.

use serde::Serialize;
use tabled::{Table, Tabled};

/// Output format selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    /// Human-readable table
    Table,
    /// JSON output
    Json,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Table
    }
}

/// Print a list of items in the selected format
pub fn print_list<T: Serialize + Tabled>(items: &[T], format: OutputFormat) {
    match format {
        OutputFormat::Table => {
            if items.is_empty() {
                println!("No results found.");
            } else {
                let table = Table::new(items).to_string();
                println!("{}", table);
            }
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(items).unwrap_or_else(|_| "[]".to_string());
            println!("{}", json);
        }
    }
}

/// Print a single item in the selected format
pub fn print_item<T: Serialize + std::fmt::Debug>(item: &T, format: OutputFormat) {
    match format {
        OutputFormat::Table => {
            println!("{:#?}", item);
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(item).unwrap_or_else(|_| "{}".to_string());
            println!("{}", json);
        }
    }
}

/// Print a success message
pub fn print_success(msg: &str) {
    println!("✓ {}", msg);
}

/// Print a warning message
pub fn print_warning(msg: &str) {
    println!("⚠ {}", msg);
}

/// Print an error message
pub fn print_error(msg: &str) {
    eprintln!("✗ {}", msg);
}

/// Print a key-value pair
pub fn print_kv(key: &str, value: &str) {
    println!("  {:<24} {}", format!("{}:", key), value);
}
