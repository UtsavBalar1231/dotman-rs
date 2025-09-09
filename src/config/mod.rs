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
    #[serde(default = "default_branch")]
    pub default_branch: String,
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
            default_branch: "main".to_string(),
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
            ("core", "default_branch") => Some(self.core.default_branch.clone()),
            ("core", "pager") => self.core.pager.clone(),
            ("performance", "parallel_threads") => {
                Some(self.performance.parallel_threads.to_string())
            }
            ("performance", "mmap_threshold") => Some(self.performance.mmap_threshold.to_string()),
            ("performance", "cache_size") => Some(self.performance.cache_size.to_string()),
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
            ("core", "default_branch") => self.core.default_branch = value,
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
            ("performance", "cache_size") => {
                self.performance.cache_size = value
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

fn default_branch() -> String {
    "main".to_string()
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

const fn default_cache_size() -> usize {
    100 // MB
}

const fn default_use_hard_links() -> bool {
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

    #[test]
    fn test_user_config_get_set() -> Result<()> {
        let mut config = Config::default();

        // Test setting user.name
        config.set("user.name", "Test User".to_string())?;
        assert_eq!(config.get("user.name"), Some("Test User".to_string()));
        assert_eq!(config.user.name, Some("Test User".to_string()));

        // Test setting user.email
        config.set("user.email", "test@example.com".to_string())?;
        assert_eq!(
            config.get("user.email"),
            Some("test@example.com".to_string())
        );
        assert_eq!(config.user.email, Some("test@example.com".to_string()));

        // Test unsetting
        config.unset("user.name")?;
        assert_eq!(config.get("user.name"), None);
        assert_eq!(config.user.name, None);

        Ok(())
    }

    #[test]
    fn test_invalid_email_validation() {
        let mut config = Config::default();

        // Test invalid email without @ symbol
        let result = config.set("user.email", "invalid".to_string());
        // Should fail for invalid email format
        assert!(result.is_err());

        // Valid email should work
        let result = config.set("user.email", "valid@example.com".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_get_various_keys() {
        let mut config = Config::default();
        config.core.compression_level = 5;
        config.performance.parallel_threads = 8;

        // Test getting various config values
        assert_eq!(config.get("core.compression_level"), Some("5".to_string()));
        assert_eq!(config.get("core.default_branch"), Some("main".to_string()));
        assert_eq!(
            config.get("performance.parallel_threads"),
            Some("8".to_string())
        );
        assert_eq!(
            config.get("performance.use_hard_links"),
            Some("true".to_string())
        );

        // Test getting non-existent keys
        assert_eq!(config.get("invalid.key"), None);
        assert_eq!(config.get("user.name"), None);
    }

    #[test]
    fn test_user_config_persistence() -> Result<()> {
        let dir = tempdir()?;
        let config_path = dir.path().join("config.toml");

        // Create config with user settings
        let mut config = Config::default();
        config.user.name = Some("John Doe".to_string());
        config.user.email = Some("john@example.com".to_string());
        config.save(&config_path)?;

        let loaded = Config::load(&config_path)?;
        assert_eq!(loaded.user.name, Some("John Doe".to_string()));
        assert_eq!(loaded.user.email, Some("john@example.com".to_string()));

        Ok(())
    }

    #[test]
    fn test_extreme_config_values() -> Result<()> {
        let dir = tempdir()?;
        let config_path = dir.path().join("extreme.toml");

        // Test config with extreme but technically valid values
        let extreme_config = Config {
            core: CoreConfig {
                repo_path: dir.path().join("repo"),
                default_branch: "a".repeat(1000), // Very long branch name
                compression: CompressionType::Zstd,
                compression_level: 22, // Maximum zstd compression level
                pager: None,
            },
            remotes: {
                let mut remotes = std::collections::HashMap::new();
                remotes.insert(
                    "origin".to_string(),
                    RemoteConfig {
                        remote_type: RemoteType::Git,
                        url: Some("file:///".to_string() + &"very_long_path/".repeat(100)),
                    },
                );
                remotes
            },
            branches: BranchConfig::default(),
            performance: PerformanceConfig {
                parallel_threads: 1024, // Very high thread count
                mmap_threshold: 1,      // Everything uses mmap
                cache_size: 10000,      // 10GB cache (max allowed)
                use_hard_links: true,
            },
            tracking: TrackingConfig {
                ignore_patterns: (0..10000).map(|i| format!("pattern_{i}")).collect(), // Many patterns
                follow_symlinks: true,
                preserve_permissions: true,
            },
            user: UserConfig::default(),
        };

        // Should be able to save extreme config
        extreme_config.save(&config_path)?;

        // Should be able to load it back
        let loaded_config = Config::load(&config_path)?;
        assert_eq!(loaded_config.core.compression_level, 22);
        assert_eq!(loaded_config.performance.parallel_threads, 1024);
        assert_eq!(loaded_config.performance.cache_size, 10000);
        assert_eq!(loaded_config.tracking.ignore_patterns.len(), 10000);

        Ok(())
    }

    #[test]
    fn test_config_unicode_and_special_chars() -> Result<()> {
        let dir = tempdir()?;
        let config_path = dir.path().join("unicode.toml");

        let unicode_config = Config {
            core: CoreConfig {
                repo_path: dir.path().join("dotman"),
                default_branch: "主分支".to_string(), // Chinese for "main branch"
                compression: CompressionType::Zstd,
                compression_level: 3,
                pager: None,
            },
            remotes: {
                let mut remotes = std::collections::HashMap::new();
                remotes.insert(
                    "origin".to_string(),
                    RemoteConfig {
                        remote_type: RemoteType::Git,
                        url: Some("git@github.com:用户/dotfiles.git".to_string()), // Mixed script
                    },
                );
                remotes
            },
            branches: BranchConfig::default(),
            performance: PerformanceConfig {
                parallel_threads: 8,
                mmap_threshold: 1_048_576,
                cache_size: 100,
                use_hard_links: true,
            },
            tracking: TrackingConfig {
                ignore_patterns: vec![
                    "*.log".to_string(),
                    "temp".to_string(),
                    "naïve_file".to_string(), // Accented characters
                    "файл*.txt".to_string(),  // Cyrillic
                ],
                follow_symlinks: false,
                preserve_permissions: true,
            },
            user: UserConfig::default(),
        };

        // Should handle Unicode in config
        unicode_config.save(&config_path)?;

        let loaded_config = Config::load(&config_path)?;
        assert_eq!(loaded_config.core.default_branch, "主分支");
        assert!(
            loaded_config
                .tracking
                .ignore_patterns
                .contains(&"файл*.txt".to_string())
        );

        Ok(())
    }

    #[test]
    fn test_config_validation_limits() -> Result<()> {
        let dir = tempdir()?;

        let invalid_configs = vec![
            (
                "cache_size_too_large",
                Config {
                    performance: PerformanceConfig {
                        cache_size: 100_000, // Over 10GB limit
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ),
            (
                "compression_level_too_high",
                Config {
                    core: CoreConfig {
                        compression_level: 99, // Too high
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ),
            (
                "zero_threads",
                Config {
                    performance: PerformanceConfig {
                        parallel_threads: 0, // Zero threads invalid
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ),
        ];

        for (test_name, invalid_config) in invalid_configs {
            let config_path = dir.path().join(format!("{test_name}.toml"));

            // These should fail validation when loading
            let result = invalid_config.save(&config_path);

            // If save succeeds, load should still validate
            if result.is_ok() {
                let loaded = Config::load(&config_path);
                // The load might apply defaults or clamp values
                if let Ok(config) = loaded {
                    // Verify values are clamped to valid ranges
                    assert!(config.performance.cache_size <= 10000);
                    assert!(config.core.compression_level <= 22);
                    assert!(config.performance.parallel_threads > 0);
                }
            }
        }

        Ok(())
    }

    #[test]
    fn test_malformed_config_recovery() -> Result<()> {
        let dir = tempdir()?;
        let config_path = dir.path().join("malformed.toml");

        // Write invalid TOML
        std::fs::write(&config_path, "invalid toml content {{ broken")?;

        // Try to load - should fail
        let result = Config::load(&config_path);
        assert!(result.is_err());

        // Recovery: use defaults and save
        let default_config = Config::default();
        default_config.save(&config_path)?;

        // Should now load successfully
        let recovered = Config::load(&config_path)?;
        assert_eq!(recovered.core.default_branch, "main");

        Ok(())
    }

    #[test]
    fn test_config_file_corruption() -> Result<()> {
        let dir = tempdir()?;
        let config_path = dir.path().join("corrupted.toml");

        // Create valid config first
        let config = Config::default();
        config.save(&config_path)?;

        std::fs::write(&config_path, vec![0xFF, 0xFE, 0xFD, 0xFC, 0xFB, 0xFA])?;
        let result = Config::load(&config_path);
        assert!(result.is_err(), "Binary data should fail UTF-8 validation");

        let invalid_utf8 = vec![
            0xC0, 0x80, // Overlong encoding of NUL
            0xF5, 0x80, 0x80, 0x80, // Out of range
        ];
        std::fs::write(&config_path, invalid_utf8)?;
        let result = Config::load(&config_path);
        assert!(result.is_err(), "Invalid UTF-8 should fail");

        // Empty file should succeed with defaults due to serde(default)
        std::fs::write(&config_path, "")?;
        let result = Config::load(&config_path);
        assert!(result.is_ok(), "Empty file should parse with defaults");

        Ok(())
    }

    #[test]
    fn test_branch_tracking_helpers() {
        let mut config = Config::default();

        // Test getting non-existent tracking
        assert!(config.get_branch_tracking("main").is_none());

        // Test setting branch tracking
        let tracking = BranchTracking {
            remote: "origin".to_string(),
            branch: "main".to_string(),
        };
        config.set_branch_tracking("main".to_string(), tracking);

        // Test getting existing tracking
        let retrieved = config.get_branch_tracking("main");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().remote, "origin");
        assert_eq!(retrieved.unwrap().branch, "main");

        // Test removing tracking
        let removed = config.remove_branch_tracking("main");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().remote, "origin");

        // Verify it's gone
        assert!(config.get_branch_tracking("main").is_none());

        // Test removing non-existent tracking
        assert!(config.remove_branch_tracking("nonexistent").is_none());
    }

    #[test]
    fn test_config_arbitrary_values() -> Result<()> {
        let dir = tempdir()?;

        // Test various config values within valid ranges
        let long_branch = "a".repeat(100);
        let test_cases = vec![
            ("main", 1, 1, 100),
            ("develop", 22, 64, 10000),
            ("feature-branch", 10, 8, 500),
            (long_branch.as_str(), 15, 32, 5000),
        ];

        for (branch_name, compression_level, parallel_threads, cache_size) in test_cases {
            let config_path = dir.path().join(format!("config_{compression_level}.toml"));

            let config_content = format!(
                r#"
                [core]
                default_branch = "{branch_name}"
                compression_level = {compression_level}

                [performance]
                parallel_threads = {parallel_threads}
                cache_size = {cache_size}
                "#
            );

            std::fs::write(&config_path, config_content)?;

            let loaded = Config::load(&config_path)?;

            // Values should be preserved correctly
            assert_eq!(loaded.core.default_branch, branch_name);
            assert_eq!(loaded.core.compression_level, compression_level);
            assert_eq!(loaded.performance.parallel_threads, parallel_threads);
            assert_eq!(loaded.performance.cache_size, cache_size);

            // Verify ranges are respected
            assert!(loaded.core.compression_level >= 1 && loaded.core.compression_level <= 22);
            assert!(loaded.performance.parallel_threads >= 1);
            assert!(!loaded.core.default_branch.is_empty());
        }

        // Test invalid compression levels
        let invalid_config = r"
            [core]
            compression_level = 100
            "
        .to_string();

        let invalid_path = dir.path().join("invalid.toml");
        std::fs::write(&invalid_path, invalid_config)?;

        // Should load but compression level might be clamped or defaulted
        let result = Config::load(&invalid_path);
        if let Ok(config) = result {
            assert!(
                config.core.compression_level <= 22,
                "Compression level should be clamped to valid range"
            );
        }

        Ok(())
    }
}
