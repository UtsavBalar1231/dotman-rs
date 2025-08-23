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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print_success() {
        // Test that print_success outputs to stdout with correct formatting
        print_success("Test success message");
        // This function prints to stdout, so we're mainly testing it doesn't panic
    }

    #[test]
    fn test_print_error() {
        // Test that print_error outputs to stderr with correct formatting
        print_error("Test error message");
        // This function prints to stderr, so we're mainly testing it doesn't panic
    }

    #[test]
    fn test_print_info() {
        // Test that print_info outputs to stdout with correct formatting
        print_info("Test info message");
        // This function prints to stdout, so we're mainly testing it doesn't panic
    }

    #[test]
    fn test_print_warning() {
        // Test that print_warning outputs to stdout with correct formatting
        print_warning("Test warning message");
        // This function prints to stdout, so we're mainly testing it doesn't panic
    }

    #[test]
    fn test_print_functions_with_special_chars() {
        // Test with special characters and Unicode
        print_success("Success with émojis 🎉");
        print_error("Error with special chars: <>&\"'");
        print_info("Info with newline\nand tabs\t\there");
        print_warning("Warning with 中文 characters");
    }

    #[test]
    fn test_print_functions_with_empty_strings() {
        // Test with empty strings
        print_success("");
        print_error("");
        print_info("");
        print_warning("");
    }

    #[test]
    fn test_print_functions_with_long_messages() {
        let long_message = "a".repeat(1000);
        print_success(&long_message);
        print_error(&long_message);
        print_info(&long_message);
        print_warning(&long_message);
    }
}
