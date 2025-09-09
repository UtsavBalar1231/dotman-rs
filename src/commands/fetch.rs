use crate::DotmanContext;
use crate::mapping::MappingManager;
use crate::mirror::GitMirror;
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
            "Remote '{}' has no type configured or is not a Git remote.",
            remote
        )),
    }
}

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

    super::print_info(&format!("Fetching from git remote {remote} ({url})"));

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

    // Update mapping to track fetched commits
    let _mapping_manager = MappingManager::new(&ctx.repo_path)?;

    // List remote branches to update tracking
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
            super::print_info(&format!("Found {} remote branches", remote_branches.len()));
            for branch in remote_branches.iter().take(5) {
                println!("  {}", branch.green());
            }
            if remote_branches.len() > 5 {
                println!("  ... and {} more", remote_branches.len() - 5);
            }
        }
    }

    super::print_success(&format!("Successfully fetched from {remote} ({url})"));

    // Suggest next steps
    if branch.is_none() && !all {
        super::print_info("Tip: Use 'dot merge origin/branch' to merge fetched changes");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, RemoteType};
    use std::fs;
    use tempfile::tempdir;

    fn create_test_context(
        remote_type: crate::config::RemoteType,
        url: Option<String>,
    ) -> Result<DotmanContext> {
        let temp = tempdir()?;
        let repo_path = temp.path().join(".dotman");
        let config_path = temp.path().join("config.toml");

        fs::create_dir_all(&repo_path)?;
        fs::create_dir_all(repo_path.join("commits"))?;
        fs::create_dir_all(repo_path.join("objects"))?;
        fs::write(repo_path.join("HEAD"), "ref: refs/heads/main")?;

        let mut config = Config::default();
        config.core.repo_path = repo_path.clone();

        let remote_config = crate::config::RemoteConfig { remote_type, url };
        config.remotes.insert("origin".to_string(), remote_config);
        config.save(&config_path)?;

        Ok(DotmanContext {
            repo_path,
            config_path,
            config,
            no_pager: true,
        })
    }

    #[test]
    fn test_execute_no_remote() -> Result<()> {
        let mut ctx = create_test_context(RemoteType::None, None)?;

        // The test tries to execute with a remote that doesn't exist
        ctx.config.remotes.clear();

        let result = execute(&ctx, "nonexistent", None, false, false);
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_execute_git_remote() -> Result<()> {
        let _ctx = create_test_context(
            RemoteType::Git,
            Some("https://github.com/user/repo.git".to_string()),
        )?;

        // Would need to mock git command for full test
        Ok(())
    }
}
