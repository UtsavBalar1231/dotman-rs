/// File tracking and staging operations.
pub mod add;
/// Branch management operations (create, delete, rename, list).
pub mod branch;
/// Checkout operations to restore files from commits.
pub mod checkout;
/// Clean untracked files and directories.
pub mod clean;
/// Commit operations and snapshot creation.
pub mod commit;
/// Configuration viewing and management.
pub mod config;
/// Shared command context and utilities.
pub mod context;
/// Show differences between commits and working tree.
pub mod diff;
/// Fetch changes from remote repositories.
pub mod fetch;
/// Import configurations from other systems.
pub mod import;
/// Repository initialization.
pub mod init;
/// View commit history.
pub mod log;
/// Merge branches and resolve conflicts.
pub mod merge;
/// Fetch and merge from remote.
pub mod pull;
/// Push changes to remote repository.
pub mod push;
/// Reference log operations.
pub mod reflog;
/// Remote repository management.
pub mod remote;
/// Remote operation utilities.
pub mod remote_ops;
/// Reset repository state to specific commits.
pub mod reset;
/// Restore files from commits.
pub mod restore;
/// Revert specific commits.
pub mod revert;
/// Remove files from tracking.
pub mod rm;
/// Show commit and file information.
pub mod show;
/// Stash and unstash changes.
pub mod stash;
/// Show working tree status.
pub mod status;
/// Tag management.
pub mod tag;

use colored::Colorize;

/// Prints a success message with a green checkmark to stdout.
///
/// # Arguments
///
/// * `message` - The success message to display
pub fn print_success(message: &str) {
    println!("{} {}", "✓".green().bold(), message);
}

/// Prints an error message with a red X to stderr.
///
/// # Arguments
///
/// * `message` - The error message to display
pub fn print_error(message: &str) {
    eprintln!("{} {}", "✗".red().bold(), message);
}

/// Prints an informational message with a blue icon to stdout.
///
/// # Arguments
///
/// * `message` - The informational message to display
pub fn print_info(message: &str) {
    println!("{} {}", "ℹ".blue().bold(), message);
}

/// Prints a warning message with a yellow icon to stdout.
///
/// # Arguments
///
/// * `message` - The warning message to display
pub fn print_warning(message: &str) {
    println!("{} {}", "⚠".yellow().bold(), message);
}
