//! Configuration parsing, validation, and management.
//!
//! This module provides the configuration system for dotman, including:
//!
//! - TOML-based configuration files
//! - Multiple remote repository support
//! - Branch tracking configuration
//! - Performance tuning options
//! - File tracking and ignore patterns
//! - User identity configuration
//!
//! # Configuration File Location
//!
//! Default: `~/.config/dotman/config`
//! Override with: `DOTMAN_CONFIG_PATH` environment variable
//!
//! # Configuration Structure
//!
//! ```toml
//! [core]
//! compression = "zstd"
//! compression_level = 3
//!
//! [user]
//! name = "Your Name"
//! email = "you@example.com"
//!
//! [performance]
//! parallel_threads = 8
//! mmap_threshold = 1048576
//!
//! [tracking]
//! ignore_patterns = [".git", "*.swp"]
//! follow_symlinks = false
//! preserve_permissions = true
//! ```
//!
//! # Examples
//!
//! ```no_run
//! use dotman::config::Config;
//! use std::path::PathBuf;
//!
//! # fn main() -> anyhow::Result<()> {
//! // Load config from default location
//! let config = Config::load(&PathBuf::from("~/.config/dotman/config"))?;
//!
//! // Modify and save
//! let mut config = Config::default();
//! config.set("user.name", "Alice".to_string())?;
//! config.save(&PathBuf::from("config.toml"))?;
//! # Ok(())
//! # }
//! ```

/// Configuration file parsing utilities.
///
/// This module provides functionality for parsing TOML configuration files
/// and converting them into strongly-typed configuration structures.
pub mod parser;

/// Configuration validation utilities.
///
/// This module provides validation logic for configuration values,
/// ensuring they meet the required constraints and are semantically correct.
pub mod validator;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Main configuration structure for dotman.
///
/// This structure contains all configuration sections including core settings,
/// remote repositories, performance tuning, file tracking, and user identity.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Core dotman settings (compression, repository path, etc.).
    #[serde(default)]
    pub core: CoreConfig,

    /// Multiple named remotes (like git's origin, upstream, etc.).
    #[serde(default)]
    pub remotes: HashMap<String, RemoteConfig>,

    /// Branch tracking configuration.
    #[serde(default)]
    pub branches: BranchConfig,

    /// Performance optimization settings.
    #[serde(default)]
    pub performance: PerformanceConfig,

    /// File tracking and ignore patterns.
    #[serde(default)]
    pub tracking: TrackingConfig,

    /// User configuration (name and email for commits).
    #[serde(default)]
    pub user: UserConfig,
}

/// Core dotman configuration settings.
///
/// Contains fundamental settings like repository path, compression
/// algorithm and level, and pager configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConfig {
    /// Path to the dotman repository directory. Default: `~/.dotman`
    #[serde(default = "default_repo_path")]
    pub repo_path: PathBuf,

    /// Compression algorithm for snapshots. Default: Zstd
    #[serde(default = "default_compression")]
    pub compression: CompressionType,

    /// Compression level (1-22 for Zstd). Default: 3
    #[serde(default = "default_compression_level")]
    pub compression_level: i32,

    /// Optional pager command for displaying output.
    #[serde(default)]
    pub pager: Option<String>,
}

/// Compression algorithm type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompressionType {
    /// Zstandard compression (high speed, good ratio)
    Zstd,
    /// No compression
    None,
}

/// Remote repository configuration.
///
/// Defines a remote repository connection, similar to git remotes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteConfig {
    /// Type of remote (Git or None).
    pub remote_type: RemoteType,

    /// URL of the remote repository (if applicable).
    pub url: Option<String>,
}

/// Remote repository type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RemoteType {
    /// Git-based remote repository
    Git,
    /// No remote (local only)
    None,
}

/// Performance optimization configuration.
///
/// Controls parallel processing, memory-mapped I/O, and hard link usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Number of parallel threads for operations. Default: min(CPU count, 8)
    #[serde(default = "default_parallel_threads")]
    pub parallel_threads: usize,

    /// Memory-map threshold in bytes (files larger use mmap). Default: 1 MB
    #[serde(default = "default_mmap_threshold")]
    pub mmap_threshold: usize,

    /// Whether to use hard links when possible. Default: true
    #[serde(default = "default_use_hard_links")]
    pub use_hard_links: bool,
    // TODO: Add cache_size field when implementing object caching
    // pub cache_size: usize,  // Reserved for future LRU cache implementation
}

/// File tracking and ignore configuration.
///
/// Controls which files are tracked and how they are processed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingConfig {
    /// Patterns to ignore when adding files (glob-style).
    pub ignore_patterns: Vec<String>,

    /// Whether to follow symbolic links during directory traversal.
    pub follow_symlinks: bool,

    /// Whether to preserve file permissions in snapshots.
    pub preserve_permissions: bool,

    /// Warn when adding files larger than this size (in bytes). Default: 100 MB
    #[serde(default = "default_large_file_threshold")]
    pub large_file_threshold: u64,
}

/// Branch tracking configuration.
///
/// Maps branch names to their upstream remote tracking information.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BranchConfig {
    /// Branch tracking information: `branch_name` -> [`BranchTracking`]
    #[serde(default)]
    pub tracking: HashMap<String, BranchTracking>,
}

/// Upstream tracking information for a branch.
///
/// Associates a local branch with a remote and remote branch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchTracking {
    /// Name of the remote (e.g., "origin")
    pub remote: String,

    /// Name of the branch on the remote
    pub branch: String,
}

/// User identity configuration.
///
/// Used for commit authorship information.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserConfig {
    /// User's full name for commits.
    pub name: Option<String>,

    /// User's email address for commits.
    pub email: Option<String>,
}

impl Default for CoreConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        Self {
            repo_path: home.join(".dotman"),
            compression: CompressionType::Zstd,
            compression_level: 3,
            pager: None,
        }
    }
}

impl Default for RemoteConfig {
    fn default() -> Self {
        Self {
            remote_type: RemoteType::None,
            url: None,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        let cpu_count = num_cpus::get();
        Self {
            parallel_threads: cpu_count.min(8),
            mmap_threshold: 1_048_576, // 1MB
            use_hard_links: true,
        }
    }
}

impl Default for TrackingConfig {
    fn default() -> Self {
        Self {
            ignore_patterns: vec![
                ".git".to_string(),
                "*.swp".to_string(),
                "*.tmp".to_string(),
                "node_modules".to_string(),
                "__pycache__".to_string(),
            ],
            follow_symlinks: false,
            preserve_permissions: true,
            large_file_threshold: default_large_file_threshold(),
        }
    }
}

impl Config {
    /// Get branch tracking information for a branch
    #[must_use]
    pub fn get_branch_tracking(&self, branch: &str) -> Option<&BranchTracking> {
        self.branches.tracking.get(branch)
    }

    /// Set branch tracking information
    pub fn set_branch_tracking(&mut self, branch: String, tracking: BranchTracking) {
        self.branches.tracking.insert(branch, tracking);
    }

    /// Remove branch tracking information
    pub fn remove_branch_tracking(&mut self, branch: &str) -> Option<BranchTracking> {
        self.branches.tracking.remove(branch)
    }

    /// Load configuration from a file
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Cannot create parent directories
    /// - Cannot read or parse the configuration file
    /// - Configuration file contains invalid TOML
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            // Create default config if it doesn't exist
            let config = Self::default();
            config.save(path)?;
            return Ok(config);
        }

        // Use our fast parser for loading
        parser::parse_config_file(path)
    }

    /// Save configuration to a file
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Cannot create parent directories
    /// - Cannot write to the file
    /// - TOML serialization fails
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let toml_str = toml::to_string_pretty(self)?;
        let mut file = std::fs::File::create(path)?;
        file.write_all(toml_str.as_bytes())?;
        Ok(())
    }

    /// Get a remote by name
    #[must_use]
    pub fn get_remote(&self, name: &str) -> Option<&RemoteConfig> {
        self.remotes.get(name)
    }

    /// Add or update a remote
    pub fn set_remote(&mut self, name: String, remote: RemoteConfig) {
        self.remotes.insert(name, remote);
    }

    /// Remove a remote
    pub fn remove_remote(&mut self, name: &str) -> Option<RemoteConfig> {
        self.remotes.remove(name)
    }

    /// Get a configuration value by key
    #[must_use]
    pub fn get(&self, key: &str) -> Option<String> {
        let parts: Vec<&str> = key.split('.').collect();
        if parts.len() != 2 {
            return None;
        }

        match (parts[0], parts[1]) {
            ("user", "name") => self.user.name.clone(),
            ("user", "email") => self.user.email.clone(),
            ("core", "compression") => Some(format!("{:?}", self.core.compression).to_lowercase()),
            ("core", "compression_level") => Some(self.core.compression_level.to_string()),
            ("core", "pager") => self.core.pager.clone(),
            ("performance", "parallel_threads") => {
                Some(self.performance.parallel_threads.to_string())
            }
            ("performance", "mmap_threshold") => Some(self.performance.mmap_threshold.to_string()),
            ("performance", "use_hard_links") => Some(self.performance.use_hard_links.to_string()),
            ("tracking", "follow_symlinks") => Some(self.tracking.follow_symlinks.to_string()),
            ("tracking", "preserve_permissions") => {
                Some(self.tracking.preserve_permissions.to_string())
            }
            _ => None,
        }
    }

    /// Set a configuration value by key
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The key format is invalid (must be section.key)
    /// - The key is unknown
    /// - The value is invalid for the key (e.g., invalid email)
    pub fn set(&mut self, key: &str, value: String) -> Result<()> {
        let parts: Vec<&str> = key.split('.').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid configuration key: {key}"));
        }

        match (parts[0], parts[1]) {
            ("user", "name") => self.user.name = Some(value),
            ("user", "email") => {
                // Basic email validation
                if !value.contains('@') {
                    return Err(anyhow::anyhow!("Invalid email address: {value}"));
                }
                self.user.email = Some(value);
            }
            ("core", "compression_level") => {
                let level: i32 = value
                    .parse()
                    .with_context(|| format!("Invalid compression level: {value}"))?;
                if !(1..=22).contains(&level) {
                    return Err(anyhow::anyhow!(
                        "Compression level must be between 1 and 22"
                    ));
                }
                self.core.compression_level = level;
            }
            ("core", "pager") => self.core.pager = Some(value),
            ("performance", "parallel_threads") => {
                self.performance.parallel_threads = value
                    .parse()
                    .with_context(|| format!("Invalid number: {value}"))?;
            }
            ("performance", "mmap_threshold") => {
                self.performance.mmap_threshold = value
                    .parse()
                    .with_context(|| format!("Invalid number: {value}"))?;
            }
            ("performance", "use_hard_links") => {
                self.performance.use_hard_links = value
                    .parse()
                    .with_context(|| format!("Invalid boolean: {value}"))?;
            }
            ("tracking", "follow_symlinks") => {
                self.tracking.follow_symlinks = value
                    .parse()
                    .with_context(|| format!("Invalid boolean: {value}"))?;
            }
            ("tracking", "preserve_permissions") => {
                self.tracking.preserve_permissions = value
                    .parse()
                    .with_context(|| format!("Invalid boolean: {value}"))?;
            }
            _ => return Err(anyhow::anyhow!("Unknown configuration key: {key}")),
        }
        Ok(())
    }

    /// Unset a configuration value by key
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The key format is invalid (must be section.key)
    /// - The key is unknown or cannot be unset
    pub fn unset(&mut self, key: &str) -> Result<()> {
        let parts: Vec<&str> = key.split('.').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid configuration key: {key}"));
        }

        match (parts[0], parts[1]) {
            ("user", "name") => self.user.name = None,
            ("user", "email") => self.user.email = None,
            ("core", "pager") => self.core.pager = None,
            _ => return Err(anyhow::anyhow!("Cannot unset configuration key: {key}")),
        }
        Ok(())
    }
}

/// Cached number of available CPUs/threads on the system.
///
/// This static is lazily initialized on first access and caches the result
/// of querying the system's available parallelism. Falls back to 1 if the
/// query fails.
static NUM_CPUS: std::sync::LazyLock<usize> = std::sync::LazyLock::new(|| {
    std::thread::available_parallelism()
        .map(std::num::NonZeroUsize::get)
        .unwrap_or(1)
});

/// Internal module for CPU count queries.
///
/// Provides a simple interface to query the cached number of available CPUs
/// without directly exposing the static.
mod num_cpus {
    use super::NUM_CPUS;

    /// Returns the number of available CPUs/threads on the system.
    ///
    /// # Returns
    ///
    /// The number of available CPUs, determined at first call and cached.
    /// Returns 1 if the system query fails.
    pub fn get() -> usize {
        *NUM_CPUS
    }
}

/// Returns the default repository path for dotman.
///
/// This function is used by serde as the default value provider for the
/// repository path configuration field.
///
/// # Returns
///
/// A `PathBuf` pointing to `~/.dotman`, or `/tmp/.dotman` if the home
/// directory cannot be determined.
fn default_repo_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    home.join(".dotman")
}

/// Returns the default compression type.
///
/// This function is used by serde as the default value provider for the
/// compression type configuration field.
///
/// # Returns
///
/// `CompressionType::Zstd` - Zstandard compression as the default algorithm.
const fn default_compression() -> CompressionType {
    CompressionType::Zstd
}

/// Returns the default compression level.
///
/// This function is used by serde as the default value provider for the
/// compression level configuration field.
///
/// # Returns
///
/// `3` - A balanced compression level providing good compression ratio
/// with reasonable performance.
const fn default_compression_level() -> i32 {
    3
}

/// Returns the default number of parallel threads.
///
/// This function is used by serde as the default value provider for the
/// parallel threads configuration field. It caps the thread count at 8
/// to prevent excessive resource usage even on high-core-count systems.
///
/// # Returns
///
/// The minimum of the system's available CPU count and 8.
fn default_parallel_threads() -> usize {
    num_cpus::get().min(8)
}

/// Returns the default memory-mapped I/O threshold.
///
/// This function is used by serde as the default value provider for the
/// mmap threshold configuration field. Files larger than this threshold
/// will use memory-mapped I/O for better performance.
///
/// # Returns
///
/// `1_048_576` (1 MB) - Files larger than 1 MB will use mmap.
const fn default_mmap_threshold() -> usize {
    1_048_576 // 1MB
}

/// Returns the default setting for using hard links.
///
/// This function is used by serde as the default value provider for the
/// `use_hard_links` configuration field.
///
/// # Returns
///
/// `true` - Hard links are enabled by default to save disk space.
const fn default_use_hard_links() -> bool {
    true
}

/// Returns the default large file threshold.
///
/// This function is used by serde as the default value provider for the
/// large file threshold configuration field. Files larger than this size
/// may be handled differently (e.g., with special warnings or processing).
///
/// # Returns
///
/// `104_857_600` (100 MB) - Files larger than 100 MB are considered large.
const fn default_large_file_threshold() -> u64 {
    100 * 1024 * 1024 // 100 MB
}
