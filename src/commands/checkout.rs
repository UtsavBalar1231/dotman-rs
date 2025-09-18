use crate::DotmanContext;
use crate::refs::RefManager;
use crate::refs::resolver::RefResolver;
use crate::storage::snapshots::SnapshotManager;
use anyhow::{Context, Result};
use colored::Colorize;

/// Execute checkout to switch to a different commit or branch
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
    super::print_info(&format!("Checking out commit {}", display_target.yellow()));

    // Get home directory as target
    let home = dirs::home_dir().context("Could not find home directory")?;

    // Get list of currently tracked files for cleanup
    let current_files = crate::commands::status::get_current_files(ctx)?;

    // Restore files with cleanup of files not in target
    snapshot_manager.restore_snapshot(&commit_id, &home, Some(&current_files))?;

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

    super::print_success(&format!(
        "Checked out commit {} ({} files restored)",
        display_id.yellow(),
        snapshot.files.len()
    ));

    println!("  {}: {}", "Author".bold(), snapshot.commit.author);
    println!("  {}: {}", "Message".bold(), snapshot.commit.message);

    Ok(())
}

/// Check if working directory is clean
///
/// # Errors
///
/// Returns an error if failed to check file status
fn check_working_directory_clean(ctx: &DotmanContext) -> Result<bool> {
    use crate::INDEX_FILE;
    use crate::commands::status::get_current_files;
    use crate::storage::index::Index;

    let index_path = ctx.repo_path.join(INDEX_FILE);
    let index = Index::load(&index_path)?;

    let current_files = get_current_files(ctx)?;
    let statuses = index.get_status_parallel(&current_files);

    Ok(statuses.is_empty())
}
