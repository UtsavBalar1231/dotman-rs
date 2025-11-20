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
//! - Stage all changes with `-A` flag (modified, deleted, and new files)
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
//! add::execute(&ctx, &["~/.bashrc".to_string()], false, false)?;
//!
//! // Add a directory recursively
//! add::execute(&ctx, &["~/.config".to_string()], false, false)?;
//!
//! // Force add (skip non-existent paths)
//! add::execute(&ctx, &["file.txt".to_string()], true, false)?;
//!
//! // Stage all changes (like git add -A)
//! add::execute(&ctx, &[], false, true)?;
//! # Ok(())
//! # }
//! ```

use crate::DotmanContext;
use crate::commands::context::CommandContext;
use crate::output;
use crate::refs::RefManager;
use crate::storage::snapshots::{SnapshotFile, SnapshotManager};
use crate::storage::{CachedHash, FileEntry};
use crate::tracking::manifest::TrackingManifest;
use crate::utils::{expand_tilde, make_relative, should_ignore};
use anyhow::{Context, Result};
use colored::Colorize;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Load committed files from the HEAD snapshot.
///
/// Returns a map of file paths to their snapshot entries, or an empty map if:
/// - HEAD doesn't point to a commit
/// - HEAD points to the placeholder commit
/// - The snapshot cannot be loaded
fn load_committed_files(ctx: &DotmanContext) -> Result<HashMap<PathBuf, SnapshotFile>> {
    let ref_manager = RefManager::new(ctx.repo_path.clone());
    let placeholder_commit = "0".repeat(40);

    ref_manager
        .get_head_commit()?
        .filter(|commit_id| commit_id != &placeholder_commit)
        .map_or(Ok(HashMap::new()), |commit_id| {
            let snapshot_manager =
                SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);
            Ok(snapshot_manager
                .load_snapshot(&commit_id)
                .map(|snapshot| snapshot.files)
                .unwrap_or_default())
        })
}

/// Stage all changes to tracked files (modified and deleted).
///
/// Similar to `git add -A`, this stages:
/// - All modified tracked files
/// - All deleted tracked files
///
/// Note: This does NOT scan for new untracked files, as that would require
/// walking the entire home directory which is too slow. Use `dot add <path>`
/// to explicitly add new files.
///
/// # Errors
///
/// Returns an error if:
/// - Cannot load the index
/// - Cannot determine home directory
/// - File operations fail
/// - Cannot save the index
fn execute_add_all(ctx: &DotmanContext) -> Result<()> {
    let index_path = ctx.repo_path.join("index.bin");
    let index = ctx.load_concurrent_index()?;
    let home = ctx.get_home_dir()?;

    // Load committed files from HEAD snapshot
    let committed_files = load_committed_files(ctx)?;

    let mut files_to_stage = Vec::new();
    let mut files_to_delete = Vec::new();

    // 1. Check all tracked files (committed + staged) for modifications/deletions
    let tracked_paths: HashSet<PathBuf> = committed_files
        .keys()
        .cloned()
        .chain(index.staged_entries().into_iter().map(|(path, _)| path))
        .collect();

    for tracked_path in tracked_paths {
        let abs_path = if tracked_path.is_relative() {
            home.join(&tracked_path)
        } else {
            tracked_path.clone()
        };

        if !abs_path.exists() {
            // File was deleted
            files_to_delete.push(tracked_path);
        } else if abs_path.is_file() {
            // Check if file was modified - always re-stage to catch modifications
            // Only get cached_hash from staged entries (committed files don't have cache)
            let cached_hash = index
                .get_staged_entry(&tracked_path)
                .and_then(|e| e.cached_hash);

            files_to_stage.push((abs_path, cached_hash));
        }
    }

    // 2. Hash and stage all modified files in parallel
    let total_files = files_to_stage.len();
    let progress = Arc::new(Mutex::new(output::start_progress(
        "Hashing files",
        total_files,
    )));
    let progress_clone = Arc::clone(&progress);

    let entries: Result<Vec<FileEntry>> = files_to_stage
        .par_iter()
        .enumerate()
        .map(|(i, (path, cached_hash))| {
            let result = create_file_entry(path, &home, cached_hash.as_ref());
            if let Ok(mut p) = progress_clone.lock() {
                p.update(i + 1);
            }
            result
        })
        .collect();

    // Finish progress bar after parallel work is done
    drop(progress_clone);
    if let Some(p) = Arc::try_unwrap(progress)
        .ok()
        .and_then(|p| p.into_inner().ok())
    {
        p.finish();
    }
    let entries = entries?;

    let mut modified_count = 0;

    for entry in entries {
        index.stage_entry(entry.clone());
        modified_count += 1;
        println!("  {} {}", "modified:".yellow(), entry.path.display());
    }

    // 3. Mark deleted files
    let deleted_count = files_to_delete.len();
    for path in &files_to_delete {
        index.mark_deleted(path);
        println!("  {} {}", "deleted:".red(), path.display());
    }

    // 4. Save the index
    index.save(&index_path)?;

    // 5. Print summary
    let total = modified_count + deleted_count;
    if total > 0 {
        output::success(&format!(
            "Staged {total} file(s): {modified_count} modified, {deleted_count} deleted"
        ));
    } else {
        output::info("No changes to stage");
    }

    Ok(())
}

/// Stage files for tracking in the next commit.
///
/// Recursively processes directories and respects ignore patterns.
/// With `force=true`, non-existent paths are skipped rather than erroring.
/// With `all=true`, stages all changes (modified, deleted, and new files).
///
/// # Errors
///
/// Returns an error if:
/// - A path does not exist and `force` is `false`
/// - The `-A` flag is used with path arguments
/// - Cannot read directory entries during recursive traversal
/// - Cannot create file entries (metadata, hashing, or path resolution failures)
/// - Cannot save the index after staging
#[allow(clippy::too_many_lines)] // Complex command with sequential state management and parallel processing
pub fn execute(ctx: &DotmanContext, paths: &[String], force: bool, all: bool) -> Result<()> {
    ctx.ensure_initialized()?;

    // Handle -A flag
    if all {
        if !paths.is_empty() {
            return Err(anyhow::anyhow!(
                "Cannot specify paths with -A flag. Use 'dot add -A' to stage all changes."
            ));
        }
        return execute_add_all(ctx);
    }

    let index_path = ctx.repo_path.join("index.bin");
    let index = ctx.load_concurrent_index()?;

    // Load tracking manifest to record user's tracking intent
    let mut manifest = TrackingManifest::load(&ctx.repo_path)?;

    // Load committed files from HEAD snapshot
    let committed_files = load_committed_files(ctx)?;

    // Extract threshold once to avoid recreating context for every file
    let large_file_threshold = ctx.config.tracking.large_file_threshold;

    let mut files_to_add = Vec::new();
    let home = ctx.get_home_dir()?;

    for path_str in paths {
        let path = expand_tilde(path_str)?;

        if !path.exists() {
            if !force {
                return Err(anyhow::anyhow!("Path does not exist: {}", path.display()));
            }
            output::warning(&format!("Skipping non-existent path: {}", path.display()));
            continue;
        }

        if path.is_file() {
            check_special_file_type(&path, large_file_threshold);
            files_to_add.push(path.clone());

            // Record file in tracking manifest (relative to home)
            if let Ok(relative_path) = make_relative(&path, &home) {
                manifest.add_file(relative_path);
            }
        } else if path.is_dir() {
            // Record directory in tracking manifest (relative to home)
            if let Ok(relative_path) = make_relative(&path, &home) {
                manifest.add_directory(relative_path);
            }

            collect_files_from_dir(
                &path,
                &mut files_to_add,
                &ctx.config.tracking.ignore_patterns,
                ctx.config.tracking.follow_symlinks,
                large_file_threshold,
            )?;
        }
    }

    if files_to_add.is_empty() {
        output::info("No files to add");
        // Still save manifest even if no files (e.g., empty directory tracked)
        manifest.save(&ctx.repo_path)?;
        return Ok(());
    }

    let total_files = files_to_add.len();
    let progress = Arc::new(Mutex::new(output::start_progress(
        "Hashing files",
        total_files,
    )));
    let progress_clone = Arc::clone(&progress);

    let entries: Result<Vec<FileEntry>> = files_to_add
        .par_iter()
        .enumerate()
        .map(|(i, path)| {
            // Try to get existing cached hash from staged entries only
            // (committed files don't have cached_hash in snapshots)
            let relative_path = make_relative(path, &home).ok();
            let cached_hash = relative_path
                .as_ref()
                .and_then(|rp| index.get_staged_entry(rp))
                .and_then(|e| e.cached_hash);
            let result = create_file_entry(path, &home, cached_hash.as_ref());
            if let Ok(mut p) = progress_clone.lock() {
                p.update(i + 1);
            }
            result
        })
        .collect();

    // Finish progress bar after parallel work is done
    drop(progress_clone);
    if let Some(p) = Arc::try_unwrap(progress)
        .ok()
        .and_then(|p| p.into_inner().ok())
    {
        p.finish();
    }
    let entries = entries?;

    let mut added_count = 0;
    let mut updated_count = 0;

    for entry in entries {
        let existing_entry = committed_files.get(&entry.path).cloned();
        let is_staged = index.get_staged_entry(&entry.path).is_some();

        if let Some(committed_entry) = existing_entry {
            // File is tracked - only stage if content changed
            if entry.hash != committed_entry.hash {
                index.stage_entry(entry.clone());
                updated_count += 1;
                println!("  {} {}", "modified:".yellow(), entry.path.display());
            }
            // else: unchanged - skip silently
        } else {
            // Not in committed entries - check if staged
            index.stage_entry(entry.clone());
            if is_staged {
                updated_count += 1;
                println!("  {} {}", "updated:".yellow(), entry.path.display());
            } else {
                // New file
                added_count += 1;
                println!("  {} {}", "added:".green(), entry.path.display());
            }
        }
    }

    index.save(&index_path)?;

    // Save tracking manifest to persist user's tracking intent
    manifest.save(&ctx.repo_path)?;

    if added_count > 0 || updated_count > 0 {
        output::success(&format!(
            "Added {added_count} file(s), updated {updated_count} file(s)"
        ));
    } else {
        output::info("No changes made");
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
/// * `large_file_threshold` - Threshold in bytes for large file warnings
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
    large_file_threshold: u64,
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
            check_special_file_type(&file_path, large_file_threshold);
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
/// * `large_file_threshold` - Threshold in bytes for large file warnings
fn check_special_file_type(path: &Path, large_file_threshold: u64) {
    let Ok(metadata) = std::fs::metadata(path) else {
        return;
    };

    // Check Unix-specific file types
    #[cfg(unix)]
    check_unix_special_types(path, &metadata);

    // Common checks for all platforms
    check_file_size(path, &metadata, large_file_threshold);
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
        output::warning(&format!("Warning: {} is a block device", path.display()));
    } else if file_type.is_char_device() {
        output::warning(&format!(
            "Warning: {} is a character device",
            path.display()
        ));
    } else if file_type.is_fifo() {
        output::warning(&format!(
            "Warning: {} is a named pipe (FIFO)",
            path.display()
        ));
    } else if file_type.is_socket() {
        output::warning(&format!("Warning: {} is a socket", path.display()));
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
/// * `threshold` - Large file threshold in bytes
#[allow(clippy::cast_precision_loss)]
fn check_file_size(path: &Path, metadata: &std::fs::Metadata, threshold: u64) {
    const MB: f64 = 1_048_576.0;

    if metadata.len() > threshold {
        let size_mb = metadata.len() as f64 / MB;
        output::warning(&format!(
            "Warning: {} is very large ({:.2} MB)",
            path.display(),
            size_mb
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
