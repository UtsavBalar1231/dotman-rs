#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]
// Allow pedantic strict lints that create false positives in this codebase
#![allow(clippy::arithmetic_side_effects)] // Simple counters and size calculations cannot overflow
#![allow(clippy::float_arithmetic)] // Required for file size/time formatting
#![allow(clippy::indexing_slicing)] // Bounds checked by logic

//! # Dotman - High-Performance Dotfiles Manager
//!
//! Dotman is a Git-like dotfiles manager built in Rust, designed for maximum performance
//! and reliability through content-addressed storage, parallel processing, and binary indexing.
//!
//! ## Features
//!
//! - **Content-Addressed Storage**: Files are hashed with xxHash3 and deduplicated automatically
//! - **Parallel Processing**: Uses Rayon for multi-core operations
//! - **SIMD Acceleration**: Uses SIMD for UTF-8 validation and JSON parsing
//! - **Binary Indexing**: Fast file tracking with bincode serialization
//! - **Git-like Semantics**: Familiar command interface (add, commit, checkout, branch, etc.)
//! - **Compression**: Zstandard compression with configurable levels
//!
//! ## Architecture
//!
//! The codebase is organized into several key modules:
//!
//! - [`commands`]: Command implementations (add, commit, checkout, etc.)
//! - [`storage`]: Core storage layer with index and snapshot management
//! - [`tracking`]: Directory and file tracking manifest system
//! - [`config`]: Configuration parsing and validation
//! - [`refs`]: Reference and branch management
//! - [`scanner`]: Filesystem scanning and directory traversal utilities
//! - [`output`]: Output formatting, styling, and progress display
//! - [`utils`]: Utility functions and helpers
//!
//! ## Example Usage
//!
//! ```no_run
//! use dotman::DotmanContext;
//!
//! # fn main() -> anyhow::Result<()> {
//! // Create a new context
//! let ctx = DotmanContext::new()?;
//!
//! // Initialize repository
//! dotman::commands::init::execute(false)?;
//!
//! // Add files
//! dotman::commands::add::execute(&ctx, &["~/.bashrc".to_string()], false, false)?;
//!
//! // Commit changes
//! dotman::commands::commit::execute(&ctx, "Initial commit", false)?;
//! # Ok(())
//! # }
//! ```

/// Command-line interface definitions (argument parsing structures).
pub mod cli;

/// Commands module containing all CLI command implementations.
pub mod commands;

/// Configuration parsing, validation, and management.
pub mod config;

/// Diff generation for file comparisons (unified diff, binary detection).
pub mod diff;

/// Merge conflict detection and resolution.
pub mod conflicts;

/// Operation locking to prevent concurrent remote operations.
pub mod lock;

/// File mapping and path resolution utilities.
pub mod mapping;

/// Mirror repository synchronization.
pub mod mirror;

/// Output formatting and progress display.
pub mod output;

/// Reflog for tracking reference changes.
pub mod reflog;

/// Reference and branch management (HEAD, branches, tags).
pub mod refs;

/// Filesystem scanning and directory traversal utilities.
pub mod scanner;

/// Core storage layer including index, snapshots, and file operations.
pub mod storage;

/// Synchronization and remote operations.
pub mod sync;

/// Tracking system for managing tracked directories and files.
pub mod tracking;

/// Utility functions and helpers.
pub mod utils;

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Current version of the dotman binary.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default repository directory name within the home directory.
pub const DEFAULT_REPO_DIR: &str = ".dotman";

/// Default configuration file path relative to home directory.
pub const DEFAULT_CONFIG_PATH: &str = ".config/dotman/config";

/// Name of the binary index file.
pub const INDEX_FILE: &str = "index.bin";

/// Directory name for storing commit snapshots.
pub const COMMITS_DIR: &str = "commits";

/// Directory name for content-addressed object storage.
pub const OBJECTS_DIR: &str = "objects";

/// Placeholder commit ID representing no commits (32-character xxHash3 format).
pub const NULL_COMMIT_ID: &str = "00000000000000000000000000000000";

/// Central context for all Dotman operations.
///
/// This structure holds the repository path, configuration, and settings
/// needed for executing commands. It provides the primary interface for
/// interacting with a dotman repository.
///
/// # Fields
///
/// - `repo_path`: Path to the `.dotman` repository directory
/// - `config_path`: Path to the configuration file
/// - `config`: Loaded configuration settings
/// - `no_pager`: Whether to disable pager output
///
/// # Examples
///
/// ```no_run
/// use dotman::DotmanContext;
///
/// # fn main() -> anyhow::Result<()> {
/// // Create context with default paths
/// let ctx = DotmanContext::new()?;
///
/// // Create context with custom paths (for testing)
/// let ctx = DotmanContext::new_explicit(
///     "/tmp/test_repo".into(),
///     "/tmp/test_config".into()
/// )?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct DotmanContext {
    /// Path to the dotman repository directory.
    pub repo_path: PathBuf,

    /// Path to the configuration file.
    pub config_path: PathBuf,

    /// Loaded configuration settings.
    pub config: config::Config,

    /// Whether to disable pager output for command results.
    pub no_pager: bool,

    /// Whether to run in non-interactive mode (no prompts).
    /// Used primarily for testing to prevent stdin reads.
    pub non_interactive: bool,
}

impl DotmanContext {
    /// Creates a new `DotmanContext` by loading the configuration from the default path.
    ///
    /// # Errors
    /// Returns an error if the home directory cannot be determined or if the configuration
    /// file cannot be read or created.
    pub fn new() -> Result<Self> {
        Self::new_with_pager(true)
    }

    /// Creates a new `DotmanContext` with an option to disable pager functionality.
    ///
    /// # Errors
    /// Returns an error if the home directory cannot be determined or if the configuration
    /// file cannot be read or created.
    pub fn new_with_pager(no_pager: bool) -> Result<Self> {
        // Check environment variable for config path first
        let config_path = if let Ok(path) = std::env::var("DOTMAN_CONFIG_PATH") {
            PathBuf::from(path)
        } else {
            let home = dirs::home_dir().context("Could not find home directory")?;
            home.join(DEFAULT_CONFIG_PATH)
        };

        let config = config::Config::load(&config_path)?;

        // Allow environment variable to override config repo_path
        let repo_path = if let Ok(path) = std::env::var("DOTMAN_REPO_PATH") {
            PathBuf::from(path)
        } else {
            config.core.repo_path.clone()
        };

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
            non_interactive: false,
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
            non_interactive: false,
        })
    }

    /// Creates a new `DotmanContext` with explicit paths and pager disabled.
    ///
    /// # Errors
    /// Returns an error if the configuration cannot be loaded or created.
    pub fn new_explicit(repo_path: PathBuf, config_path: PathBuf) -> Result<Self> {
        let mut context = Self::new_with_explicit_paths(repo_path, config_path)?;
        context.no_pager = true;
        context.non_interactive = true;
        Ok(context)
    }

    /// Checks if the repository is initialized by verifying the existence of required files.
    ///
    /// A repository is considered initialized if the repository directory exists
    /// and contains both the index file and HEAD reference.
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
                "Repository not initialized: Dotman repository not found in {}. Did you run 'dot init'?",
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
