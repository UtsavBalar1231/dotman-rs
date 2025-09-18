use crate::DotmanContext;
use crate::config::BranchTracking;
use crate::refs::RefManager;
use crate::storage::snapshots::SnapshotManager;
use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::HashSet;

/// List all branches
/// List all branches
///
/// # Errors
///
/// Returns an error if:
/// - Repository is not initialized
/// - Failed to read branch information
pub fn list(ctx: &DotmanContext) -> Result<()> {
    ctx.check_repo_initialized()?;

    let ref_manager = RefManager::new(ctx.repo_path.clone());
    let branches = ref_manager.list_branches()?;
    let current = ref_manager.current_branch()?;

    if branches.is_empty() {
        super::print_info("No branches exist");
        return Ok(());
    }

    for branch in branches {
        let is_current = current.as_ref().is_some_and(|c| c == &branch);
        let prefix = if is_current { "* " } else { "  " };

        let tracking_info =
            ctx.config
                .branches
                .tracking
                .get(&branch)
                .map_or_else(String::new, |tracking| {
                    format!(" -> {}/{}", tracking.remote, tracking.branch)
                        .dimmed()
                        .to_string()
                });

        if is_current {
            println!(
                "{}{branch}{tracking_info}",
                prefix.green(),
                branch = branch.green()
            );
        } else {
            println!("{prefix}{branch}{tracking_info}");
        }
    }

    Ok(())
}

/// Create a new branch
/// Create a new branch
///
/// # Errors
///
/// Returns an error if:
/// - Repository is not initialized
/// - Branch already exists
/// - Failed to create branch
pub fn create(ctx: &DotmanContext, name: &str, start_point: Option<&str>) -> Result<()> {
    ctx.check_repo_initialized()?;

    let ref_manager = RefManager::new(ctx.repo_path.clone());

    if ref_manager.branch_exists(name) {
        return Err(anyhow::anyhow!("Branch '{name}' already exists"));
    }

    // If start_point is provided, resolve it to a commit
    let commit_str;
    let commit_id = if let Some(point) = start_point {
        // This could be a branch name or commit ID
        if ref_manager.branch_exists(point) {
            commit_str = ref_manager.get_branch_commit(point)?;
            Some(commit_str.as_str())
        } else {
            // Assume it's a commit ID
            Some(point)
        }
    } else {
        None
    };

    ref_manager.create_branch(name, commit_id)?;
    super::print_success(&format!("Created branch '{name}'"));

    Ok(())
}

/// Check if a branch is fully merged into another branch
///
/// A branch is considered fully merged if all its commits are reachable from the target branch.
/// This is done by following the parent chain from both branches and checking if the branch's
/// tip commit appears in the target's history.
///
/// # Errors
///
/// Returns an error if:
/// - Failed to get branch commits
/// - Failed to load snapshots
fn is_branch_fully_merged(
    ctx: &DotmanContext,
    branch_name: &str,
    target_branch: &str,
) -> Result<bool> {
    let ref_manager = RefManager::new(ctx.repo_path.clone());
    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    // Get the commit at the tip of the branch to check
    let branch_commit = ref_manager.get_branch_commit(branch_name)?;

    // Handle empty branches (no commits)
    if branch_commit == "0".repeat(40) {
        // An empty branch is considered "merged" since it has no unique commits
        return Ok(true);
    }

    // If the branch points to the same commit as target, it's merged
    let target_commit = ref_manager.get_branch_commit(target_branch)?;
    if branch_commit == target_commit {
        return Ok(true);
    }

    // Handle empty target branch
    if target_commit == "0".repeat(40) {
        // If target has no commits, the branch cannot be merged into it
        return Ok(false);
    }

    // Build the set of all commits reachable from the target branch
    let mut reachable_commits = HashSet::new();
    let mut current = Some(target_commit);
    let mut visited = HashSet::new();

    while let Some(commit_id) = current {
        // Prevent infinite loops in case of cycles (shouldn't happen but safety first)
        if visited.contains(&commit_id) {
            break;
        }
        visited.insert(commit_id.clone());
        reachable_commits.insert(commit_id.clone());

        // Load the snapshot to get the parent
        match snapshot_manager.load_snapshot(&commit_id) {
            Ok(snapshot) => {
                current = snapshot.commit.parent;
            }
            Err(_) => {
                // If we can't load a snapshot, it means we've reached a broken chain
                // or the initial commit. Either way, we've traversed what we can.
                break;
            }
        }
    }

    // Check if the branch's tip commit is in the reachable set
    Ok(reachable_commits.contains(&branch_commit))
}

/// Get the default branch to check merge status against
///
/// Returns "main" if it exists, "master" if it exists, or the current branch.
/// If no branches exist, returns None.
fn get_default_merge_target(ref_manager: &RefManager) -> Result<Option<String>> {
    // First try to get current branch
    let current = ref_manager.current_branch()?;

    // Check if main exists
    if ref_manager.branch_exists("main") {
        return Ok(Some("main".to_string()));
    }

    // Check if master exists
    if ref_manager.branch_exists("master") {
        return Ok(Some("master".to_string()));
    }

    // Return current branch if it exists
    Ok(current)
}

/// Delete a branch
/// Delete a branch
///
/// # Errors
///
/// Returns an error if:
/// - Repository is not initialized
/// - Branch does not exist
/// - Trying to delete the current branch
/// - Branch is not fully merged (unless force is used)
/// - Failed to delete branch
pub fn delete(ctx: &DotmanContext, name: &str, force: bool) -> Result<()> {
    ctx.check_repo_initialized()?;

    let ref_manager = RefManager::new(ctx.repo_path.clone());

    // Check if branch exists
    if !ref_manager.branch_exists(name) {
        return Err(anyhow::anyhow!("Branch '{}' does not exist", name));
    }

    // Check if it's the current branch
    if ref_manager
        .current_branch()?
        .as_ref()
        .is_some_and(|c| c == name)
    {
        return Err(anyhow::anyhow!(
            "Cannot delete the currently checked out branch '{}'",
            name
        ));
    }

    // Don't allow deletion of main/master branches without force
    if !force && (name == "main" || name == "master") {
        return Err(anyhow::anyhow!(
            "Cannot delete the '{}' branch without --force\n\
             This is typically the default branch and should not be deleted",
            name
        ));
    }

    // Check if branch is fully merged unless force is true
    if !force {
        // Determine which branch to check against
        let merge_target = get_default_merge_target(&ref_manager)?
            .ok_or_else(|| anyhow::anyhow!("No branches available to check merge status"))?;

        // Don't check against itself
        if merge_target != name {
            let is_merged = is_branch_fully_merged(ctx, name, &merge_target)?;

            if !is_merged {
                super::print_warning(&format!(
                    "Branch '{name}' is not fully merged into '{merge_target}'"
                ));
                super::print_info("If you are sure you want to delete it, use --force");
                return Err(anyhow::anyhow!(
                    "Branch '{}' is not fully merged. Use --force to delete anyway",
                    name
                ));
            }
        }
    }

    // Perform the deletion
    ref_manager.delete_branch(name)?;
    super::print_success(&format!("Deleted branch '{name}'"));

    Ok(())
}

/// Switch to a branch
///
/// # Errors
///
/// Returns an error if:
/// - Repository is not initialized
/// - Branch does not exist
/// - Failed to switch branch
/// - Working directory has uncommitted changes (unless forced)
pub fn checkout(ctx: &DotmanContext, name: &str, force: bool) -> Result<()> {
    ctx.check_repo_initialized()?;

    // Check if branch exists first for better error message
    let ref_manager = RefManager::new(ctx.repo_path.clone());
    if !ref_manager.branch_exists(name) {
        return Err(anyhow::anyhow!("Branch '{name}' does not exist"));
    }

    // Check if the branch points to a valid commit
    let commit_id = ref_manager.get_branch_commit(name)?;

    // Check for the special "no commits" marker (40 zeros)
    if commit_id == "0".repeat(40) {
        // This is an empty branch with no commits
        // Just update HEAD to point to this branch without trying to restore files
        ref_manager.set_head_to_branch(name, None, None)?;
        super::print_success(&format!("Switched to empty branch '{name}'"));
        super::print_info("No commits on this branch yet");
        return Ok(());
    }

    // Delegate to the checkout command which handles branch resolution,
    // working directory updates, and proper reflog entries
    crate::commands::checkout::execute(ctx, name, force)?;

    Ok(())
}

/// Rename a branch
///
/// # Errors
///
/// Returns an error if:
/// - Repository is not initialized
/// - Not on any branch (when renaming current)
/// - Failed to rename branch
pub fn rename(ctx: &DotmanContext, old_name: Option<&str>, new_name: &str) -> Result<()> {
    ctx.check_repo_initialized()?;

    let ref_manager = RefManager::new(ctx.repo_path.clone());

    let old = if let Some(name) = old_name {
        name.to_string()
    } else {
        // Rename current branch
        ref_manager
            .current_branch()?
            .context("Not on any branch (detached HEAD)")?
    };

    ref_manager.rename_branch(&old, new_name)?;
    super::print_success(&format!("Renamed branch '{old}' to '{new_name}'"));

    Ok(())
}

/// Set upstream tracking for a branch
///
/// # Errors
///
/// Returns an error if:
/// - The repository is not initialized
/// - The branch does not exist
/// - Failed to update branch configuration
pub fn set_upstream(
    ctx: &mut DotmanContext,
    branch: Option<&str>,
    remote: &str,
    remote_branch: Option<&str>,
) -> Result<()> {
    ctx.check_repo_initialized()?;

    let ref_manager = RefManager::new(ctx.repo_path.clone());

    // Determine which branch to set upstream for
    let branch_name = if let Some(b) = branch {
        b.to_string()
    } else {
        ref_manager
            .current_branch()?
            .context("Not on any branch (detached HEAD)")?
    };

    if !ref_manager.branch_exists(&branch_name) {
        return Err(anyhow::anyhow!("Branch '{}' does not exist", branch_name));
    }

    if !ctx.config.remotes.contains_key(remote) {
        return Err(anyhow::anyhow!("Remote '{}' does not exist", remote));
    }

    // Use same branch name on remote if not specified
    let remote_branch_name = remote_branch.unwrap_or(&branch_name);

    // Set tracking information
    let tracking = BranchTracking {
        remote: remote.to_string(),
        branch: remote_branch_name.to_string(),
    };

    ctx.config
        .branches
        .tracking
        .insert(branch_name.clone(), tracking);
    ctx.config.save(&ctx.config_path)?;

    super::print_success(&format!(
        "Branch '{branch_name}' set up to track '{remote}/{remote_branch_name}'"
    ));

    Ok(())
}

/// Remove upstream tracking for a branch
///
/// # Errors
///
/// Returns an error if:
/// - The repository is not initialized
/// - The branch does not exist
/// - Failed to update branch configuration
pub fn unset_upstream(ctx: &mut DotmanContext, branch: Option<&str>) -> Result<()> {
    ctx.check_repo_initialized()?;

    let ref_manager = RefManager::new(ctx.repo_path.clone());

    let branch_name = if let Some(b) = branch {
        b.to_string()
    } else {
        ref_manager
            .current_branch()?
            .context("Not on any branch (detached HEAD)")?
    };

    if ctx.config.branches.tracking.remove(&branch_name).is_none() {
        super::print_info(&format!("Branch '{branch_name}' has no upstream tracking"));
    } else {
        ctx.config.save(&ctx.config_path)?;
        super::print_success(&format!(
            "Removed upstream tracking for branch '{branch_name}'"
        ));
    }

    Ok(())
}
