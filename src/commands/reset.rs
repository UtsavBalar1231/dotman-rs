use crate::refs::resolver::RefResolver;
use crate::storage::index::Index;
use crate::storage::snapshots::SnapshotManager;
use crate::{DotmanContext, INDEX_FILE};
use anyhow::{Context, Result};
use colored::Colorize;
use std::path::PathBuf;

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
#[allow(clippy::fn_params_excessive_bools)]
#[allow(clippy::too_many_lines)]
pub fn execute(
    ctx: &DotmanContext,
    commit: &str,
    hard: bool,
    soft: bool,
    mixed: bool,
    keep: bool,
    paths: &[String],
) -> Result<()> {
    ctx.check_repo_initialized()?;

    // Count how many modes are specified
    let mode_count = [hard, soft, mixed, keep].iter().filter(|&&x| x).count();
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

    let snapshot = snapshot_manager
        .load_snapshot(&commit_id)
        .with_context(|| format!("Failed to load commit: {commit_id}"))?;

    if hard {
        // Hard reset: update index and working directory
        super::print_info(&format!(
            "Hard reset to commit {}",
            commit_id[..8.min(commit_id.len())].yellow()
        ));

        // Restore files to working directory
        let home = dirs::home_dir().context("Could not find home directory")?;
        snapshot_manager.restore_snapshot(&commit_id, &home, None)?;

        // Update index to match commit
        let mut index = Index::new();
        for (path, file) in &snapshot.files {
            index.add_entry(crate::storage::FileEntry {
                path: path.clone(),
                hash: file.hash.clone(),
                size: 0, // Will be updated on next status
                modified: snapshot.commit.timestamp,
                mode: file.mode,
                cached_hash: None,
            });
        }

        let index_path = ctx.repo_path.join(INDEX_FILE);
        index.save(&index_path)?;

        super::print_success(&format!(
            "Hard reset complete. Working directory and index updated to match commit {}",
            commit_id[..8.min(commit_id.len())].yellow()
        ));
    } else if soft {
        // Soft reset: only move HEAD, keep index and working directory
        super::print_info(&format!(
            "Soft reset to commit {}",
            commit_id[..8.min(commit_id.len())].yellow()
        ));

        super::print_success(&format!(
            "Soft reset complete. HEAD now points to commit {}",
            commit_id[..8.min(commit_id.len())].yellow()
        ));
    } else if keep {
        // Keep reset: reset HEAD and index but keep working directory changes
        super::print_info(&format!(
            "Keep reset to commit {}",
            commit_id[..8.min(commit_id.len())].yellow()
        ));

        // Only update index if files differ from target commit
        let current_index = Index::load(&ctx.repo_path.join(INDEX_FILE))?;
        let mut new_index = Index::new();

        for (path, file) in &snapshot.files {
            // Keep local changes if they exist
            if let Some(current_entry) = current_index.get_entry(path)
                && current_entry.hash != file.hash
            {
                // File has local changes, keep them
                continue;
            }
            new_index.add_entry(crate::storage::FileEntry {
                path: path.clone(),
                hash: file.hash.clone(),
                size: 0,
                modified: snapshot.commit.timestamp,
                mode: file.mode,
                cached_hash: None,
            });
        }

        let index_path = ctx.repo_path.join(INDEX_FILE);
        new_index.save(&index_path)?;

        super::print_success(&format!(
            "Keep reset complete. Local changes preserved, HEAD now points to {}",
            commit_id[..8.min(commit_id.len())].yellow()
        ));
    } else {
        // Mixed reset (default or explicit): update index but not working directory
        super::print_info(&format!(
            "Mixed reset to commit {}",
            commit_id[..8.min(commit_id.len())].yellow()
        ));

        // Update index to match commit
        let mut index = Index::new();
        for (path, file) in &snapshot.files {
            index.add_entry(crate::storage::FileEntry {
                path: path.clone(),
                hash: file.hash.clone(),
                size: 0,
                modified: snapshot.commit.timestamp,
                mode: file.mode,
                cached_hash: None,
            });
        }

        let index_path = ctx.repo_path.join(INDEX_FILE);
        index.save(&index_path)?;

        super::print_success(&format!(
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
    super::print_info(&format!(
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
            index.add_entry(crate::storage::FileEntry {
                path: index_path.clone(),
                hash: file.hash.clone(),
                size: 0, // Will be updated
                modified: snapshot.commit.timestamp,
                mode: file.mode,
                cached_hash: None,
            });

            println!("  {} {}", "reset:".green(), index_path.display());
            reset_count += 1;
        } else {
            // File doesn't exist in target commit - remove from index (unstage)
            if index.remove_entry(&index_path).is_some() {
                println!("  {} {}", "unstaged:".yellow(), index_path.display());
                reset_count += 1;
            } else {
                super::print_warning(&format!("File not in index: {}", path.display()));
                not_found_count += 1;
            }
        }
    }

    // Save updated index
    if reset_count > 0 {
        index.save(&index_path)?;
        super::print_success(&format!("Reset {reset_count} file(s)"));
    }

    if not_found_count > 0 {
        super::print_info(&format!("{not_found_count} file(s) were not in the index"));
    }

    Ok(())
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
