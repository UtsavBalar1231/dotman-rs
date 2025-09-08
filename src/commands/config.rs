use crate::DotmanContext;
use anyhow::Result;
use colored::Colorize;

pub fn execute(
    ctx: &mut DotmanContext,
    key: Option<&str>,
    value: Option<String>,
    unset: bool,
    list: bool,
) -> Result<()> {
    // If --list flag is set or no key is provided, show all configuration
    if list || key.is_none() {
        show_all_config(ctx)?;
        return Ok(());
    }

    let key = key.unwrap();

    if unset {
        // Unset a configuration value
        ctx.config.unset(key)?;
        ctx.config.save(&ctx.config_path)?;
        super::print_success(&format!("Unset {}", key));
    } else if let Some(val) = value {
        // Set a configuration value
        ctx.config.set(key, val.clone())?;
        ctx.config.save(&ctx.config_path)?;
        super::print_success(&format!("Set {} = {}", key, val));
    } else if let Some(val) = ctx.config.get(key) {
        println!("{}", val);
    } else {
        super::print_warning(&format!("Configuration key '{}' is not set", key));
    }

    Ok(())
}

fn show_all_config(ctx: &DotmanContext) -> Result<()> {
    println!("{}", "[user]".bold());
    if let Some(name) = &ctx.config.user.name {
        println!("  name = {}", name);
    }
    if let Some(email) = &ctx.config.user.email {
        println!("  email = {}", email);
    }

    println!("\n{}", "[core]".bold());
    println!("  repo_path = {}", ctx.config.core.repo_path.display());
    println!("  compression = {:?}", ctx.config.core.compression);
    println!(
        "  compression_level = {}",
        ctx.config.core.compression_level
    );
    println!("  default_branch = {}", ctx.config.core.default_branch);

    println!("\n{}", "[performance]".bold());
    println!(
        "  parallel_threads = {}",
        ctx.config.performance.parallel_threads
    );
    println!(
        "  mmap_threshold = {}",
        ctx.config.performance.mmap_threshold
    );
    println!("  cache_size = {}", ctx.config.performance.cache_size);
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
            println!("  {}.remote = {}", branch, tracking.remote);
            println!("  {}.branch = {}", branch, tracking.branch);
        }
    }

    if !ctx.config.remotes.is_empty() {
        println!("\n{}", "[remote]".bold());
        for (name, remote) in &ctx.config.remotes {
            println!("  {}.type = {:?}", name, remote.remote_type);
            if let Some(url) = &remote.url {
                println!("  {}.url = {}", name, url);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use tempfile::tempdir;

    #[test]
    fn test_config_set_get() -> Result<()> {
        let dir = tempdir()?;
        let config_path = dir.path().join("config.toml");
        let repo_path = dir.path().join(".dotman");

        std::fs::create_dir_all(&repo_path)?;

        let config = Config::default();
        config.save(&config_path)?;

        let mut ctx = DotmanContext {
            repo_path,
            config_path: config_path.clone(),
            config,
            no_pager: true,
        };

        // Set user.name
        execute(
            &mut ctx,
            Some("user.name"),
            Some("Test User".to_string()),
            false,
            false,
        )?;

        // Reload config and verify
        let config = Config::load(&config_path)?;
        assert_eq!(config.user.name, Some("Test User".to_string()));

        // Set user.email
        ctx.config = config;
        execute(
            &mut ctx,
            Some("user.email"),
            Some("test@example.com".to_string()),
            false,
            false,
        )?;

        // Reload and verify both values
        let config = Config::load(&config_path)?;
        assert_eq!(config.user.name, Some("Test User".to_string()));
        assert_eq!(config.user.email, Some("test@example.com".to_string()));

        Ok(())
    }

    #[test]
    fn test_config_unset() -> Result<()> {
        let dir = tempdir()?;
        let config_path = dir.path().join("config.toml");
        let repo_path = dir.path().join(".dotman");

        std::fs::create_dir_all(&repo_path)?;

        let mut config = Config::default();
        config.user.name = Some("Test User".to_string());
        config.user.email = Some("test@example.com".to_string());
        config.save(&config_path)?;

        let mut ctx = DotmanContext {
            repo_path,
            config_path: config_path.clone(),
            config,
            no_pager: true,
        };

        // Unset user.name
        execute(&mut ctx, Some("user.name"), None, true, false)?;

        // Reload and verify
        let config = Config::load(&config_path)?;
        assert_eq!(config.user.name, None);
        assert_eq!(config.user.email, Some("test@example.com".to_string()));

        Ok(())
    }

    #[test]
    fn test_invalid_email() -> Result<()> {
        let dir = tempdir()?;
        let config_path = dir.path().join("config.toml");
        let repo_path = dir.path().join(".dotman");

        std::fs::create_dir_all(&repo_path)?;

        let config = Config::default();
        config.save(&config_path)?;

        let mut ctx = DotmanContext {
            repo_path,
            config_path: config_path.clone(),
            config,
            no_pager: true,
        };

        // Try to set invalid email
        let result = execute(
            &mut ctx,
            Some("user.email"),
            Some("invalid".to_string()),
            false,
            false,
        );
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_config_list() -> Result<()> {
        let dir = tempdir()?;
        let config_path = dir.path().join("config.toml");
        let repo_path = dir.path().join(".dotman");

        std::fs::create_dir_all(&repo_path)?;

        let mut config = Config::default();
        config.user.name = Some("Test User".to_string());
        config.user.email = Some("test@example.com".to_string());
        config.save(&config_path)?;

        let mut ctx = DotmanContext {
            repo_path,
            config_path: config_path.clone(),
            config,
            no_pager: true,
        };

        // Test list flag
        let result = execute(&mut ctx, None, None, false, true);
        assert!(result.is_ok());

        // Test no key provided (should also show all)
        let result = execute(&mut ctx, None, None, false, false);
        assert!(result.is_ok());

        Ok(())
    }
}
