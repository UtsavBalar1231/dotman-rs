use crate::DotmanContext;
use crate::output;
use anyhow::Result;
use colored::Colorize;

/// Execute config command to get/set configuration values
///
/// # Errors
///
/// Returns an error if:
/// - Failed to set or unset configuration value
/// - Failed to save configuration
pub fn execute(
    ctx: &mut DotmanContext,
    key: Option<&str>,
    value: Option<String>,
    unset: bool,
    list: bool,
) -> Result<()> {
    // If --list flag is set or no key is provided, show all configuration
    if list || key.is_none() {
        show_all_config(ctx);
        return Ok(());
    }

    let key =
        key.ok_or_else(|| anyhow::anyhow!("Key must be provided when not using --list flag"))?;

    if unset {
        // Unset a configuration value
        ctx.config.unset(key)?;
        ctx.config.save(&ctx.config_path)?;
        output::success(&format!("Unset {key}"));
    } else if let Some(val) = value {
        // Set a configuration value
        ctx.config.set(key, val.clone())?;
        ctx.config.save(&ctx.config_path)?;
        output::success(&format!("Set {key} = {val}"));
    } else if let Some(val) = ctx.config.get(key) {
        println!("{val}");
    } else {
        output::warning(&format!("Configuration key '{key}' is not set"));
    }

    Ok(())
}

/// Show all configuration values
fn show_all_config(ctx: &DotmanContext) {
    println!("{}", "[user]".bold());
    if let Some(name) = &ctx.config.user.name {
        println!("  name = {name}");
    }
    if let Some(email) = &ctx.config.user.email {
        println!("  email = {email}");
    }

    println!("\n{}", "[core]".bold());
    println!("  repo_path = {}", ctx.config.core.repo_path.display());
    println!("  compression = {:?}", ctx.config.core.compression);
    println!(
        "  compression_level = {}",
        ctx.config.core.compression_level
    );

    println!("\n{}", "[performance]".bold());
    println!(
        "  parallel_threads = {}",
        ctx.config.performance.parallel_threads
    );
    println!(
        "  mmap_threshold = {}",
        ctx.config.performance.mmap_threshold
    );
    println!(
        "  use_hard_links = {}",
        ctx.config.performance.use_hard_links
    );

    println!("\n{}", "[tracking]".bold());
    println!(
        "  follow_symlinks = {}",
        ctx.config.tracking.follow_symlinks
    );
    println!(
        "  preserve_permissions = {}",
        ctx.config.tracking.preserve_permissions
    );

    if !ctx.config.branches.tracking.is_empty() {
        println!("\n{}", "[branch]".bold());
        for (branch, tracking) in &ctx.config.branches.tracking {
            println!("  {branch}.remote = {}", tracking.remote);
            println!("  {branch}.branch = {}", tracking.branch);
        }
    }

    if !ctx.config.remotes.is_empty() {
        println!("\n{}", "[remote]".bold());
        for (name, remote) in &ctx.config.remotes {
            println!("  {name}.type = {:?}", remote.remote_type);
            if let Some(url) = &remote.url {
                println!("  {name}.url = {url}");
            }
        }
    }
}
