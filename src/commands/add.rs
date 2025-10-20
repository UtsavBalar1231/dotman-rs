//! File addition and staging operations.
//!
//! This module provides functionality for staging files to be tracked by dotman,
//! similar to `git add`. It handles:
//!
//! - Recursive directory processing
//! - Ignore pattern matching
//! - Special file type detection (devices, sockets, large files, sensitive files)
//! - Parallel file hashing with cache optimization
//! - Force mode for non-existent paths
//!
//! # Examples
//!
//! ```no_run
//! use dotman::DotmanContext;
//! use dotman::commands::add;
//!
//! # fn main() -> anyhow::Result<()> {
//! let ctx = DotmanContext::new()?;
//!
//! // Add a single file
//! add::execute(&ctx, &["~/.bashrc".to_string()], false)?;
//!
//! // Add a directory recursively
//! add::execute(&ctx, &["~/.config".to_string()], false)?;
//!
//! // Force add (skip non-existent paths)
//! add::execute(&ctx, &["file.txt".to_string()], true)?;
//! # Ok(())
//! # }
//! ```

use crate::DotmanContext;
use crate::commands::context::CommandContext;
use crate::storage::{CachedHash, FileEntry};
use crate::utils::{expand_tilde, make_relative, should_ignore};
use anyhow::{Context, Result};
use colored::Colorize;
use rayon::prelude::*;
use std::path::{Path, PathBuf};

/// Stage files for tracking in the next commit.
///
/// Recursively processes directories and respects ignore patterns.
/// With `force=true`, non-existent paths are skipped rather than erroring.
///
/// # Errors
///
/// Returns an error if:
/// - A path does not exist and `force` is `false`
/// - Cannot read directory entries during recursive traversal
/// - Cannot create file entries (metadata, hashing, or path resolution failures)
/// - Cannot save the index after staging
pub fn execute(ctx: &DotmanContext, paths: &[String], force: bool) -> Result<()> {
    ctx.ensure_initialized()?;

    let index_path = ctx.repo_path.join("index.bin");
    let index = ctx.load_concurrent_index()?;

    let mut files_to_add = Vec::new();

    for path_str in paths {
        let path = expand_tilde(path_str)?;

        if !path.exists() {
            if !force {
                return Err(anyhow::anyhow!("Path does not exist: {}", path.display()));
            }
            super::print_warning(&format!("Skipping non-existent path: {}", path.display()));
            continue;
        }

        if path.is_file() {
            check_special_file_type(&path);
            files_to_add.push(path);
        } else if path.is_dir() {
            collect_files_from_dir(
                &path,
                &mut files_to_add,
                &ctx.config.tracking.ignore_patterns,
                ctx.config.tracking.follow_symlinks,
            )?;
        }
    }

    if files_to_add.is_empty() {
        super::print_info("No files to add");
        return Ok(());
    }

    let home = ctx.get_home_dir()?;

    let entries: Result<Vec<FileEntry>> = files_to_add
        .par_iter()
        .map(|path| {
            // Try to get existing cached hash from index
            let relative_path = make_relative(path, &home).ok();
            let cached_hash = relative_path
                .as_ref()
                .and_then(|rp| index.get_staged_entry(rp).or_else(|| index.get_entry(rp)))
                .and_then(|e| e.cached_hash);
            create_file_entry(path, &home, cached_hash.as_ref())
        })
        .collect();

    let entries = entries?;

    let mut added_count = 0;
    let mut updated_count = 0;

    for entry in entries {
        let is_tracked = index.get_entry(&entry.path).is_some();
        let is_staged = index.get_staged_entry(&entry.path).is_some();

        index.stage_entry(entry.clone());

        if is_tracked {
            updated_count += 1;
            println!("  {} {}", "modified:".yellow(), entry.path.display());
        } else if is_staged {
            updated_count += 1;
            println!("  {} {}", "updated:".yellow(), entry.path.display());
        } else {
            added_count += 1;
            println!("  {} {}", "added:".green(), entry.path.display());
        }
    }

    index.save_merge(&index_path)?;

    if added_count > 0 || updated_count > 0 {
        super::print_success(&format!(
            "Added {added_count} file(s), updated {updated_count} file(s)"
        ));
    } else {
        super::print_info("No changes made");
    }

    Ok(())
}

/// Recursively collect files from a directory, respecting ignore patterns.
///
/// This function walks through a directory tree and collects all file paths
/// that pass the ignore pattern filter. It also performs special file type
/// checking on each discovered file.
///
/// # Arguments
///
/// * `dir` - Directory to traverse
/// * `files` - Mutable vector to collect file paths into
/// * `ignore_patterns` - Patterns to exclude from collection
/// * `follow_symlinks` - Whether to follow symbolic links
///
/// # Errors
///
/// Returns an error if:
/// - Cannot read directory entries
/// - Directory traversal fails due to permissions or I/O errors
fn collect_files_from_dir(
    dir: &Path,
    files: &mut Vec<PathBuf>,
    ignore_patterns: &[String],
    follow_symlinks: bool,
) -> Result<()> {
    for entry in walkdir::WalkDir::new(dir)
        .follow_links(follow_symlinks)
        .into_iter()
        .filter_entry(|e| !should_ignore(e.path(), ignore_patterns))
    {
        let entry =
            entry.with_context(|| format!("Failed to read directory: {}", dir.display()))?;
        if entry.file_type().is_file() {
            let file_path = entry.path().to_path_buf();
            check_special_file_type(&file_path);
            files.push(file_path);
        }
    }
    Ok(())
}

/// Check for special file types and issue warnings.
///
/// This function performs platform-specific checks for special file types
/// (block devices, character devices, sockets, FIFOs on Unix) and common
/// checks for large files and potentially sensitive filenames.
///
/// Warnings are printed to the user but do not prevent the file from being added.
///
/// # Arguments
///
/// * `path` - Path to the file to check
fn check_special_file_type(path: &Path) {
    let Ok(metadata) = std::fs::metadata(path) else {
        return;
    };

    // Check Unix-specific file types
    #[cfg(unix)]
    check_unix_special_types(path, &metadata);

    // Common checks for all platforms
    check_file_size(path, &metadata);
    check_sensitive_filename(path);
}

/// Check for Unix-specific special file types.
///
/// On Unix systems, this function checks if a file is a block device,
/// character device, FIFO (named pipe), or socket, and warns the user
/// if any of these special types are detected.
///
/// # Arguments
///
/// * `path` - Path to the file
/// * `metadata` - File metadata containing type information
#[cfg(unix)]
fn check_unix_special_types(path: &Path, metadata: &std::fs::Metadata) {
    use std::os::unix::fs::FileTypeExt;

    let file_type = metadata.file_type();

    if file_type.is_block_device() {
        super::print_warning(&format!("Warning: {} is a block device", path.display()));
    } else if file_type.is_char_device() {
        super::print_warning(&format!(
            "Warning: {} is a character device",
            path.display()
        ));
    } else if file_type.is_fifo() {
        super::print_warning(&format!(
            "Warning: {} is a named pipe (FIFO)",
            path.display()
        ));
    } else if file_type.is_socket() {
        super::print_warning(&format!("Warning: {} is a socket", path.display()));
    }
}

/// Check if a file exceeds the large file threshold and warn the user.
///
/// The threshold is configurable via [`crate::config::TrackingConfig::large_file_threshold`].
/// Default is 100 MB.
///
/// # Arguments
///
/// * `path` - Path to the file
/// * `metadata` - File metadata containing size information
#[allow(clippy::cast_precision_loss)]
fn check_file_size(path: &Path, metadata: &std::fs::Metadata) {
    use crate::DotmanContext;

    const MB: f64 = 1_048_576.0;

    // Try to get threshold from config, fall back to default if unavailable
    let threshold = DotmanContext::new().ok().map_or(100 * 1024 * 1024, |ctx| {
        ctx.config.tracking.large_file_threshold
    });

    if metadata.len() > threshold {
        let size_mb = metadata.len() as f64 / MB;
        super::print_warning(&format!(
            "Warning: {} is very large ({:.2} MB)",
            path.display(),
            size_mb
        ));
    }
}

/// Check if a filename contains patterns that suggest sensitive content.
///
/// This function scans the filename for common patterns associated with
/// sensitive data (passwords, secrets, private keys, certificates) and
/// warns the user if any are detected.
///
/// Patterns checked: "password", "secret", "key", ".pem", ".key", ".pfx"
///
/// # Arguments
///
/// * `path` - Path to the file to check
fn check_sensitive_filename(path: &Path) {
    const SENSITIVE_PATTERNS: &[&str] = &["password", "secret", "key", ".pem", ".key", ".pfx"];

    if let Some(name) = path.file_name().and_then(|n| n.to_str())
        && SENSITIVE_PATTERNS
            .iter()
            .any(|&pattern| name.contains(pattern))
    {
        super::print_warning(&format!(
            "Warning: {} may contain sensitive information",
            path.display()
        ));
    }
}

/// Build `FileEntry` with hash, metadata, and relative path.
///
/// This function creates a complete file entry suitable for adding to the index.
/// It computes the file hash (using cache if available), extracts metadata,
/// and converts the path to be relative to the home directory.
///
/// # Arguments
///
/// * `path` - Absolute path to the file
/// * `home` - Home directory path for making relative paths
/// * `cached_hash` - Optional cached hash for performance optimization
///
/// # Errors
///
/// Returns an error if:
/// - Cannot read file metadata
/// - Cannot hash the file contents
/// - Cannot get file modification time
/// - File modification time is invalid or too large
/// - Cannot make path relative to home directory
///
/// # Examples
///
/// ```no_run
/// use dotman::commands::add::create_file_entry;
/// use std::path::PathBuf;
///
/// # fn main() -> anyhow::Result<()> {
/// let path = PathBuf::from("/home/user/.bashrc");
/// let home = PathBuf::from("/home/user");
/// let entry = create_file_entry(&path, &home, None)?;
/// # Ok(())
/// # }
/// ```
pub fn create_file_entry(
    path: &Path,
    home: &Path,
    cached_hash: Option<&CachedHash>,
) -> Result<FileEntry> {
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("Failed to get metadata for: {}", path.display()))?;

    let (hash, cache) = crate::storage::file_ops::hash_file(path, cached_hash)
        .with_context(|| format!("Failed to hash file: {}", path.display()))?;

    let modified = i64::try_from(
        metadata
            .modified()
            .context("Failed to get file modification time")?
            .duration_since(std::time::UNIX_EPOCH)
            .context("Invalid file modification time")?
            .as_secs(),
    )
    .context("File modification time too large")?;

    // Use the cross-platform permissions module
    let permissions = crate::utils::permissions::FilePermissions::from_path(path)?;
    let mode = permissions.mode();

    let relative_path = make_relative(path, home)
        .with_context(|| format!("Failed to make path relative: {}", path.display()))?;

    Ok(FileEntry {
        path: relative_path,
        hash,
        size: metadata.len(),
        modified,
        mode,
        cached_hash: Some(cache),
    })
}
