use crate::DotmanContext;
use crate::mirror::GitMirror;
use crate::output;
use anyhow::{Context, Result};
use colored::Colorize;
use std::process::Command;

/// Execute fetch command - download objects and refs from remote repository
///
/// # Errors
///
/// Returns an error if:
/// - The repository is not initialized
/// - The specified remote does not exist
/// - The remote has no URL configured
/// - Network operations fail
/// - The fetch operation fails
pub fn execute(
    ctx: &DotmanContext,
    remote: &str,
    branch: Option<&str>,
    all: bool,
    tags: bool,
) -> Result<()> {
    ctx.check_repo_initialized()?;

    let remote_config = ctx.config.get_remote(remote).with_context(|| {
        format!("Remote '{remote}' does not exist. Use 'dot remote add' to add it.")
    })?;

    match &remote_config.remote_type {
        crate::config::RemoteType::Git => {
            fetch_from_git(ctx, remote_config, remote, branch, all, tags)
        }
        crate::config::RemoteType::None => Err(anyhow::anyhow!(
            "Remote '{remote}' has no type configured or is not a Git remote."
        )),
    }
}

/// Performs the actual git fetch operation from a remote repository
///
/// This function handles the core fetch workflow:
/// - Initializes or updates the git mirror repository
/// - Executes git fetch with appropriate arguments (branch, --all, --tags)
/// - Updates remote tracking branches
/// - Displays fetch progress and results
///
/// The function creates a mirror repository in `~/.dotman/mirrors/<remote>/` which acts
/// as a bare git repository tracking the remote. If the mirror doesn't exist, it's created
/// and initialized. If it exists, the fetch operation updates the mirror's state.
///
/// # Arguments
///
/// * `ctx` - The dotman context containing repository path and configuration
/// * `remote_config` - Configuration for the remote, including URL and type
/// * `remote` - Name of the remote to fetch from (e.g., "origin")
/// * `branch` - Optional specific branch to fetch. If None, behavior depends on `all` flag
/// * `all` - If true and no branch specified, fetches all branches from the remote
/// * `tags` - If true, fetches tags in addition to branches
///
/// # Errors
///
/// Returns an error if:
/// - The remote URL is not configured in `remote_config`
/// - Mirror initialization fails (e.g., filesystem errors, git not found)
/// - The git fetch command fails (network issues, authentication, invalid refs)
/// - Unable to list remote branches after fetch
fn fetch_from_git(
    ctx: &DotmanContext,
    remote_config: &crate::config::RemoteConfig,
    remote: &str,
    branch: Option<&str>,
    all: bool,
    tags: bool,
) -> Result<()> {
    let url = remote_config
        .url
        .as_ref()
        .with_context(|| format!("Remote '{remote}' has no URL configured"))?;

    output::info(&format!("Fetching from git remote {remote} ({url})"));

    // Create and initialize mirror
    let mirror = GitMirror::new(&ctx.repo_path, remote, url, ctx.config.clone());
    mirror.init_mirror()?;

    // Run git fetch in the mirror repository
    let mirror_path = mirror.get_mirror_path();

    let mut args = vec!["fetch", "origin"];

    // Add branch if specified
    let branch_str;
    if let Some(b) = branch {
        branch_str = b.to_string();
        args.push(&branch_str);
    } else if all {
        args.push("--all");
    }

    if tags {
        args.push("--tags");
    }

    args.push("--verbose");

    let output = Command::new("git")
        .args(&args)
        .current_dir(mirror_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Git fetch failed: {stderr}"));
    }

    let _stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Git fetch outputs to stderr for progress
    if !stderr.is_empty() {
        for line in stderr.lines() {
            if line.contains("->") || line.contains("new") || line.contains("tag") {
                println!("  {line}");
            }
        }
    }

    // Update remote tracking refs (refs/remotes/origin/*)
    // Get commit IDs for all remote tracking branches
    let output = Command::new("git")
        .args([
            "for-each-ref",
            &format!("refs/remotes/{remote}"),
            "--format=%(objectname) %(refname)",
        ])
        .current_dir(mirror_path)
        .output()?;

    let ref_manager = crate::refs::RefManager::new(ctx.repo_path.clone());

    if output.status.success() {
        let refs = String::from_utf8_lossy(&output.stdout);
        let mut updated_count = 0;

        for line in refs.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() == 2 {
                let git_commit = parts[0];
                let ref_name = parts[1];

                // Extract branch name from refs/remotes/remote/branch
                if let Some(branch_name) = ref_name.strip_prefix(&format!("refs/remotes/{remote}/"))
                {
                    // Try to get dotman commit from mapping
                    let mapping_manager = crate::mapping::MappingManager::new(&ctx.repo_path)?;
                    if let Some(dotman_commit) = mapping_manager
                        .mapping()
                        .get_dotman_commit(remote, git_commit)
                    {
                        // Update remote ref to point to dotman commit
                        ref_manager.update_remote_ref(remote, branch_name, &dotman_commit)?;
                    } else {
                        // No mapping yet - this is a branch that hasn't been pulled/pushed
                        // We can still track the git commit hash for reference
                        // Store git commit hash temporarily (will be replaced when pulled)
                        ref_manager.update_remote_ref(remote, branch_name, git_commit)?;
                    }
                    updated_count += 1;
                }
            }
        }

        if updated_count > 0 {
            output::info(&format!("Updated {updated_count} remote tracking refs"));
        }
    }

    // List remote branches
    let output = Command::new("git")
        .args(["branch", "-r"])
        .current_dir(mirror_path)
        .output()?;

    if output.status.success() {
        let branches = String::from_utf8_lossy(&output.stdout);
        let remote_branches: Vec<&str> = branches
            .lines()
            .map(str::trim)
            .filter(|l| l.starts_with("origin/"))
            .collect();

        if !remote_branches.is_empty() {
            output::info(&format!("Found {} remote branches", remote_branches.len()));
            for branch in remote_branches.iter().take(5) {
                println!("  {}", branch.green());
            }
            if remote_branches.len() > 5 {
                println!("  ... and {} more", remote_branches.len() - 5);
            }
        }
    }

    output::success(&format!("Successfully fetched from {remote} ({url})"));

    // Suggest next steps
    if branch.is_none() && !all {
        output::info("Tip: Use 'dot merge origin/branch' to merge fetched changes");
    }

    Ok(())
}
