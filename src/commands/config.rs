use crate::DotmanContext;
use anyhow::Result;

pub fn execute(
    ctx: &mut DotmanContext,
    key: &str,
    value: Option<String>,
    unset: bool,
) -> Result<()> {
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
    } else {
        // Get a configuration value
        if let Some(val) = ctx.config.get(key) {
            println!("{}", val);
        } else {
            super::print_warning(&format!("Configuration key '{}' is not set", key));
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
        };

        // Set user.name
        execute(&mut ctx, "user.name", Some("Test User".to_string()), false)?;

        // Reload config and verify
        let config = Config::load(&config_path)?;
        assert_eq!(config.user.name, Some("Test User".to_string()));

        // Set user.email
        ctx.config = config;
        execute(
            &mut ctx,
            "user.email",
            Some("test@example.com".to_string()),
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
        };

        // Unset user.name
        execute(&mut ctx, "user.name", None, true)?;

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
        };

        // Try to set invalid email
        let result = execute(&mut ctx, "user.email", Some("invalid".to_string()), false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid email"));

        Ok(())
    }
}
