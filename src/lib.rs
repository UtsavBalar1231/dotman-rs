pub mod commands;
pub mod config;
pub mod mapping;
pub mod mirror;
pub mod reflog;
pub mod refs;
pub mod storage;
pub mod sync;
pub mod utils;

#[cfg(test)]
pub mod test_utils;

use anyhow::{Context, Result};
use std::path::PathBuf;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_REPO_DIR: &str = ".dotman";
pub const DEFAULT_CONFIG_PATH: &str = ".config/dotman/config";
pub const INDEX_FILE: &str = "index.bin";
pub const COMMITS_DIR: &str = "commits";
pub const OBJECTS_DIR: &str = "objects";
pub const NULL_COMMIT_ID: &str = "0000000000000000000000000000000000000000";

#[derive(Debug, Clone)]
pub struct DotmanContext {
    pub repo_path: PathBuf,
    pub config_path: PathBuf,
    pub config: config::Config,
    pub no_pager: bool,
}

impl DotmanContext {
    pub fn new() -> Result<Self> {
        Self::new_with_pager(false)
    }

    pub fn new_with_pager(no_pager: bool) -> Result<Self> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        let config_path = home.join(DEFAULT_CONFIG_PATH);
        let config = config::Config::load(&config_path)?;
        let repo_path = config.core.repo_path.clone();

        Ok(Self {
            repo_path,
            config_path,
            config,
            no_pager,
        })
    }

    pub fn is_repo_initialized(&self) -> bool {
        self.repo_path.exists()
            && self.repo_path.join(INDEX_FILE).exists()
            && self.repo_path.join("HEAD").exists()
    }

    pub fn check_repo_initialized(&self) -> Result<()> {
        if !self.is_repo_initialized() {
            return Err(anyhow::anyhow!(
                "Dotman repository not found in {}. Did you run 'dot init'?",
                self.repo_path.display()
            ));
        }
        Ok(())
    }

    pub fn ensure_repo_exists(&self) -> Result<()> {
        std::fs::create_dir_all(&self.repo_path).with_context(|| {
            format!(
                "Failed to create repository directory: {}",
                self.repo_path.display()
            )
        })?;
        std::fs::create_dir_all(self.repo_path.join(COMMITS_DIR))
            .context("Failed to create commits directory")?;
        std::fs::create_dir_all(self.repo_path.join(OBJECTS_DIR))
            .context("Failed to create objects directory")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    #[serial_test::serial]
    fn test_dotman_context_new() -> Result<()> {
        let temp = tempdir()?;
        let config_path = temp.path().join(DEFAULT_CONFIG_PATH);

        fs::create_dir_all(
            config_path
                .parent()
                .context("Config path must have a parent directory")?,
        )?;

        let config_content = r#"
[core]
repo_path = "~/.dotman"
compression_level = 3

[branches]
current = "main"

[performance]
parallel_threads = 4
cache_size = 100
mmap_threshold = 1048576

[tracking]
ignore_patterns = []
follow_symlinks = false
preserve_permissions = true
"#;
        fs::write(&config_path, config_content)?;

        unsafe {
            std::env::set_var("HOME", temp.path());
        }

        let ctx = DotmanContext::new()?;
        assert!(ctx.repo_path.to_string_lossy().contains(".dotman"));
        assert_eq!(ctx.config_path, config_path);

        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_dotman_context_new_with_default_config() {
        let temp = tempdir().unwrap();

        unsafe {
            std::env::set_var("HOME", temp.path());
        }

        let result = DotmanContext::new();
        if let Err(e) = &result {
            eprintln!("Error creating context: {}", e);
        }
        assert!(
            result.is_ok(),
            "Failed to create context with default config"
        );

        let config_path = temp.path().join(DEFAULT_CONFIG_PATH);
        assert!(config_path.exists());
    }

    #[test]
    #[serial_test::serial]
    fn test_dotman_context_new_invalid_config() -> Result<()> {
        let temp = tempdir()?;
        let config_path = temp.path().join(DEFAULT_CONFIG_PATH);

        fs::create_dir_all(
            config_path
                .parent()
                .context("Config path must have a parent directory")?,
        )?;

        fs::write(&config_path, "invalid toml content {")?;

        unsafe {
            std::env::set_var("HOME", temp.path());
        }

        let result = DotmanContext::new();
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_ensure_repo_exists() -> Result<()> {
        let temp = tempdir()?;
        let repo_path = temp.path().join("test_repo");

        let ctx = DotmanContext {
            repo_path: repo_path.clone(),
            config_path: temp.path().join("config"),
            config: config::Config::default(),
            no_pager: true,
        };

        // Ensure directories don't exist initially
        assert!(!repo_path.exists());

        ctx.ensure_repo_exists()?;

        // Verify all directories were created
        assert!(repo_path.exists());
        assert!(repo_path.join(COMMITS_DIR).exists());
        assert!(repo_path.join(OBJECTS_DIR).exists());

        // Call again to ensure idempotency
        ctx.ensure_repo_exists()?;
        assert!(repo_path.exists());

        Ok(())
    }

    #[test]
    fn test_ensure_repo_exists_permission_denied() -> Result<()> {
        // Skip this test if running as root (common in CI/Docker environments)
        // Root can bypass permission restrictions
        #[cfg(unix)]
        {
            if unsafe { libc::getuid() } == 0 {
                println!("Skipping permission test when running as root");
                return Ok(());
            }
        }

        let temp = tempdir()?;
        let readonly_dir = temp.path().join("readonly");
        fs::create_dir(&readonly_dir)?;

        // Make directory read-only
        let mut perms = fs::metadata(&readonly_dir)?.permissions();
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o444);
        fs::set_permissions(&readonly_dir, perms)?;

        let repo_path = readonly_dir.join("test_repo");
        let ctx = DotmanContext {
            repo_path: repo_path.clone(),
            config_path: temp.path().join("config"),
            config: config::Config::default(),
            no_pager: true,
        };

        let result = ctx.ensure_repo_exists();
        assert!(result.is_err());

        // Restore permissions for cleanup
        let mut perms = fs::metadata(&readonly_dir)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&readonly_dir, perms)?;

        Ok(())
    }

    #[test]
    fn test_ensure_repo_exists_nested_path() -> Result<()> {
        let temp = tempdir()?;
        let repo_path = temp.path().join("deeply").join("nested").join("repo");

        let ctx = DotmanContext {
            repo_path: repo_path.clone(),
            config_path: temp.path().join("config"),
            config: config::Config::default(),
            no_pager: true,
        };

        ctx.ensure_repo_exists()?;

        // Verify deeply nested structure was created
        assert!(repo_path.exists());
        assert!(repo_path.join(COMMITS_DIR).exists());
        assert!(repo_path.join(OBJECTS_DIR).exists());

        Ok(())
    }
}
