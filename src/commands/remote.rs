use crate::DotmanContext;
use crate::config::{RemoteConfig, RemoteType};
use anyhow::Result;
use colored::Colorize;

/// List all configured remotes
pub fn list(ctx: &DotmanContext) -> Result<()> {
    if ctx.config.remotes.is_empty() {
        super::print_info("No remotes configured");
        return Ok(());
    }

    for (name, remote) in &ctx.config.remotes {
        let url = remote.url.as_deref().unwrap_or("<no url>");
        println!(
            "{}\t{} ({})",
            name.yellow(),
            url,
            format!("{:?}", remote.remote_type).dimmed()
        );
    }

    Ok(())
}

/// Add a new remote
pub fn add(ctx: &mut DotmanContext, name: &str, url: &str) -> Result<()> {
    if ctx.config.remotes.contains_key(name) {
        anyhow::bail!("Remote '{}' already exists", name);
    }

    // Determine remote type from URL
    let remote_type = detect_remote_type(url);

    let remote = RemoteConfig {
        remote_type,
        url: Some(url.to_string()),
    };

    ctx.config.set_remote(name.to_string(), remote);
    ctx.config.save(&ctx.config_path)?;

    super::print_success(&format!("Added remote '{}'", name));
    Ok(())
}

/// Remove a remote
pub fn remove(ctx: &mut DotmanContext, name: &str) -> Result<()> {
    if ctx.config.remove_remote(name).is_none() {
        anyhow::bail!("Remote '{}' does not exist", name);
    }

    ctx.config.save(&ctx.config_path)?;
    super::print_success(&format!("Removed remote '{}'", name));
    Ok(())
}

/// Set or update the URL for a remote
pub fn set_url(ctx: &mut DotmanContext, name: &str, url: &str) -> Result<()> {
    let remote = ctx
        .config
        .remotes
        .get_mut(name)
        .ok_or_else(|| anyhow::anyhow!("Remote '{}' does not exist", name))?;

    remote.url = Some(url.to_string());
    remote.remote_type = detect_remote_type(url);

    ctx.config.save(&ctx.config_path)?;
    super::print_success(&format!("Updated URL for remote '{}'", name));
    Ok(())
}

/// Show detailed information about a remote
pub fn show(ctx: &DotmanContext, name: &str) -> Result<()> {
    let remote = ctx
        .config
        .get_remote(name)
        .ok_or_else(|| anyhow::anyhow!("Remote '{}' does not exist", name))?;

    println!("* remote {}", name.yellow());
    println!("  URL: {}", remote.url.as_deref().unwrap_or("<no url>"));
    println!("  Type: {:?}", remote.remote_type);

    // Show branch tracking information if any
    let mut has_tracking = false;
    for (branch_name, tracking) in &ctx.config.branches.tracking {
        if tracking.remote == name {
            if !has_tracking {
                println!("  {} branches configured for push:", "Remote".bold());
                has_tracking = true;
            }
            println!("    {} pushes to {}", branch_name, tracking.branch);
        }
    }

    if !has_tracking {
        println!("  {} branches configured for push", "No".dimmed());
    }

    Ok(())
}

/// Rename a remote
pub fn rename(ctx: &mut DotmanContext, old_name: &str, new_name: &str) -> Result<()> {
    if !ctx.config.remotes.contains_key(old_name) {
        anyhow::bail!("Remote '{}' does not exist", old_name);
    }

    if ctx.config.remotes.contains_key(new_name) {
        anyhow::bail!("Remote '{}' already exists", new_name);
    }

    // Move the remote config
    let remote = ctx
        .config
        .remove_remote(old_name)
        .expect("Remote should exist");
    ctx.config.set_remote(new_name.to_string(), remote);

    // Update branch tracking references
    for tracking in ctx.config.branches.tracking.values_mut() {
        if tracking.remote == old_name {
            tracking.remote = new_name.to_string();
        }
    }

    ctx.config.save(&ctx.config_path)?;
    super::print_success(&format!("Renamed remote '{}' to '{}'", old_name, new_name));
    Ok(())
}

/// Detect remote type from URL
fn detect_remote_type(url: &str) -> RemoteType {
    if url.starts_with("s3://") || url.contains("amazonaws.com") {
        RemoteType::S3
    } else if url.ends_with(".git") || url.contains("github.com") || url.contains("gitlab.com") {
        RemoteType::Git
    } else if url.starts_with("rsync://")
        || (url.contains('@') && url.contains(':') && !url.contains("://"))
    {
        RemoteType::Rsync
    } else {
        RemoteType::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::fs;
    use tempfile::tempdir;

    fn create_test_context() -> Result<(tempfile::TempDir, DotmanContext)> {
        let temp = tempdir()?;
        let repo_path = temp.path().join(".dotman");
        let config_path = temp.path().join("config.toml");

        fs::create_dir_all(&repo_path)?;

        let config = Config::default();
        let ctx = DotmanContext {
            repo_path,
            config_path: config_path.clone(),
            config,
        };

        ctx.config.save(&config_path)?;

        Ok((temp, ctx))
    }

    #[test]
    fn test_list_empty() -> Result<()> {
        let (_temp, ctx) = create_test_context()?;

        // Should not error on empty remotes
        list(&ctx)?;

        Ok(())
    }

    #[test]
    fn test_add_remote() -> Result<()> {
        let (_temp, mut ctx) = create_test_context()?;

        add(&mut ctx, "origin", "https://github.com/user/repo.git")?;

        assert!(ctx.config.remotes.contains_key("origin"));
        let remote = ctx.config.get_remote("origin").unwrap();
        assert_eq!(
            remote.url,
            Some("https://github.com/user/repo.git".to_string())
        );
        assert!(matches!(remote.remote_type, RemoteType::Git));

        Ok(())
    }

    #[test]
    fn test_add_duplicate_remote() -> Result<()> {
        let (_temp, mut ctx) = create_test_context()?;

        add(&mut ctx, "origin", "https://github.com/user/repo.git")?;
        let result = add(&mut ctx, "origin", "https://github.com/other/repo.git");

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));

        Ok(())
    }

    #[test]
    fn test_remove_remote() -> Result<()> {
        let (_temp, mut ctx) = create_test_context()?;

        add(&mut ctx, "origin", "https://github.com/user/repo.git")?;
        remove(&mut ctx, "origin")?;

        assert!(!ctx.config.remotes.contains_key("origin"));

        Ok(())
    }

    #[test]
    fn test_remove_nonexistent_remote() -> Result<()> {
        let (_temp, mut ctx) = create_test_context()?;

        let result = remove(&mut ctx, "nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));

        Ok(())
    }

    #[test]
    fn test_set_url() -> Result<()> {
        let (_temp, mut ctx) = create_test_context()?;

        add(&mut ctx, "origin", "https://github.com/user/repo.git")?;
        set_url(&mut ctx, "origin", "s3://my-bucket")?;

        let remote = ctx.config.get_remote("origin").unwrap();
        assert_eq!(remote.url, Some("s3://my-bucket".to_string()));
        assert!(matches!(remote.remote_type, RemoteType::S3));

        Ok(())
    }

    #[test]
    fn test_rename_remote() -> Result<()> {
        let (_temp, mut ctx) = create_test_context()?;

        add(&mut ctx, "origin", "https://github.com/user/repo.git")?;
        rename(&mut ctx, "origin", "upstream")?;

        assert!(!ctx.config.remotes.contains_key("origin"));
        assert!(ctx.config.remotes.contains_key("upstream"));

        Ok(())
    }

    #[test]
    fn test_detect_remote_type() {
        assert!(matches!(
            detect_remote_type("https://github.com/user/repo.git"),
            RemoteType::Git
        ));
        assert!(matches!(
            detect_remote_type("git@github.com:user/repo.git"),
            RemoteType::Git
        ));
        assert!(matches!(
            detect_remote_type("s3://my-bucket"),
            RemoteType::S3
        ));
        assert!(matches!(
            detect_remote_type("https://s3.amazonaws.com/bucket"),
            RemoteType::S3
        ));
        assert!(matches!(
            detect_remote_type("rsync://server/path"),
            RemoteType::Rsync
        ));
        assert!(matches!(
            detect_remote_type("user@host:/path"),
            RemoteType::Rsync
        ));
        assert!(matches!(
            detect_remote_type("/local/path"),
            RemoteType::None
        ));
    }
}
