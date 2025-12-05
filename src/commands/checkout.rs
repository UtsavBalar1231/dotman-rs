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
//! checkout::execute(&ctx, "main", false, false)?;
//!
//! // Checkout a specific commit (detached HEAD)
//! checkout::execute(&ctx, "abc123", false, false)?;
//!
//! // Checkout with uncommitted changes (force)
//! checkout::execute(&ctx, "main", true, false)?;
//! # Ok(())
//! # }
//! ```

use crate::DotmanContext;
use crate::NULL_COMMIT_ID;
use crate::output;
use crate::refs::RefManager;
use crate::refs::resolver::RefResolver;
use crate::storage::snapshots::SnapshotManager;
use anyhow::{Context, Result};
use colored::Colorize;
use std::io::IsTerminal;

/// Switch to a different commit or branch
///
/// # Arguments
///
/// * `ctx` - The dotman context
/// * `target` - Branch name, commit ID, or reference (e.g., `HEAD~1`)
/// * `force` - If `true`, proceed even with uncommitted changes
/// * `dry_run` - If `true`, show what would happen without making changes
///
/// # Errors
///
/// Returns an error if:
/// - Repository is not initialized
/// - Working directory has uncommitted changes (unless forced)
/// - Failed to resolve the target reference
/// - Failed to load or restore the snapshot
pub fn execute(ctx: &DotmanContext, target: &str, force: bool, dry_run: bool) -> Result<()> {
    ctx.check_repo_initialized()?;

    if !force && !dry_run {
        let status_output = check_working_directory_clean(ctx)?;
        if !status_output {
            return Err(anyhow::anyhow!(
                "You have uncommitted changes. Use --force to override or commit your changes first."
            ));
        }
    }

    let commit_id = resolve_target_ref(target, &ctx.repo_path)?;

    if commit_id == NULL_COMMIT_ID {
        if dry_run {
            println!("\n{}", "Dry run - would checkout:".yellow().bold());
            println!("  {} Target: {}", "→".dimmed(), target);
            println!("  {} No commits exist yet", "→".dimmed());
            println!("\n{}", "Run without --dry-run to execute".dimmed());
            return Ok(());
        }
        return handle_null_commit(target, &ctx.repo_path);
    }

    let snapshot_manager = create_snapshot_manager(ctx);
    let snapshot = snapshot_manager
        .load_snapshot(&commit_id)
        .with_context(|| format!("Failed to load commit: {commit_id}"))?;

    let home = dirs::home_dir().context("Could not find home directory")?;
    let current_files = get_current_tracked_files(&snapshot_manager, &ctx.repo_path, &home)?;

    if dry_run {
        preview_checkout(ctx, target, &commit_id, &snapshot, &home, &current_files);
        return Ok(());
    }

    display_checkout_info(&commit_id);

    if !force {
        prompt_for_untracked_conflicts(ctx, &snapshot, &home, &current_files)?;
    }

    restore_and_clear_index(ctx, &snapshot_manager, &commit_id, &home, &current_files)?;
    update_head_after_checkout(target, &commit_id, &ctx.repo_path)?;
    display_checkout_success(&commit_id, &snapshot);

    Ok(())
}

/// Resolve a target reference to a commit ID
fn resolve_target_ref(target: &str, repo_path: &std::path::Path) -> Result<String> {
    let resolver = RefResolver::new(repo_path.to_path_buf());
    resolver.resolve(target).with_context(|| {
        if target.contains('/') || std::path::Path::new(target).exists() {
            format!(
                "Failed to resolve reference: {target}\n\
                    Hint: To restore files, use: dot restore {target}"
            )
        } else {
            format!("Failed to resolve reference: {target}")
        }
    })
}

/// Handle checkout of NULL commit (no commits exist yet)
fn handle_null_commit(target: &str, repo_path: &std::path::Path) -> Result<()> {
    let ref_manager = RefManager::new(repo_path.to_path_buf());
    let message = format!("checkout: moving to {target}");

    if ref_manager.branch_exists(target) {
        ref_manager.set_head_to_branch(target, Some("checkout"), Some(&message))?;
        output::success(&format!("Switched to branch '{target}'"));
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Cannot checkout '{target}' - no commits exist yet"
        ))
    }
}

/// Create a snapshot manager from context
fn create_snapshot_manager(ctx: &DotmanContext) -> SnapshotManager {
    SnapshotManager::with_permissions(
        ctx.repo_path.clone(),
        ctx.config.core.compression_level,
        ctx.config.tracking.preserve_permissions,
    )
}

/// Display checkout progress info
fn display_checkout_info(commit_id: &str) {
    let display_target = if commit_id.len() >= 8 {
        &commit_id[..8]
    } else {
        commit_id
    };
    output::info(&format!("Checking out commit {}", display_target.yellow()));
}

/// Get list of currently tracked files for cleanup
fn get_current_tracked_files(
    snapshot_manager: &SnapshotManager,
    repo_path: &std::path::Path,
    home: &std::path::Path,
) -> Result<Vec<std::path::PathBuf>> {
    let ref_manager = RefManager::new(repo_path.to_path_buf());

    if let Some(head_commit) = ref_manager.get_head_commit()? {
        if head_commit == NULL_COMMIT_ID {
            return Ok(Vec::new());
        }

        let head_snapshot = snapshot_manager
            .load_snapshot(&head_commit)
            .with_context(|| format!("Failed to load HEAD commit: {head_commit}"))?;

        Ok(head_snapshot
            .files
            .keys()
            .map(|path| {
                if path.is_relative() {
                    home.join(path)
                } else {
                    path.clone()
                }
            })
            .collect())
    } else {
        Ok(Vec::new())
    }
}

/// Prompt user for confirmation if untracked files would be overwritten
fn prompt_for_untracked_conflicts(
    ctx: &DotmanContext,
    snapshot: &crate::storage::snapshots::Snapshot,
    home: &std::path::Path,
    current_files: &[std::path::PathBuf],
) -> Result<()> {
    let conflicts = detect_untracked_conflicts(snapshot, home, current_files);
    if conflicts.is_empty() {
        return Ok(());
    }

    eprintln!(
        "\n{}: The following untracked files would be overwritten by checkout:",
        "Warning".yellow().bold()
    );
    for file in &conflicts {
        if let Ok(rel_path) = file.strip_prefix(home) {
            eprintln!("  {}", rel_path.display());
        } else {
            eprintln!("  {}", file.display());
        }
    }

    // Check if we're in a non-interactive environment
    let is_non_interactive = ctx.non_interactive
        || std::env::var("DOTMAN_NON_INTERACTIVE").is_ok()
        || !std::io::stdin().is_terminal();

    if is_non_interactive {
        // In non-interactive mode, fail with a clear error message
        return Err(anyhow::anyhow!(
            "Checkout would overwrite untracked files. Use --force to proceed anyway."
        ));
    }

    eprint!("\n{}? (Y/n): ", "Continue".bold());
    std::io::Write::flush(&mut std::io::stderr())?;

    let mut response = String::new();
    std::io::stdin().read_line(&mut response)?;
    let response = response.trim().to_lowercase();

    if !response.is_empty() && response != "y" && response != "yes" {
        return Err(anyhow::anyhow!("Checkout cancelled by user"));
    }

    Ok(())
}

/// Restore snapshot and clear the index
fn restore_and_clear_index(
    ctx: &DotmanContext,
    snapshot_manager: &SnapshotManager,
    commit_id: &str,
    home: &std::path::Path,
    current_files: &[std::path::PathBuf],
) -> Result<()> {
    snapshot_manager.restore_snapshot(commit_id, home, Some(current_files))?;

    let index_path = ctx.repo_path.join(crate::INDEX_FILE);
    let index = crate::storage::index::Index::new();
    index
        .save(&index_path)
        .with_context(|| "Failed to clear index after checkout")
}

/// Update HEAD reference after checkout
fn update_head_after_checkout(
    target: &str,
    commit_id: &str,
    repo_path: &std::path::Path,
) -> Result<()> {
    let ref_manager = RefManager::new(repo_path.to_path_buf());
    let message = format!("checkout: moving to {target}");

    if ref_manager.branch_exists(target) {
        ref_manager.set_head_to_branch(target, Some("checkout"), Some(&message))
    } else {
        ref_manager.set_head_to_commit(commit_id, Some("checkout"), Some(&message))
    }
}

/// Display success message after checkout
fn display_checkout_success(commit_id: &str, snapshot: &crate::storage::snapshots::Snapshot) {
    let display_id = if commit_id.len() >= 8 {
        &commit_id[..8]
    } else {
        commit_id
    };

    output::success(&format!(
        "Checked out commit {} ({} files restored)",
        display_id.yellow(),
        snapshot.files.len()
    ));

    println!("  {}: {}", "Author".bold(), snapshot.commit.author);
    println!("  {}: {}", "Message".bold(), snapshot.commit.message);
}

/// Preview what would happen during checkout
fn preview_checkout(
    ctx: &DotmanContext,
    target: &str,
    commit_id: &str,
    snapshot: &crate::storage::snapshots::Snapshot,
    home: &std::path::Path,
    current_files: &[std::path::PathBuf],
) {
    let ref_manager = RefManager::new(ctx.repo_path.clone());
    let is_branch = ref_manager.branch_exists(target);

    println!("\n{}", "Dry run - would checkout:".yellow().bold());
    println!(
        "  {} Target: {}",
        "→".dimmed(),
        if is_branch {
            format!("branch '{target}'")
        } else {
            format!("commit {}", &commit_id[..8.min(commit_id.len())])
        }
    );

    let mut files_to_restore = 0;
    let mut files_to_delete = 0;

    // Files that would be restored from snapshot
    for path in snapshot.files.keys() {
        let abs_path = home.join(path);
        if abs_path.exists() || current_files.contains(&abs_path) {
            files_to_restore += 1;
        }
    }

    // Files that would be deleted (in current but not in target)
    for current_file in current_files {
        let rel_path = current_file.strip_prefix(home).unwrap_or(current_file);
        if !snapshot.files.contains_key(&rel_path.to_path_buf()) {
            files_to_delete += 1;
        }
    }

    // Check for untracked file conflicts
    let conflicts = detect_untracked_conflicts(snapshot, home, current_files);
    let untracked_conflicts = conflicts.len();

    println!(
        "  {} {} file(s) would be restored",
        "→".dimmed(),
        files_to_restore
    );
    if files_to_delete > 0 {
        println!(
            "  {} {} file(s) would be deleted",
            "→".dimmed(),
            files_to_delete.to_string().red()
        );
    }
    if untracked_conflicts > 0 {
        println!(
            "  {} {} {} untracked file(s) would be overwritten",
            "⚠".yellow(),
            "WARNING:".yellow().bold(),
            untracked_conflicts.to_string().red()
        );
    }
    println!("  {} Index would be cleared", "→".dimmed());

    println!("\n{}", "Run without --dry-run to execute".dimmed());
}

/// Returns true if no modifications or staged changes exist
///
/// # Errors
///
/// Returns an error if failed to check file status
fn check_working_directory_clean(ctx: &DotmanContext) -> Result<bool> {
    use crate::INDEX_FILE;
    use crate::storage::index::Index;

    const PROGRESS_THRESHOLD: usize = 10;

    let index_path = ctx.repo_path.join(INDEX_FILE);
    let index = Index::load(&index_path)?;

    // Check for staged changes
    if index.has_staged_changes() {
        return Ok(false);
    }

    // Check for unstaged modifications by comparing with HEAD snapshot
    let ref_manager = RefManager::new(ctx.repo_path.clone());
    let head_commit = match ref_manager.get_head_commit()? {
        Some(commit) if commit != NULL_COMMIT_ID => commit,
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

    // Show progress for larger file sets (hashing is I/O-bound)
    let file_count = snapshot.files.len();
    let mut progress = (file_count > PROGRESS_THRESHOLD)
        .then(|| output::start_progress("Checking working directory", file_count));

    // Check all files, tracking whether directory is clean
    let mut is_clean = true;
    for (i, (path, file)) in snapshot.files.iter().enumerate() {
        let abs_path = home.join(path);

        if !abs_path.exists() {
            is_clean = false;
            break;
        }

        // Handle hash errors gracefully - file may have been deleted between exists() and hash_file()
        if let Ok((current_hash, _)) = crate::storage::file_ops::hash_file(&abs_path, None) {
            if current_hash != file.hash {
                is_clean = false;
                break;
            }
        } else {
            // File became inaccessible (race condition or permissions) - treat as unclean
            is_clean = false;
            break;
        }

        if let Some(ref mut p) = progress {
            p.update(i + 1);
        }
    }

    if let Some(p) = progress {
        p.finish();
    }

    Ok(is_clean)
}

/// Detects untracked files that would be overwritten by checkout
///
/// Returns a vector of paths to untracked files that conflict with files in the target snapshot.
/// A file is considered a conflict if:
/// - It exists on disk
/// - It's not in the current HEAD's tracked files
/// - The target snapshot wants to write to that path
fn detect_untracked_conflicts(
    snapshot: &crate::storage::snapshots::Snapshot,
    home: &std::path::Path,
    current_files: &[std::path::PathBuf],
) -> Vec<std::path::PathBuf> {
    use std::collections::HashSet;

    let current_files_set: HashSet<_> = current_files.iter().collect();
    let mut conflicts = Vec::new();

    for path in snapshot.files.keys() {
        let abs_path = if path.is_relative() {
            home.join(path)
        } else {
            path.clone()
        };

        // If file exists and is not currently tracked, it's an untracked file that would be overwritten
        if abs_path.exists() && !current_files_set.contains(&abs_path) {
            conflicts.push(abs_path);
        }
    }

    conflicts
}
