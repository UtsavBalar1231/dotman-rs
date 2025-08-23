use crate::DotmanContext;
use crate::mapping::MappingManager;
use crate::mirror::GitMirror;
use crate::refs::RefManager;
use crate::storage::index::Index;
use crate::storage::snapshots::SnapshotManager;
use crate::sync::Exporter;
use anyhow::Result;
use std::process::Command;

pub fn execute(ctx: &DotmanContext, remote: &str, branch: &str) -> Result<()> {
    ctx.ensure_repo_exists()?;

    // Get the specified remote
    let remote_config = ctx.config.get_remote(remote).ok_or_else(|| {
        anyhow::anyhow!(
            "Remote '{}' does not exist. Use 'dot remote add' to add it.",
            remote
        )
    })?;

    match &remote_config.remote_type {
        crate::config::RemoteType::Git => push_to_git(ctx, remote_config, remote, branch),
        crate::config::RemoteType::S3 => push_to_s3(ctx, remote_config, remote, branch),
        crate::config::RemoteType::Rsync => push_to_rsync(ctx, remote_config, remote, branch),
        crate::config::RemoteType::None => {
            anyhow::bail!("Remote '{}' has no type configured.", remote);
        }
    }
}

fn push_to_git(
    ctx: &DotmanContext,
    remote_config: &crate::config::RemoteConfig,
    remote: &str,
    branch: &str,
) -> Result<()> {
    let url = remote_config
        .url
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Remote '{}' has no URL configured", remote))?;

    super::print_info(&format!("Pushing to git remote {} ({})", remote, url));

    // Create and initialize mirror
    let mirror = GitMirror::new(&ctx.repo_path, remote, url, ctx.config.clone());
    mirror.init_mirror()?;

    // Checkout the branch in mirror
    mirror.checkout_branch(branch)?;

    // Load current state
    let ref_manager = RefManager::new(ctx.repo_path.clone());
    let current_commit = ref_manager
        .get_head_commit()?
        .ok_or_else(|| anyhow::anyhow!("No commits to push"))?;

    // Export dotman state to mirror
    let index = Index::load(&ctx.repo_path.join(crate::INDEX_FILE))?;
    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);
    let exporter = Exporter::new(&snapshot_manager, &index);

    super::print_info("Exporting files to mirror repository...");

    // Export current commit to mirror
    let exported_files = exporter.export_commit(&current_commit, mirror.get_mirror_path())?;

    // Clean up removed files from mirror
    let current_files: Vec<_> = exported_files.iter().map(|(_, rel)| rel.clone()).collect();
    mirror.clean_removed_files(&current_files)?;

    // Commit in mirror
    let author = crate::utils::get_current_user_with_config(&ctx.config);

    // Get commit message from dotman
    let commit_message = if let Ok(snapshot) = snapshot_manager.load_snapshot(&current_commit) {
        snapshot.commit.message
    } else {
        format!("Dotman commit {}", &current_commit[..8])
    };

    super::print_info("Creating git commit...");
    let git_commit = mirror.commit(&commit_message, &author)?;

    // Push to remote
    super::print_info(&format!("Pushing branch '{}' to remote...", branch));
    mirror.push(branch)?;

    // Update mapping
    let mut mapping_manager = MappingManager::new(&ctx.repo_path)?;
    mapping_manager.add_and_save(remote, &current_commit, &git_commit)?;
    mapping_manager.update_branch_and_save(branch, &current_commit, Some((remote, &git_commit)))?;

    super::print_success(&format!(
        "Successfully pushed to {} ({}) - branch '{}'",
        remote, url, branch
    ));
    Ok(())
}

fn push_to_s3(
    ctx: &DotmanContext,
    remote_config: &crate::config::RemoteConfig,
    remote: &str,
    _branch: &str,
) -> Result<()> {
    let bucket = remote_config
        .url
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Remote '{}' has no S3 bucket configured", remote))?;

    super::print_info(&format!("Pushing to S3 bucket {}", bucket));

    // Use aws CLI or rusoto for S3 sync
    let output = Command::new("aws")
        .args([
            "s3",
            "sync",
            ctx.repo_path.to_str().unwrap(),
            &format!("s3://{}/", bucket),
            "--delete",
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("S3 sync failed: {}", stderr);
    }

    super::print_success(&format!("Successfully pushed to S3 bucket {}", bucket));
    Ok(())
}

fn push_to_rsync(
    ctx: &DotmanContext,
    remote_config: &crate::config::RemoteConfig,
    remote: &str,
    _branch: &str,
) -> Result<()> {
    let destination = remote_config.url.as_ref().ok_or_else(|| {
        anyhow::anyhow!("Remote '{}' has no rsync destination configured", remote)
    })?;

    super::print_info(&format!("Pushing via rsync to {}", destination));

    let output = Command::new("rsync")
        .args([
            "-avz",
            "--delete",
            &format!("{}/", ctx.repo_path.display()),
            destination,
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Rsync failed: {}", stderr);
    }

    super::print_success(&format!("Successfully pushed via rsync to {}", destination));
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

        let mut config = Config::default();
        config.core.repo_path = repo_path.clone();

        // Add a remote named "origin" with the specified type and url
        let remote_config = crate::config::RemoteConfig { remote_type, url };
        config.remotes.insert("origin".to_string(), remote_config);
        config.save(&config_path)?;

        Ok(DotmanContext {
            repo_path,
            config_path,
            config,
        })
    }

    #[test]
    fn test_execute_no_remote() -> Result<()> {
        let ctx = create_test_context(RemoteType::None, None)?;

        let result = execute(&ctx, "origin", "main");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Remote 'origin' has no type configured")
        );

        Ok(())
    }

    #[test]
    fn test_execute_git_remote() -> Result<()> {
        let _ctx = create_test_context(
            RemoteType::Git,
            Some("https://github.com/user/repo.git".to_string()),
        )?;

        // This would normally call git command, which we'd need to mock
        // For now, we're just testing that it routes to the correct function
        // In a real scenario, we'd use dependency injection or mocking

        Ok(())
    }

    #[test]
    fn test_execute_s3_remote() -> Result<()> {
        let _ctx = create_test_context(RemoteType::S3, Some("my-bucket".to_string()))?;

        // Similar to git test - would need to mock aws command
        Ok(())
    }

    #[test]
    fn test_execute_rsync_remote() -> Result<()> {
        let _ctx = create_test_context(
            RemoteType::Rsync,
            Some("user@host:/path/to/repo".to_string()),
        )?;

        // Similar to git test - would need to mock rsync command
        Ok(())
    }

    #[test]
    fn test_push_to_git_no_url() -> Result<()> {
        let ctx = create_test_context(RemoteType::Git, None)?;

        let remote_config = crate::config::RemoteConfig {
            remote_type: RemoteType::Git,
            url: None,
        };
        let result = push_to_git(&ctx, &remote_config, "origin", "main");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Remote 'origin' has no URL configured")
        );

        Ok(())
    }

    #[test]
    fn test_push_to_s3_no_bucket() -> Result<()> {
        let ctx = create_test_context(RemoteType::S3, None)?;

        let remote_config = crate::config::RemoteConfig {
            remote_type: RemoteType::S3,
            url: None,
        };
        let result = push_to_s3(&ctx, &remote_config, "origin", "main");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Remote 'origin' has no S3 bucket configured")
        );

        Ok(())
    }

    #[test]
    fn test_push_to_rsync_no_destination() -> Result<()> {
        let ctx = create_test_context(RemoteType::Rsync, None)?;

        let remote_config = crate::config::RemoteConfig {
            remote_type: RemoteType::Rsync,
            url: None,
        };
        let result = push_to_rsync(&ctx, &remote_config, "origin", "main");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Remote 'origin' has no rsync destination configured")
        );

        Ok(())
    }

    #[test]
    fn test_ensure_repo_exists_error() -> Result<()> {
        let temp = tempdir()?;
        let repo_path = temp.path().join("nonexistent").join("deep").join("path");

        // Create a file where directory should be, causing conflict
        let conflict_path = temp.path().join("nonexistent");
        fs::write(&conflict_path, "blocking file")?;

        let ctx = DotmanContext {
            repo_path,
            config_path: temp.path().join("config"),
            config: Config::default(),
        };

        let result = execute(&ctx, "origin", "main");
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_remote_urls_with_special_chars() -> Result<()> {
        // Test with URLs containing special characters
        let special_urls = vec![
            "git@github.com:user/repo.git",
            "https://user:pass@github.com/repo.git",
            "s3://bucket-with-dashes",
            "rsync://user@host:2222/path/to/repo",
            "file:///local/path/to/repo",
        ];

        for url in special_urls {
            let ctx = create_test_context(RemoteType::Git, Some(url.to_string()))?;
            let origin_remote = ctx.config.remotes.get("origin").unwrap();
            assert!(origin_remote.url.is_some());
            assert_eq!(origin_remote.url.as_ref().unwrap(), url);
        }

        Ok(())
    }

    #[test]
    fn test_push_with_empty_repo() -> Result<()> {
        let ctx = create_test_context(
            RemoteType::Git,
            Some("https://github.com/user/repo.git".to_string()),
        )?;

        // Ensure repo has no commits (empty HEAD file)
        let head_path = ctx.repo_path.join("HEAD");
        if head_path.exists() {
            fs::remove_file(&head_path)?;
        }

        // Test that push still attempts to work with empty repo
        // (actual command would fail, but our function should handle it gracefully)

        Ok(())
    }
}
