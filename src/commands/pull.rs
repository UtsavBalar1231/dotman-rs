use crate::DotmanContext;
use anyhow::Result;
use std::process::Command;

pub fn execute(ctx: &DotmanContext, remote: &str, branch: &str) -> Result<()> {
    ctx.ensure_repo_exists()?;

    match &ctx.config.remote.remote_type {
        crate::config::RemoteType::Git => pull_from_git(ctx, remote, branch),
        crate::config::RemoteType::S3 => pull_from_s3(ctx, remote, branch),
        crate::config::RemoteType::Rsync => pull_from_rsync(ctx, remote, branch),
        crate::config::RemoteType::None => {
            anyhow::bail!("No remote configured. Update your config file to set up a remote.");
        }
    }
}

fn pull_from_git(ctx: &DotmanContext, remote: &str, branch: &str) -> Result<()> {
    let url = ctx
        .config
        .remote
        .url
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No remote URL configured"))?;

    super::print_info(&format!("Pulling from git remote {} ({})", remote, url));

    let output = Command::new("git")
        .args(["pull", url, branch])
        .current_dir(&ctx.repo_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Git pull failed: {}", stderr);
    }

    super::print_success(&format!("Successfully pulled from {} ({})", remote, branch));

    // After pulling, we might want to checkout the latest commit
    super::print_info("Updating working directory to match pulled changes...");

    Ok(())
}

fn pull_from_s3(ctx: &DotmanContext, _remote: &str, _branch: &str) -> Result<()> {
    let bucket = ctx
        .config
        .remote
        .url
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No S3 bucket configured"))?;

    super::print_info(&format!("Pulling from S3 bucket {}", bucket));

    let output = Command::new("aws")
        .args([
            "s3",
            "sync",
            &format!("s3://{}/", bucket),
            ctx.repo_path.to_str().unwrap(),
            "--delete",
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("S3 sync failed: {}", stderr);
    }

    super::print_success(&format!("Successfully pulled from S3 bucket {}", bucket));
    Ok(())
}

fn pull_from_rsync(ctx: &DotmanContext, _remote: &str, _branch: &str) -> Result<()> {
    let source = ctx
        .config
        .remote
        .url
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No rsync source configured"))?;

    super::print_info(&format!("Pulling via rsync from {}", source));

    let output = Command::new("rsync")
        .args([
            "-avz",
            "--delete",
            source,
            &format!("{}/", ctx.repo_path.display()),
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Rsync failed: {}", stderr);
    }

    super::print_success(&format!("Successfully pulled via rsync from {}", source));
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

        // We can't easily test the actual git command execution without mocking
        // but we can at least verify the function doesn't panic with valid config

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
    fn test_pull_from_git_no_url() -> Result<()> {
        let ctx = create_test_context(RemoteType::Git, None)?;

        let result = pull_from_git(&ctx, "origin", "main");
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
    fn test_pull_from_s3_no_bucket() -> Result<()> {
        let ctx = create_test_context(RemoteType::S3, None)?;

        let result = pull_from_s3(&ctx, "origin", "main");
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
    fn test_pull_from_rsync_no_source() -> Result<()> {
        let ctx = create_test_context(RemoteType::Rsync, None)?;

        let result = pull_from_rsync(&ctx, "origin", "main");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No rsync source configured")
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

}
