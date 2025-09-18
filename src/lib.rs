pub mod commands;
pub mod config;
pub mod mapping;
pub mod mirror;
pub mod reflog;
pub mod refs;
pub mod storage;
pub mod sync;
pub mod utils;

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

/// Context for Dotman operations, holding configuration and repository information.
/// This struct is used throughout the application to access settings and paths.
/// # Fields
/// - `repo_path`: The path to the Dotman repository.
/// - `config_path`: The path to the configuration file.
/// - `config`: The loaded configuration settings.
/// - `no_pager`: A flag indicating whether to disable pager functionality.
impl DotmanContext {
    /// Creates a new `DotmanContext` by loading the configuration from the default path.
    ///
    /// # Errors
    /// Returns an error if the home directory cannot be determined or if the configuration
    /// file cannot be read or created.
    pub fn new() -> Result<Self> {
        Self::new_with_pager(false)
    }

    /// Creates a new `DotmanContext` with an option to disable pager functionality.
    ///
    /// # Errors
    /// Returns an error if the home directory cannot be determined or if the configuration
    /// file cannot be read or created.
    pub fn new_with_pager(no_pager: bool) -> Result<Self> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        let config_path = home.join(DEFAULT_CONFIG_PATH);
        let config = config::Config::load(&config_path)?;
        let repo_path = config.core.repo_path.clone();

        // Validate configuration and warn about issues
        let validator = config::validator::ConfigValidator::new();
        if let Err(e) = validator.validate_config_file(&config_path) {
            eprintln!("Warning: Configuration validation failed: {e}");
        }
        config::validator::ConfigValidator::warn_unused_options(&config);

        // Configure thread pool based on config
        if let Err(e) = utils::thread_pool::configure_from_config(&config) {
            eprintln!("Warning: Failed to configure thread pool: {e}");
        }

        Ok(Self {
            repo_path,
            config_path,
            config,
            no_pager,
        })
    }

    /// Creates a new `DotmanContext` with explicit paths for testing.
    /// This avoids the need for environment variable manipulation.
    ///
    /// # Errors
    /// Returns an error if the configuration cannot be loaded or created.
    pub fn new_with_explicit_paths(repo_path: PathBuf, config_path: PathBuf) -> Result<Self> {
        let config = if config_path.exists() {
            config::Config::load(&config_path)?
        } else {
            // Create a default config with the provided repo path
            let mut config = config::Config::default();
            config.core.repo_path.clone_from(&repo_path);

            // Ensure the config directory exists
            if let Some(parent) = config_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            // Save the config
            config.save(&config_path)?;
            config
        };

        Ok(Self {
            repo_path,
            config_path,
            config,
            no_pager: false,
        })
    }

    /// Creates a new `DotmanContext` with explicit paths and pager disabled.
    ///
    /// # Errors
    /// Returns an error if the configuration cannot be loaded or created.
    pub fn new_explicit(repo_path: PathBuf, config_path: PathBuf) -> Result<Self> {
        let mut context = Self::new_with_explicit_paths(repo_path, config_path)?;
        context.no_pager = true;
        Ok(context)
    }

    /// Checks if the repository is initialized by verifying the existence of
    #[must_use]
    pub fn is_repo_initialized(&self) -> bool {
        self.repo_path.exists()
            && self.repo_path.join(INDEX_FILE).exists()
            && self.repo_path.join("HEAD").exists()
    }

    /// Checks if the repository is initialized, returning an error if not.
    ///
    /// # Errors
    /// Returns an error if the repository is not initialized.
    pub fn check_repo_initialized(&self) -> Result<()> {
        if !self.is_repo_initialized() {
            return Err(anyhow::anyhow!(
                "Dotman repository not found in {}. Did you run 'dot init'?",
                self.repo_path.display()
            ));
        }
        Ok(())
    }

    /// Ensures that the repository directory and its subdirectories exist.
    ///
    /// # Errors
    /// Returns an error if the directories cannot be created.
    /// This can happen due to permission issues or invalid paths.
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
