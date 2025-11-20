//! File removal and untracking operations.
//!
//! This module provides functionality for removing files from dotman tracking,
//! similar to `git rm`. It handles:
//!
//! - Index-only removal (--cached mode)
//! - Glob pattern matching
//! - Recursive directory removal
//! - Dry-run mode for previewing changes
//! - Force mode for non-tracked files
//!
//! # Safety
//!
//! For data safety, this module NEVER deletes actual files from disk.
//! It only removes entries from the tracking index.
//!
//! # Examples
//!
//! ```no_run
//! use dotman::DotmanContext;
//! use dotman::commands::rm::{self, RmOptions};
//!
//! # fn main() -> anyhow::Result<()> {
//! let ctx = DotmanContext::new()?;
//!
//! // Remove a file from tracking
//! rm::execute(&ctx, &["file.txt".to_string()], &RmOptions::default())?;
//!
//! // Remove with glob pattern
//! rm::execute(&ctx, &["*.tmp".to_string()], &RmOptions::default())?;
//!
//! // Dry run (preview)
//! rm::execute(&ctx, &["file.txt".to_string()], &RmOptions { dry_run: true, ..Default::default() })?;
//! # Ok(())
//! # }
//! ```

use crate::output;
use crate::storage::index::Index;
use crate::{DotmanContext, INDEX_FILE};
use anyhow::{Context, Result};
use colored::Colorize;
use glob::Pattern;
use std::fs;
use std::path::{Path, PathBuf};

/// Options for the rm command
#[derive(Clone, Copy, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct RmOptions {
    /// Only remove from index, not from working tree
    pub cached: bool,
    /// Allow removing non-tracked files
    pub force: bool,
    /// Recursively remove directories
    pub recursive: bool,
    /// Preview changes without removing
    pub dry_run: bool,
}

/// Remove files from tracking (index only, never deletes actual files)
///
/// Similar to git rm, this removes files from being tracked. With --cached,
/// only removes from index. Without --cached, removes from both index and
/// working directory (but we protect against data loss).
///
/// # Errors
///
/// Returns an error if:
/// - The repository is not initialized
/// - No files match the specified patterns
/// - File operations fail
/// - Index update fails
pub fn execute(ctx: &DotmanContext, paths: &[String], options: &RmOptions) -> Result<()> {
    ctx.check_repo_initialized()?;

    if options.dry_run {
        output::info("Dry run mode - no files will be removed");
    }

    let index_path = ctx.repo_path.join(INDEX_FILE);
    let mut index = Index::load(&index_path)?;

    let mut removed_count = 0;
    let mut not_found_count = 0;

    // Get home directory for making paths relative
    let home = dirs::home_dir().context("Could not find home directory")?;

    // Expand paths with glob patterns and recursive directory handling
    let mut expanded_paths = Vec::new();

    for path_str in paths {
        if path_str.contains('*') || path_str.contains('?') || path_str.contains('[') {
            // Handle glob pattern
            if let Ok(pattern) = Pattern::new(path_str) {
                // Match against files in index
                for indexed_path in index.entries.keys() {
                    if pattern.matches(&indexed_path.to_string_lossy()) {
                        expanded_paths.push(indexed_path.clone());
                    }
                }
            } else {
                output::warning(&format!("Invalid glob pattern: {path_str}"));
            }
        } else {
            let path = PathBuf::from(path_str);

            if options.recursive && path.is_dir() {
                // Add all files in directory recursively
                expand_directory_recursive(&path, &mut expanded_paths)?;
            } else {
                expanded_paths.push(path);
            }
        }
    }

    // Remove duplicates
    expanded_paths.sort();
    expanded_paths.dedup();

    for path in expanded_paths {
        let index_path = if path.is_absolute() {
            path.strip_prefix(&home).unwrap_or(&path).to_path_buf()
        } else {
            path.clone()
        };

        let in_index = index.get_entry(&index_path).is_some();

        if !in_index && !options.force {
            output::warning(&format!("File not tracked: {}", path.display()));
            not_found_count += 1;
            continue;
        }

        if options.dry_run {
            println!(
                "  {} {} (dry run)",
                "would remove:".yellow(),
                path.display()
            );
            removed_count += 1;
            continue;
        }

        // Check if file is tracked
        if index.entries.contains_key(&index_path) {
            // Mark the file as deleted
            index.mark_deleted(&index_path);
            println!("  {} {}", "removed:".red(), path.display());
            removed_count += 1;
        } else if index.staged_entries.remove(&index_path).is_some() {
            // File was only in staging area, not committed yet
            println!("  {} {}", "removed:".red(), path.display());
            removed_count += 1;
        }

        // For safety, we never delete actual files from disk
        if !options.cached {}
    }

    // Save updated index (only if not in dry run mode)
    if removed_count > 0 && !options.dry_run {
        index.save(&index_path)?;
        if options.cached {
            output::success(&format!(
                "Removed {removed_count} file(s) from index (files unchanged on disk)"
            ));
        } else {
            output::success(&format!("Removed {removed_count} file(s) from tracking"));
        }
    } else if removed_count > 0 && options.dry_run {
        output::success(&format!("Would remove {removed_count} file(s) (dry run)"));
    }

    if not_found_count > 0 {
        output::info(&format!("{not_found_count} file(s) were not tracked"));
    }

    Ok(())
}

/// Recursively expand a directory and collect all file paths.
///
/// This function traverses a directory tree depth-first and collects
/// all file paths (not directories) into the provided vector.
///
/// # Arguments
///
/// * `dir` - Directory to expand
/// * `paths` - Mutable vector to collect file paths into
///
/// # Errors
///
/// Returns an error if:
/// - Cannot read directory entries
/// - Directory traversal fails due to permissions
/// - I/O errors occur during traversal
fn expand_directory_recursive(dir: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Recursively expand subdirectories
                expand_directory_recursive(&path, paths)?;
            } else {
                paths.push(path);
            }
        }
    }
    Ok(())
}
