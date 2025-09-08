use crate::DotmanContext;
use crate::config::BranchTracking;
use crate::refs::RefManager;
use anyhow::{Context, Result};
use colored::Colorize;

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

/// Delete a branch
/// Delete a branch
///
/// # Errors
///
/// Returns an error if:
/// - Repository is not initialized
/// - Failed to delete branch
pub fn delete(ctx: &DotmanContext, name: &str, force: bool) -> Result<()> {
    ctx.check_repo_initialized()?;

    let ref_manager = RefManager::new(ctx.repo_path.clone());

    // TODO: Check if branch is fully merged unless force is true
    if !force {
        super::print_info("Warning: deleting branch without checking if it's fully merged");
    }

    ref_manager.delete_branch(name)?;
    super::print_success(&format!("Deleted branch '{name}'"));

    Ok(())
}

/// Switch to a branch
/// Switch to a branch
///
/// # Errors
///
/// Returns an error if:
/// - Repository is not initialized
/// - Branch does not exist
/// - Failed to switch branch
pub fn checkout(ctx: &DotmanContext, name: &str) -> Result<()> {
    ctx.check_repo_initialized()?;

    let ref_manager = RefManager::new(ctx.repo_path.clone());

    if !ref_manager.branch_exists(name) {
        return Err(anyhow::anyhow!("Branch '{name}' does not exist"));
    }

    ref_manager.set_head_to_branch(name)?;
    super::print_success(&format!("Switched to branch '{name}'"));

    // TODO: Update working directory to match branch state

    Ok(())
}

/// Rename a branch
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::fixtures::create_test_context;

    // Use the create_test_context from fixtures
    // No local definition needed

    #[test]
    fn test_list_branches() -> Result<()> {
        let (_temp, ctx) = create_test_context()?;

        // Should list the default main branch
        list(&ctx)?;

        Ok(())
    }

    #[test]
    fn test_create_branch() -> Result<()> {
        let (_temp, ctx) = create_test_context()?;

        create(&ctx, "feature", None)?;

        let ref_manager = RefManager::new(ctx.repo_path);
        assert!(ref_manager.branch_exists("feature"));

        Ok(())
    }

    #[test]
    fn test_create_duplicate_branch() -> Result<()> {
        let (_temp, ctx) = create_test_context()?;

        create(&ctx, "feature", None)?;
        let result = create(&ctx, "feature", None);

        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_delete_branch() -> Result<()> {
        let (_temp, ctx) = create_test_context()?;

        create(&ctx, "temp", None)?;
        delete(&ctx, "temp", false)?;

        let ref_manager = RefManager::new(ctx.repo_path);
        assert!(!ref_manager.branch_exists("temp"));

        Ok(())
    }

    #[test]
    fn test_checkout_branch() -> Result<()> {
        let (_temp, ctx) = create_test_context()?;

        create(&ctx, "feature", None)?;
        checkout(&ctx, "feature")?;

        let ref_manager = RefManager::new(ctx.repo_path);
        assert_eq!(ref_manager.current_branch()?, Some("feature".to_string()));

        Ok(())
    }

    #[test]
    fn test_rename_branch() -> Result<()> {
        let (_temp, ctx) = create_test_context()?;

        create(&ctx, "old", None)?;
        rename(&ctx, Some("old"), "new")?;

        let ref_manager = RefManager::new(ctx.repo_path);
        assert!(!ref_manager.branch_exists("old"));
        assert!(ref_manager.branch_exists("new"));

        Ok(())
    }

    #[test]
    fn test_set_upstream() -> Result<()> {
        let (_temp, mut ctx) = create_test_context()?;

        crate::commands::remote::add(&mut ctx, "origin", "https://github.com/user/repo.git")?;

        // Set upstream for main branch
        set_upstream(&mut ctx, Some("main"), "origin", None)?;

        assert!(ctx.config.branches.tracking.contains_key("main"));
        let tracking = ctx.config.branches.tracking.get("main").unwrap();
        assert_eq!(tracking.remote, "origin");
        assert_eq!(tracking.branch, "main");

        Ok(())
    }
}
