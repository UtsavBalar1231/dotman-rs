pub mod parser;
pub mod validator;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub core: CoreConfig,

    /// Multiple named remotes (like git's origin, upstream, etc.)
    #[serde(default)]
    pub remotes: HashMap<String, RemoteConfig>,

    /// Branch tracking configuration
    #[serde(default)]
    pub branches: BranchConfig,

    #[serde(default)]
    pub performance: PerformanceConfig,

    #[serde(default)]
    pub tracking: TrackingConfig,

    /// User configuration (name and email for commits)
    #[serde(default)]
    pub user: UserConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConfig {
    #[serde(default = "default_repo_path")]
    pub repo_path: PathBuf,
    #[serde(default = "default_compression")]
    pub compression: CompressionType,
    #[serde(default = "default_compression_level")]
    pub compression_level: i32,
    #[serde(default)]
    pub pager: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompressionType {
    Zstd,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteConfig {
    pub remote_type: RemoteType,
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RemoteType {
    Git,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    #[serde(default = "default_parallel_threads")]
    pub parallel_threads: usize,
    #[serde(default = "default_mmap_threshold")]
    pub mmap_threshold: usize,
    #[serde(default = "default_use_hard_links")]
    pub use_hard_links: bool,
    // TODO: Add cache_size field when implementing object caching
    // pub cache_size: usize,  // Reserved for future LRU cache implementation
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingConfig {
    pub ignore_patterns: Vec<String>,
    pub follow_symlinks: bool,
    pub preserve_permissions: bool,
}

/// Branch configuration for tracking branches and their upstream remotes
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BranchConfig {
    /// Branch tracking information: `branch_name` -> (`remote_name`, `remote_branch`)
    #[serde(default)]
    pub tracking: HashMap<String, BranchTracking>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchTracking {
    pub remote: String,
    pub branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserConfig {
    pub name: Option<String>,
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

// Add dependency for num_cpus
static NUM_CPUS: std::sync::LazyLock<usize> = std::sync::LazyLock::new(|| {
    std::thread::available_parallelism()
        .map(std::num::NonZeroUsize::get)
        .unwrap_or(1)
});

mod num_cpus {
    use super::NUM_CPUS;

    pub fn get() -> usize {
        *NUM_CPUS
    }
}

// Default functions for serde
fn default_repo_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    home.join(".dotman")
}

const fn default_compression() -> CompressionType {
    CompressionType::Zstd
}

const fn default_compression_level() -> i32 {
    3
}

fn default_parallel_threads() -> usize {
    num_cpus::get().min(8)
}

const fn default_mmap_threshold() -> usize {
    1_048_576 // 1MB
}

const fn default_use_hard_links() -> bool {
    true
}
