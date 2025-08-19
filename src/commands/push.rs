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
