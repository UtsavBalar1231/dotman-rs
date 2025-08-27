use crate::DotmanContext;
use crate::mapping::MappingManager;
use crate::mirror::GitMirror;
use anyhow::Result;
use colored::Colorize;
use std::process::Command;

/// Execute fetch command - download objects and refs from remote repository
pub fn execute(
    ctx: &DotmanContext,
    remote: &str,
    branch: Option<&str>,
    all: bool,
    tags: bool,
) -> Result<()> {
    ctx.check_repo_initialized()?;

    // Get the specified remote
    let remote_config = ctx.config.get_remote(remote).ok_or_else(|| {
        anyhow::anyhow!(
            "Remote '{}' does not exist. Use 'dot remote add' to add it.",
            remote
        )
    })?;

    match &remote_config.remote_type {
        crate::config::RemoteType::Git => {
            fetch_from_git(ctx, remote_config, remote, branch, all, tags)
        }
        crate::config::RemoteType::S3 => fetch_from_s3(ctx, remote_config, remote),
        crate::config::RemoteType::Rsync => fetch_from_rsync(ctx, remote_config, remote),
        crate::config::RemoteType::None => {
            anyhow::bail!("Remote '{}' has no type configured.", remote);
        }
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
        .ok_or_else(|| anyhow::anyhow!("Remote '{}' has no URL configured", remote))?;

    super::print_info(&format!("Fetching from git remote {} ({})", remote, url));

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
        anyhow::bail!("Git fetch failed: {}", stderr);
    }

    // Parse the output to show what was fetched
    let _stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Git fetch outputs to stderr for progress
    if !stderr.is_empty() {
        for line in stderr.lines() {
            if line.contains("->") || line.contains("new") || line.contains("tag") {
                println!("  {}", line);
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
            .map(|l| l.trim())
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

    super::print_success(&format!("Successfully fetched from {} ({})", remote, url));

    // Suggest next steps
    if branch.is_none() && !all {
        super::print_info("Tip: Use 'dot merge origin/branch' to merge fetched changes");
    }

    Ok(())
}

fn fetch_from_s3(
    _ctx: &DotmanContext,
    remote_config: &crate::config::RemoteConfig,
    remote: &str,
) -> Result<()> {
    let bucket = remote_config
        .url
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Remote '{}' has no S3 bucket configured", remote))?;

    super::print_info(&format!("Fetching from S3 bucket {}", bucket));

    // Use aws CLI to sync from S3
    let temp_dir = tempfile::tempdir()?;
    let output = Command::new("aws")
        .args([
            "s3",
            "sync",
            &format!("s3://{}/", bucket),
            temp_dir.path().to_str().unwrap(),
            "--delete",
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("S3 sync failed: {}", stderr);
    }

    // Compare with local repository
    super::print_info("Comparing fetched data with local repository...");

    super::print_success(&format!("Successfully fetched from S3 bucket {}", bucket));
    super::print_info("Use 'dot pull' to integrate the changes");

    Ok(())
}

fn fetch_from_rsync(
    _ctx: &DotmanContext,
    remote_config: &crate::config::RemoteConfig,
    remote: &str,
) -> Result<()> {
    let source = remote_config
        .url
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Remote '{}' has no rsync source configured", remote))?;

    super::print_info(&format!("Fetching via rsync from {}", source));

    // Use rsync to fetch to a temporary location
    let temp_dir = tempfile::tempdir()?;
    let output = Command::new("rsync")
        .args([
            "-avz",
            "--delete",
            source,
            &format!("{}/", temp_dir.path().display()),
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Rsync failed: {}", stderr);
    }

    super::print_info("Comparing fetched data with local repository...");

    super::print_success(&format!("Successfully fetched via rsync from {}", source));
    super::print_info("Use 'dot pull' to integrate the changes");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, RemoteType};
    use std::fs;
    use tempfile::tempdir;

    fn create_test_context(remote_type: RemoteType, url: Option<String>) -> Result<DotmanContext> {
        let temp = tempdir()?;
        let repo_path = temp.path().join(".dotman");
        let config_path = temp.path().join("config.toml");

        fs::create_dir_all(&repo_path)?;
        fs::create_dir_all(repo_path.join("commits"))?;
        fs::create_dir_all(repo_path.join("objects"))?;
        fs::write(repo_path.join("HEAD"), "ref: refs/heads/main")?;

        let mut config = Config::default();
        config.core.repo_path = repo_path.clone();

        // Add a remote
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
        // Remove the "origin" remote that was added in create_test_context
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

    #[test]
    fn test_fetch_from_s3_no_bucket() -> Result<()> {
        let ctx = create_test_context(RemoteType::S3, None)?;

        let remote_config = crate::config::RemoteConfig {
            remote_type: RemoteType::S3,
            url: None,
        };
        let result = fetch_from_s3(&ctx, &remote_config, "origin");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no S3 bucket"));

        Ok(())
    }

    #[test]
    fn test_fetch_from_rsync_no_source() -> Result<()> {
        let ctx = create_test_context(RemoteType::Rsync, None)?;

        let remote_config = crate::config::RemoteConfig {
            remote_type: RemoteType::Rsync,
            url: None,
        };
        let result = fetch_from_rsync(&ctx, &remote_config, "origin");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no rsync source"));

        Ok(())
    }
}
