use crate::output;
use crate::refs::resolver::RefResolver;
use crate::storage::FileEntry;
use crate::storage::index::Index;
use crate::storage::snapshots::SnapshotManager;
use crate::{DotmanContext, INDEX_FILE};
use anyhow::{Context, Result};
use colored::Colorize;
use std::path::{Path, PathBuf};

/// Options for the reset command
#[derive(Clone, Copy, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct ResetOptions {
    /// Hard reset: update index and working directory
    pub hard: bool,
    /// Soft reset: only move HEAD, keep index and working directory
    pub soft: bool,
    /// Mixed reset: update index but not working directory (default)
    pub mixed: bool,
    /// Keep reset: reset HEAD and index but keep working directory changes
    pub keep: bool,
}

/// Execute reset command - reset current HEAD to the specified state
///
/// # Errors
///
/// Returns an error if:
/// - The repository is not initialized
/// - The specified commit cannot be resolved
/// - Multiple reset modes are specified
/// - File operations fail during hard reset
/// - Index update fails
#[allow(clippy::too_many_lines)]
pub fn execute(
    ctx: &DotmanContext,
    commit: &str,
    options: &ResetOptions,
    paths: &[String],
) -> Result<()> {
    ctx.check_repo_initialized()?;

    // Count how many modes are specified
    let mode_count = [options.hard, options.soft, options.mixed, options.keep]
        .iter()
        .filter(|&&x| x)
        .count();
    if mode_count > 1 {
        return Err(anyhow::anyhow!(
            "Cannot use multiple reset modes simultaneously"
        ));
    }

    // If paths are specified, this is a file-specific reset
    if !paths.is_empty() {
        return reset_files(ctx, commit, paths);
    }

    // Use the reference resolver to handle HEAD, HEAD~n, branches, and short hashes
    let resolver = RefResolver::new(ctx.repo_path.clone());
    let commit_id = resolver.resolve(commit)?;

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    let _snapshot = snapshot_manager
        .load_snapshot(&commit_id)
        .with_context(|| format!("Failed to load commit: {commit_id}"))?;

    if options.hard {
        // Hard reset: update index and working directory
        output::info(&format!(
            "Hard reset to commit {}",
            commit_id[..8.min(commit_id.len())].yellow()
        ));

        // Restore files to working directory
        let home = dirs::home_dir().context("Could not find home directory")?;
        snapshot_manager.restore_snapshot(&commit_id, &home, None)?;

        // Clear the staging area - files are now in the working directory and snapshot
        let index = Index::new();
        let index_path = ctx.repo_path.join(INDEX_FILE);
        index.save(&index_path)?;

        output::success(&format!(
            "Hard reset complete. Working directory and index updated to match commit {}",
            commit_id[..8.min(commit_id.len())].yellow()
        ));
    } else if options.soft {
        // Soft reset: only move HEAD, keep index and working directory
        output::info(&format!(
            "Soft reset to commit {}",
            commit_id[..8.min(commit_id.len())].yellow()
        ));

        output::success(&format!(
            "Soft reset complete. HEAD now points to commit {}",
            commit_id[..8.min(commit_id.len())].yellow()
        ));
    } else if options.keep {
        // Keep reset: reset HEAD and index but keep working directory changes
        output::info(&format!(
            "Keep reset to commit {}",
            commit_id[..8.min(commit_id.len())].yellow()
        ));

        // Clear the staging area - committed files are in snapshots
        let index = Index::new();
        let index_path = ctx.repo_path.join(INDEX_FILE);
        index.save(&index_path)?;

        output::success(&format!(
            "Keep reset complete. Local changes preserved, HEAD now points to {}",
            commit_id[..8.min(commit_id.len())].yellow()
        ));
    } else {
        // Mixed reset (default or explicit): update index but not working directory
        output::info(&format!(
            "Mixed reset to commit {}",
            commit_id[..8.min(commit_id.len())].yellow()
        ));

        // Clear the staging area - committed files are in snapshots
        let index = Index::new();
        let index_path = ctx.repo_path.join(INDEX_FILE);
        index.save(&index_path)?;

        output::success(&format!(
            "Mixed reset complete. Index updated to match commit {}",
            commit_id[..8.min(commit_id.len())].yellow()
        ));
    }

    // Update HEAD to point to the new commit
    update_head(ctx, &commit_id)?;

    Ok(())
}

/// Reset specific files to their state in a given commit
///
/// This function performs a file-specific reset operation, updating the index
/// for the specified files to match their state in the target commit. Unlike
/// a full reset, this only affects the specified files and leaves other files
/// and the HEAD pointer unchanged.
///
/// # Arguments
///
/// * `ctx` - The dotman context containing repository configuration
/// * `commit` - The commit reference to reset files to (e.g., "HEAD", "HEAD~1", branch name, commit hash)
/// * `paths` - Slice of file paths to reset (can be absolute or relative)
///
/// # Returns
///
/// Returns `Ok(())` if the reset operation succeeds, or an error if:
/// - The commit reference cannot be resolved
/// - The snapshot for the commit cannot be loaded
/// - The index file cannot be read or written
/// - The home directory cannot be determined
///
/// # Behavior
///
/// For each specified file:
/// - If the file exists in the target commit, the index entry is updated to match that commit
/// - If the file doesn't exist in the target commit, it is removed from the index (unstaged)
/// - Files not present in the current index generate a warning
///
/// The working directory is not modified; only the index is updated.
fn reset_files(ctx: &DotmanContext, commit: &str, paths: &[String]) -> Result<()> {
    output::info(&format!(
        "Resetting {} file(s) to {}",
        paths.len(),
        if commit == "HEAD" {
            "HEAD"
        } else {
            &commit[..8.min(commit.len())]
        }
    ));

    // Resolve the commit
    let resolver = RefResolver::new(ctx.repo_path.clone());
    let commit_id = resolver.resolve(commit)?;

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);
    let snapshot = snapshot_manager
        .load_snapshot(&commit_id)
        .with_context(|| format!("Failed to load commit: {commit_id}"))?;

    // Load current index
    let index_path = ctx.repo_path.join(INDEX_FILE);
    let mut index = Index::load(&index_path)?;

    // Get home directory for path resolution
    let home = dirs::home_dir().context("Could not find home directory")?;

    let mut reset_count = 0;
    let mut not_found_count = 0;

    for path_str in paths {
        let path = PathBuf::from(path_str);

        let index_path = if path.is_absolute() {
            path.strip_prefix(&home).unwrap_or(&path).to_path_buf()
        } else {
            path.clone()
        };

        if let Some(file) = snapshot.files.get(&index_path) {
            // Update index with file from target commit
            let entry = create_file_entry_with_metadata(
                &index_path,
                &file.hash,
                file.mode,
                &home,
                snapshot.commit.timestamp,
                false, // Working directory not modified
            )?;
            index.stage_entry(entry);

            println!("  {} {}", "reset:".green(), index_path.display());
            reset_count += 1;
        } else {
            // File doesn't exist in target commit - remove from index (unstage)
            if index.staged_entries.remove(&index_path).is_some() {
                println!("  {} {}", "unstaged:".yellow(), index_path.display());
                reset_count += 1;
            } else {
                output::warning(&format!("File not in index: {}", path.display()));
                not_found_count += 1;
            }
        }
    }

    // Save updated index
    if reset_count > 0 {
        index.save(&index_path)?;
        output::success(&format!("Reset {reset_count} file(s)"));
    }

    if not_found_count > 0 {
        output::info(&format!("{not_found_count} file(s) were not in the index"));
    }

    Ok(())
}

/// Create a `FileEntry` from snapshot file info using actual disk metadata
///
/// This helper builds a `FileEntry` with correct metadata from the actual file on disk,
/// enabling hash caching and accurate status detection.
///
/// # Arguments
///
/// * `path` - Relative path of the file
/// * `hash` - File content hash from snapshot
/// * `mode` - File permissions mode from snapshot
/// * `home` - Home directory for constructing absolute path
/// * `fallback_timestamp` - Timestamp to use if file doesn't exist on disk
/// * `require_file_exists` - If true, returns error when file doesn't exist; if false, uses fallback
///
/// # Returns
///
/// A `FileEntry` with actual file metadata if the file exists on disk, or an error
/// if `require_file_exists` is true and the file is missing. With `require_file_exists=false`,
/// falls back to snapshot metadata.
///
/// # Errors
///
/// Returns an error if `require_file_exists` is true and the file doesn't exist on disk.
fn create_file_entry_with_metadata(
    path: &PathBuf,
    hash: &str,
    mode: u32,
    home: &Path,
    fallback_timestamp: i64,
    require_file_exists: bool,
) -> Result<FileEntry> {
    let abs_path = home.join(path);

    // Try to get actual file metadata if it exists on disk
    match std::fs::metadata(&abs_path) {
        Ok(metadata) => {
            let size = metadata.len();
            let modified = metadata
                .modified()
                .context("Failed to get file modification time")?
                .duration_since(std::time::UNIX_EPOCH)
                .context("Invalid file modification time")?
                .as_secs();

            let modified = i64::try_from(modified).unwrap_or(fallback_timestamp);

            // DON'T create a cached hash here - the hash is from the commit snapshot,
            // not computed from the current file on disk. Creating a cache with the
            // commit hash and current size/mtime would be INVALID and cause status
            // to incorrectly report files as unchanged when they've been modified.
            // Setting cached_hash to None forces status to recompute the hash from disk.

            Ok(FileEntry {
                path: path.clone(),
                hash: hash.to_string(),
                size,
                modified,
                mode,
                cached_hash: None,
            })
        }
        Err(_) if require_file_exists => Err(anyhow::anyhow!(
            "File should exist after reset but not found: {}",
            abs_path.display()
        )),
        Err(_) => {
            // File doesn't exist on disk, use fallback metadata (expected for mixed/keep reset)
            Ok(FileEntry {
                path: path.clone(),
                hash: hash.to_string(),
                size: 0,
                modified: fallback_timestamp,
                mode,
                cached_hash: None,
            })
        }
    }
}

/// Update HEAD to point to a new commit
///
/// This function updates the HEAD reference to point to the specified commit ID,
/// handling both attached HEAD (on a branch) and detached HEAD states. It also
/// maintains the reflog for tracking HEAD movements.
///
/// # Arguments
///
/// * `ctx` - The dotman context containing repository configuration
/// * `commit_id` - The full commit ID (hash) to update HEAD to
///
/// # Returns
///
/// Returns `Ok(())` if HEAD is successfully updated, or an error if:
/// - The current branch cannot be determined
/// - The branch reference cannot be updated
/// - The reflog cannot be written
/// - HEAD file cannot be updated in detached state
///
/// # Behavior
///
/// If HEAD is attached to a branch:
/// - Updates the branch reference to point to the new commit
/// - Records the change in the reflog with the old and new commit IDs
///
/// If HEAD is detached (not on a branch):
/// - Updates HEAD directly to point to the new commit
/// - Records the change in the reflog automatically via `set_head_to_commit`
///
/// The reflog entry includes a "reset" action message with a short commit hash.
fn update_head(ctx: &DotmanContext, commit_id: &str) -> Result<()> {
    use crate::reflog::ReflogManager;
    use crate::refs::RefManager;

    let ref_manager = RefManager::new(ctx.repo_path.clone());
    let reflog_manager = ReflogManager::new(ctx.repo_path.clone());

    if let Some(branch) = ref_manager.current_branch()? {
        let old_value = reflog_manager
            .get_current_head()
            .unwrap_or_else(|_| "0".repeat(40));

        ref_manager.update_branch(&branch, commit_id)?;

        // Log the reflog entry
        reflog_manager.log_head_update(
            &old_value,
            commit_id,
            "reset",
            &format!("reset: moving to {}", &commit_id[..8.min(commit_id.len())]),
        )?;
    } else {
        // Detached HEAD - update HEAD directly with reflog
        ref_manager.set_head_to_commit(
            commit_id,
            Some("reset"),
            Some(&format!(
                "reset: moving to {}",
                &commit_id[..8.min(commit_id.len())]
            )),
        )?;
    }

    Ok(())
}
