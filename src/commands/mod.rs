pub mod add;
pub mod checkout;
pub mod commit;
pub mod diff;
pub mod init;
pub mod log;
pub mod pull;
pub mod push;
pub mod reset;
pub mod rm;
pub mod show;
pub mod status;

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
