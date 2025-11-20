//! Checkout operations for switching between commits and branches.
//!
//! This module provides functionality for checking out different commits or branches,
//! similar to `git checkout`. It handles:
//!
//! - Branch switching with reflog tracking
//! - Commit checkout (detached HEAD state)
//! - Working directory validation (uncommitted changes detection)
//! - Snapshot restoration with file cleanup
//! - Reference resolution (HEAD, branches, commit IDs, ancestry)
//!
//! # Safety
//!
//! By default, checkout will fail if there are uncommitted changes. Use `--force`
//! to override this safety check.
//!
//! # Examples
//!
//! ```no_run
//! use dotman::DotmanContext;
//! use dotman::commands::checkout;
//!
//! # fn main() -> anyhow::Result<()> {
//! let ctx = DotmanContext::new()?;
//!
//! // Checkout a branch
//! checkout::execute(&ctx, "main", false)?;
//!
//! // Checkout a specific commit (detached HEAD)
//! checkout::execute(&ctx, "abc123", false)?;
//!
//! // Checkout with uncommitted changes (force)
//! checkout::execute(&ctx, "main", true)?;
//! # Ok(())
//! # }
//! ```

use crate::DotmanContext;
use crate::output;
use crate::refs::RefManager;
use crate::refs::resolver::RefResolver;
use crate::storage::snapshots::SnapshotManager;
use anyhow::{Context, Result};
use colored::Colorize;

/// Switch to a different commit or branch
///
/// # Errors
///
/// Returns an error if:
/// - Repository is not initialized
/// - Working directory has uncommitted changes (unless forced)
/// - Failed to resolve the target reference
/// - Failed to load or restore the snapshot
pub fn execute(ctx: &DotmanContext, target: &str, force: bool) -> Result<()> {
    ctx.check_repo_initialized()?;

    if !force {
        let status_output = check_working_directory_clean(ctx)?;
        if !status_output {
            return Err(anyhow::anyhow!(
                "You have uncommitted changes. Use --force to override or commit your changes first."
            ));
        }
    }

    // Use the reference resolver to handle HEAD, HEAD~n, branches, and short hashes
    let resolver = RefResolver::new(ctx.repo_path.clone());
    let commit_id = resolver
        .resolve(target)
        .with_context(|| format!("Failed to resolve reference: {target}"))?;

    // Check if we're checking out the NULL commit (no commits yet)
    if commit_id == crate::NULL_COMMIT_ID {
        // Update HEAD to point to the branch
        let ref_manager = RefManager::new(ctx.repo_path.clone());
        let message = format!("checkout: moving to {target}");

        if ref_manager.branch_exists(target) {
            ref_manager.set_head_to_branch(target, Some("checkout"), Some(&message))?;
            output::success(&format!("Switched to branch '{target}'"));
        } else {
            return Err(anyhow::anyhow!(
                "Cannot checkout '{target}' - no commits exist yet"
            ));
        }
        return Ok(());
    }

    let snapshot_manager = SnapshotManager::with_permissions(
        ctx.repo_path.clone(),
        ctx.config.core.compression_level,
        ctx.config.tracking.preserve_permissions,
    );

    let snapshot = snapshot_manager
        .load_snapshot(&commit_id)
        .with_context(|| format!("Failed to load commit: {commit_id}"))?;

    let display_target = if commit_id.len() >= 8 {
        &commit_id[..8]
    } else {
        &commit_id
    };
    output::info(&format!("Checking out commit {}", display_target.yellow()));

    // Get home directory as target
    let home = dirs::home_dir().context("Could not find home directory")?;

    // Get list of currently tracked files for cleanup by loading HEAD snapshot
    let ref_manager = RefManager::new(ctx.repo_path.clone());
    let current_files = if let Some(head_commit) = ref_manager.get_head_commit()? {
        if head_commit == crate::NULL_COMMIT_ID {
            Vec::new()
        } else {
            // Load current HEAD snapshot to get tracked files
            let head_snapshot = snapshot_manager
                .load_snapshot(&head_commit)
                .with_context(|| format!("Failed to load HEAD commit: {head_commit}"))?;

            // Convert snapshot paths to absolute paths
            head_snapshot
                .files
                .keys()
                .map(|path| {
                    if path.is_relative() {
                        home.join(path)
                    } else {
                        path.clone()
                    }
                })
                .collect()
        }
    } else {
        Vec::new()
    };

    // Restore files with cleanup of files not in target
    snapshot_manager.restore_snapshot(&commit_id, &home, Some(&current_files))?;

    // Clear the index - after checkout, there are no staged changes
    // Committed files are stored in the snapshot, not in the index
    let index_path = ctx.repo_path.join(crate::INDEX_FILE);
    let index = crate::storage::index::Index::new();
    index
        .save(&index_path)
        .with_context(|| "Failed to clear index after checkout")?;

    // Update HEAD with reflog entry
    let ref_manager = RefManager::new(ctx.repo_path.clone());
    let message = format!("checkout: moving to {target}");

    // Check if target is a branch name
    if ref_manager.branch_exists(target) {
        // Checkout the branch (update HEAD to point to the branch)
        ref_manager.set_head_to_branch(target, Some("checkout"), Some(&message))?;
    } else {
        // Checkout a specific commit (detached HEAD)
        ref_manager.set_head_to_commit(&commit_id, Some("checkout"), Some(&message))?;
    }

    let display_id = if commit_id.len() >= 8 {
        &commit_id[..8]
    } else {
        &commit_id
    };

    output::success(&format!(
        "Checked out commit {} ({} files restored)",
        display_id.yellow(),
        snapshot.files.len()
    ));

    println!("  {}: {}", "Author".bold(), snapshot.commit.author);
    println!("  {}: {}", "Message".bold(), snapshot.commit.message);

    Ok(())
}

/// Returns true if no modifications or staged changes exist
///
/// # Errors
///
/// Returns an error if failed to check file status
fn check_working_directory_clean(ctx: &DotmanContext) -> Result<bool> {
    use crate::INDEX_FILE;
    use crate::storage::index::Index;

    let index_path = ctx.repo_path.join(INDEX_FILE);
    let index = Index::load(&index_path)?;

    // Check for staged changes
    if index.has_staged_changes() {
        return Ok(false);
    }

    // Check for unstaged modifications by comparing with HEAD snapshot
    let ref_manager = RefManager::new(ctx.repo_path.clone());
    let head_commit = match ref_manager.get_head_commit()? {
        Some(commit) if commit != crate::NULL_COMMIT_ID => commit,
        _ => return Ok(true), // No commits yet, so working directory is clean
    };

    // Load HEAD snapshot
    let snapshot_manager = SnapshotManager::with_permissions(
        ctx.repo_path.clone(),
        ctx.config.core.compression_level,
        ctx.config.tracking.preserve_permissions,
    );

    let snapshot = snapshot_manager
        .load_snapshot(&head_commit)
        .with_context(|| format!("Failed to load HEAD commit: {head_commit}"))?;

    // Get home directory
    let home = dirs::home_dir().context("Could not find home directory")?;

    // Check if any files in the snapshot have been modified or deleted
    for (path, file) in &snapshot.files {
        let abs_path = home.join(path);

        // Check if file exists
        if !abs_path.exists() {
            return Ok(false); // File was deleted
        }

        // Check if file has been modified by comparing hash
        // Use the cached hash if available to avoid re-hashing
        let (current_hash, _) = crate::storage::file_ops::hash_file(&abs_path, None)?;
        if current_hash != file.hash {
            return Ok(false); // File was modified
        }
    }

    // Check for new untracked files that might conflict
    // (This is a basic check - a more thorough check would scan for all untracked files)
    // For now, if we've made it this far, consider it clean
    Ok(true)
}
