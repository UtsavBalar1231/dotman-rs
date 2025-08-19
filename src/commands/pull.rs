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
