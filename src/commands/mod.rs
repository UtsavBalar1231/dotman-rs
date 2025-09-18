pub mod add;
pub mod branch;
pub mod checkout;
pub mod clean;
pub mod commit;
pub mod config;
pub mod context;
pub mod diff;
pub mod fetch;
pub mod import;
pub mod init;
pub mod log;
pub mod merge;
pub mod pull;
pub mod push;
pub mod reflog;
pub mod remote;
pub mod remote_ops;
pub mod reset;
pub mod restore;
pub mod revert;
pub mod rm;
pub mod show;
pub mod stash;
pub mod status;
pub mod tag;

use colored::Colorize;

pub fn print_success(message: &str) {
    println!("{} {}", "✓".green().bold(), message);
}

pub fn print_error(message: &str) {
    eprintln!("{} {}", "✗".red().bold(), message);
}

pub fn print_info(message: &str) {
    println!("{} {}", "ℹ".blue().bold(), message);
}

pub fn print_warning(message: &str) {
    println!("{} {}", "⚠".yellow().bold(), message);
}
