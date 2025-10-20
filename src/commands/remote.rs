use crate::DotmanContext;
use crate::config::{RemoteConfig, RemoteType};
use anyhow::{Context, Result};
use colored::Colorize;

/// List all configured remotes
///
/// # Errors
///
/// Returns an error if remotes cannot be accessed
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
///
/// # Errors
///
/// Returns an error if:
/// - A remote with the same name already exists
/// - Failed to save configuration
pub fn add(ctx: &mut DotmanContext, name: &str, url: &str) -> Result<()> {
    if ctx.config.remotes.contains_key(name) {
        return Err(anyhow::anyhow!("Remote '{name}' already exists"));
    }

    // Determine remote type from URL
    let remote_type = detect_remote_type(url);

    let remote = RemoteConfig {
        remote_type,
        url: Some(url.to_string()),
    };

    ctx.config.set_remote(name.to_string(), remote);
    ctx.config.save(&ctx.config_path)?;

    super::print_success(&format!("Added remote '{name}'"));
    Ok(())
}

/// Remove a remote
///
/// # Errors
///
/// Returns an error if:
/// - The remote does not exist
/// - Failed to save configuration
pub fn remove(ctx: &mut DotmanContext, name: &str) -> Result<()> {
    if ctx.config.remove_remote(name).is_none() {
        return Err(anyhow::anyhow!("Remote '{name}' does not exist"));
    }

    ctx.config.save(&ctx.config_path)?;
    super::print_success(&format!("Removed remote '{name}'"));
    Ok(())
}

/// Set or update the URL for a remote
///
/// # Errors
///
/// Returns an error if:
/// - The remote does not exist
/// - Failed to save configuration
pub fn set_url(ctx: &mut DotmanContext, name: &str, url: &str) -> Result<()> {
    let remote = ctx
        .config
        .remotes
        .get_mut(name)
        .with_context(|| format!("Remote '{name}' does not exist"))?;

    remote.url = Some(url.to_string());
    remote.remote_type = detect_remote_type(url);

    ctx.config.save(&ctx.config_path)?;
    super::print_success(&format!("Updated URL for remote '{name}'"));
    Ok(())
}

/// Show detailed information about a remote
///
/// # Errors
///
/// Returns an error if the remote does not exist
pub fn show(ctx: &DotmanContext, name: &str) -> Result<()> {
    let remote = ctx
        .config
        .get_remote(name)
        .with_context(|| format!("Remote '{name}' does not exist"))?;

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
///
/// # Errors
///
/// Returns an error if:
/// - The old remote does not exist
/// - The new name is already in use
/// - Failed to save configuration
pub fn rename(ctx: &mut DotmanContext, old_name: &str, new_name: &str) -> Result<()> {
    if !ctx.config.remotes.contains_key(old_name) {
        return Err(anyhow::anyhow!("Remote '{old_name}' does not exist"));
    }

    if ctx.config.remotes.contains_key(new_name) {
        return Err(anyhow::anyhow!("Remote '{new_name}' already exists"));
    }

    // Move the remote config
    let remote = ctx
        .config
        .remove_remote(old_name)
        .with_context(|| format!("Failed to remove remote '{old_name}' during rename"))?;
    ctx.config.set_remote(new_name.to_string(), remote);

    // Update branch tracking references
    for tracking in ctx.config.branches.tracking.values_mut() {
        if tracking.remote == old_name {
            tracking.remote = new_name.to_string();
        }
    }

    ctx.config.save(&ctx.config_path)?;
    super::print_success(&format!("Renamed remote '{old_name}' to '{new_name}'"));
    Ok(())
}

/// Detect remote type from URL
#[allow(clippy::case_sensitive_file_extension_comparisons)]
fn detect_remote_type(url: &str) -> RemoteType {
    if url.ends_with(".git")
        || url.contains("github.com")
        || url.contains("gitlab.com")
        || url.contains("bitbucket.org")
        || url.contains("git@")
        || url.starts_with("git://")
        || url.starts_with("https://") && url.contains(".git")
        || url.starts_with("http://") && url.contains(".git")
    {
        RemoteType::Git
    } else {
        RemoteType::None
    }
}
