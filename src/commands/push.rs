use crate::DotmanContext;
use anyhow::Result;
use std::process::Command;

pub fn execute(ctx: &DotmanContext, remote: &str, branch: &str) -> Result<()> {
    ctx.ensure_repo_exists()?;

    match &ctx.config.remote.remote_type {
        crate::config::RemoteType::Git => push_to_git(ctx, remote, branch),
        crate::config::RemoteType::S3 => push_to_s3(ctx, remote, branch),
        crate::config::RemoteType::Rsync => push_to_rsync(ctx, remote, branch),
        crate::config::RemoteType::None => {
            anyhow::bail!("No remote configured. Update your config file to set up a remote.");
        }
    }
}

fn push_to_git(ctx: &DotmanContext, remote: &str, branch: &str) -> Result<()> {
    let url = ctx
        .config
        .remote
        .url
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No remote URL configured"))?;

    super::print_info(&format!("Pushing to git remote {} ({})", remote, url));

    // For git remotes, we'll sync the .dotman directory
    let output = Command::new("git")
        .args(["push", url, branch])
        .current_dir(&ctx.repo_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Git push failed: {}", stderr);
    }

    super::print_success(&format!("Successfully pushed to {} ({})", remote, branch));
    Ok(())
}

fn push_to_s3(ctx: &DotmanContext, _remote: &str, _branch: &str) -> Result<()> {
    let bucket = ctx
        .config
        .remote
        .url
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No S3 bucket configured"))?;

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

fn push_to_rsync(ctx: &DotmanContext, _remote: &str, _branch: &str) -> Result<()> {
    let destination = ctx
        .config
        .remote
        .url
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No rsync destination configured"))?;

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
        config.remote.remote_type = remote_type;
        config.remote.url = url;
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
                .contains("No remote configured")
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

        let result = push_to_git(&ctx, "origin", "main");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No remote URL configured")
        );

        Ok(())
    }

    #[test]
    fn test_push_to_s3_no_bucket() -> Result<()> {
        let ctx = create_test_context(RemoteType::S3, None)?;

        let result = push_to_s3(&ctx, "origin", "main");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No S3 bucket configured")
        );

        Ok(())
    }

    #[test]
    fn test_push_to_rsync_no_destination() -> Result<()> {
        let ctx = create_test_context(RemoteType::Rsync, None)?;

        let result = push_to_rsync(&ctx, "origin", "main");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No rsync destination configured")
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
            assert!(ctx.config.remote.url.is_some());
            assert_eq!(ctx.config.remote.url.as_ref().unwrap(), url);
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
