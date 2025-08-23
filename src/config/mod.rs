pub mod parser;

use anyhow::Result;
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConfig {
    #[serde(default = "default_repo_path")]
    pub repo_path: PathBuf,
    #[serde(default = "default_branch")]
    pub default_branch: String,
    #[serde(default = "default_compression")]
    pub compression: CompressionType,
    #[serde(default = "default_compression_level")]
    pub compression_level: i32,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RemoteType {
    Git,
    S3,
    Rsync,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    #[serde(default = "default_parallel_threads")]
    pub parallel_threads: usize,
    #[serde(default = "default_mmap_threshold")]
    pub mmap_threshold: usize,
    #[serde(default = "default_cache_size")]
    pub cache_size: usize,
    #[serde(default = "default_use_hard_links")]
    pub use_hard_links: bool,
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
    /// Current active branch
    #[serde(default = "default_current_branch")]
    pub current: String,

    /// Branch tracking information: branch_name -> (remote_name, remote_branch)
    #[serde(default)]
    pub tracking: HashMap<String, BranchTracking>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchTracking {
    pub remote: String,
    pub branch: String,
}

impl Default for CoreConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        Self {
            repo_path: home.join(".dotman"),
            default_branch: "main".to_string(),
            compression: CompressionType::Zstd,
            compression_level: 3,
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
            cache_size: 100,           // MB
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
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            // Create default config if it doesn't exist
            let config = Config::default();
            config.save(path)?;
            return Ok(config);
        }

        // Use our fast parser for loading
        parser::parse_config_file(path)
    }

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
}

// Add dependency for num_cpus
use once_cell::sync::Lazy;

static NUM_CPUS: Lazy<usize> = Lazy::new(|| {
    std::thread::available_parallelism()
        .map(|n| n.get())
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

fn default_branch() -> String {
    "main".to_string()
}

fn default_compression() -> CompressionType {
    CompressionType::Zstd
}

fn default_compression_level() -> i32 {
    3
}

fn default_parallel_threads() -> usize {
    num_cpus::get().min(8)
}

fn default_mmap_threshold() -> usize {
    1_048_576 // 1MB
}

fn default_cache_size() -> usize {
    100 // MB
}

fn default_use_hard_links() -> bool {
    true
}

fn default_current_branch() -> String {
    "main".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.core.default_branch, "main");
        assert!(config.performance.parallel_threads > 0);
    }

    #[test]
    fn test_save_and_load() -> Result<()> {
        let dir = tempdir()?;
        let config_path = dir.path().join("config.toml");

        let config = Config::default();
        config.save(&config_path)?;

        let loaded = Config::load(&config_path)?;
        assert_eq!(loaded.core.default_branch, config.core.default_branch);

        Ok(())
    }
}
