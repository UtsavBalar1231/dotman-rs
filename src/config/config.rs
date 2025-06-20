use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tokio::fs;
use crate::core::error::{DotmanError, Result};

/// Package configuration for organized backups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageConfig {
    /// Package name
    pub name: String,
    /// Description of what this package contains
    pub description: String,
    /// Paths to include in this package backup
    pub paths: Vec<PathBuf>,
    /// Package-specific exclude patterns
    pub exclude_patterns: Vec<String>,
    /// Package-specific include patterns
    pub include_patterns: Vec<String>,
}

impl PackageConfig {
    pub fn new(name: String, description: String, paths: Vec<PathBuf>) -> Self {
        Self {
            name,
            description,
            paths,
            exclude_patterns: Vec::new(),
            include_patterns: Vec::new(),
        }
    }
}

/// Compression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Whether compression is enabled
    pub enabled: bool,
    /// Compression level (0-9)
    pub level: u32,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            level: 6,
        }
    }
}

/// Main configuration structure for dotman-rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Backup directory path
    pub backup_dir: PathBuf,
    /// Configuration directory path
    pub config_dir: PathBuf,
    /// Include patterns (glob patterns)
    pub include_patterns: Vec<String>,
    /// Exclude patterns (glob patterns)
    pub exclude_patterns: Vec<String>,
    /// Whether to follow symlinks
    pub follow_symlinks: bool,
    /// Whether to preserve file permissions
    pub preserve_permissions: bool,
    /// Whether to create backups of existing files before restore
    pub create_backups: bool,
    /// Whether to verify file integrity after operations
    pub verify_integrity: bool,
    /// Operation mode for the current session
    pub operation_mode: crate::core::types::OperationMode,
    /// Maximum number of backup versions to keep
    pub max_backup_versions: u32,
    /// Whether to use compression for backups
    pub enable_compression: bool,
    /// Whether to enable encryption for sensitive files
    pub enable_encryption: bool,
    /// Logging configuration
    pub log_level: String,
    /// Custom backup patterns
    pub backup_patterns: HashMap<String, String>,
    /// Compression configuration
    pub compression: CompressionConfig,
    /// Package configurations for organized backups
    pub packages: HashMap<String, PackageConfig>,
}

impl Default for Config {
    fn default() -> Self {
        // Start with empty packages - users will define their own
        let packages = HashMap::new();

        Self {
            backup_dir: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".dotman/backups"),
            config_dir: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".dotman"),
            include_patterns: vec![
                "*".to_string(), // Include all files by default
            ],
            exclude_patterns: vec![
                ".git/*".to_string(),
                ".DS_Store".to_string(),
                "*.tmp".to_string(),
                "*.log".to_string(),
                "target/*".to_string(),  // Rust build artifacts
                "node_modules/*".to_string(),  // Node.js dependencies
                "*.pyc".to_string(),  // Python compiled files
                "__pycache__/*".to_string(),  // Python cache
            ],
            follow_symlinks: true,
            preserve_permissions: true,
            create_backups: true,
            verify_integrity: true,
            operation_mode: crate::core::types::OperationMode::Default,
            max_backup_versions: 5,
            enable_compression: false,
            enable_encryption: false,
            log_level: "info".to_string(),
            backup_patterns: HashMap::new(),
            compression: CompressionConfig::default(),
            packages,
        }
    }
}

impl Config {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from file
    pub async fn load_from_file(path: &PathBuf) -> Result<Self> {
        let content = fs::read_to_string(path).await
            .map_err(|e| DotmanError::config(format!("Failed to read config file: {}", e)))?;
        
        let config: Config = toml::from_str(&content)
            .map_err(|e| DotmanError::config(format!("Failed to parse config file: {}", e)))?;
        
        config.validate()?;
        Ok(config)
    }

    /// Save configuration to file
    pub async fn save_to_file(&self, path: &PathBuf) -> Result<()> {
        self.validate()?;
        
        let content = toml::to_string_pretty(self)
            .map_err(|e| DotmanError::config(format!("Failed to serialize config: {}", e)))?;
        
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await
                .map_err(|e| DotmanError::config(format!("Failed to create config directory: {}", e)))?;
        }
        
        fs::write(path, content).await
            .map_err(|e| DotmanError::config(format!("Failed to write config file: {}", e)))?;
        
        Ok(())
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate max versions
        if self.max_backup_versions == 0 {
            return Err(DotmanError::config("Maximum versions must be greater than 0"));
        }

        // Validate backup directory
        if !self.backup_dir.is_absolute() {
            return Err(DotmanError::config("Backup directory must be an absolute path"));
        }

        // Validate log level
        match self.log_level.to_lowercase().as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => {},
            _ => return Err(DotmanError::config("Invalid log level")),
        }

        // Validate compression configuration
        if self.compression.enabled && self.compression.level > 9 {
            return Err(DotmanError::config("Compression level must be between 0 and 9"));
        }

        Ok(())
    }

    /// Get the default configuration file path
    pub fn default_config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")))
            .join("dotman")
            .join("config.toml")
    }

    /// Get package configuration by name
    pub fn get_package(&self, name: &str) -> Option<&PackageConfig> {
        self.packages.get(name)
    }

    /// Add or update a package configuration
    pub fn set_package(&mut self, package: PackageConfig) {
        self.packages.insert(package.name.clone(), package);
    }

    /// Remove a package configuration
    pub fn remove_package(&mut self, name: &str) -> Option<PackageConfig> {
        self.packages.remove(name)
    }

    /// List all available package names
    pub fn list_packages(&self) -> Vec<&String> {
        self.packages.keys().collect()
    }

    /// Merge another configuration into this one, overriding fields
    pub fn merge_config(&mut self, other: Config) {
        self.backup_dir = other.backup_dir;
        self.config_dir = other.config_dir;
        self.include_patterns = other.include_patterns;
        self.exclude_patterns = other.exclude_patterns;
        self.follow_symlinks = other.follow_symlinks;
        self.preserve_permissions = other.preserve_permissions;
        self.create_backups = other.create_backups;
        self.verify_integrity = other.verify_integrity;
        self.operation_mode = other.operation_mode;
        self.max_backup_versions = other.max_backup_versions;
        self.enable_compression = other.enable_compression;
        self.enable_encryption = other.enable_encryption;
        self.log_level = other.log_level;
        self.backup_patterns = other.backup_patterns;
        self.compression = other.compression;
        self.packages = other.packages;
    }

    /// Merge configuration overrides into this configuration
    pub fn merge(&mut self, override_config: crate::config::profile::ConfigOverride) {
        if let Some(backup_dir) = override_config.backup_dir {
            self.backup_dir = backup_dir;
        }
        if let Some(include_patterns) = override_config.include_patterns {
            self.include_patterns = include_patterns;
        }
        if let Some(exclude_patterns) = override_config.exclude_patterns {
            self.exclude_patterns = exclude_patterns;
        }
        if let Some(follow_symlinks) = override_config.follow_symlinks {
            self.follow_symlinks = follow_symlinks;
        }
        if let Some(preserve_permissions) = override_config.preserve_permissions {
            self.preserve_permissions = preserve_permissions;
        }
        if let Some(create_backups) = override_config.create_backups {
            self.create_backups = create_backups;
        }
        if let Some(verify_integrity) = override_config.verify_integrity {
            self.verify_integrity = verify_integrity;
        }
        if let Some(max_backup_versions) = override_config.max_backup_versions {
            if let Some(versions) = max_backup_versions {
                self.max_backup_versions = versions;
            }
        }
    }

    /// Check if a path should be included based on patterns
    pub fn should_include(&self, path: &PathBuf) -> bool {
        let path_str = path.to_string_lossy();
        
        // Check exclude patterns first
        for pattern in &self.exclude_patterns {
            if glob_match(pattern, &path_str) {
                return false;
            }
        }
        
        // Check include patterns
        for pattern in &self.include_patterns {
            if glob_match(pattern, &path_str) {
                return true;
            }
        }
        
        false
    }

    /// Get the backup directory for a specific profile
    pub fn backup_dir_for_profile(&self, profile: &str) -> PathBuf {
        self.backup_dir.join(profile)
    }

    /// Get the configuration directory
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")))
            .join("dotman")
    }

    /// Save configuration to a file
    pub async fn save(&self, path: &Path) -> Result<()> {
        let toml_content = toml::to_string(self)
            .map_err(|e| DotmanError::config(format!("Failed to serialize config: {}", e)))?;
        
        // Ensure directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| DotmanError::filesystem(format!("Failed to create config directory: {}", e)))?;
        }
        
        tokio::fs::write(path, toml_content).await
            .map_err(|e| DotmanError::filesystem(format!("Failed to write config file: {}", e)))
    }

    /// Load configuration from a file
    pub async fn load(path: &Path) -> Result<Self> {
        let content = tokio::fs::read_to_string(path).await
            .map_err(|e| DotmanError::filesystem(format!("Failed to read config file: {}", e)))?;
        
        toml::from_str(&content)
            .map_err(|e| DotmanError::config(format!("Failed to parse config: {}", e)))
    }
}

/// Simple glob pattern matching
fn glob_match(pattern: &str, text: &str) -> bool {
    // Simple implementation - for production use, consider using the `glob` crate
    if pattern == "*" || pattern == ".*" {
        return true;
    }
    
    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            let (prefix, suffix) = (parts[0], parts[1]);
            return text.starts_with(prefix) && text.ends_with(suffix);
        }
    }
    
    text == pattern
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.backup_dir.ends_with(".dotman/backups"));
        assert!(!config.include_patterns.is_empty());
        assert!(!config.exclude_patterns.is_empty());
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        
        // Valid configuration should pass
        assert!(config.validate().is_ok());
        
        // Invalid compression level should fail
        config.compression.level = 15;
        config.compression.enabled = true;
        assert!(config.validate().is_err());
    }

    #[tokio::test]
    async fn test_config_save_load() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        let original_config = Config::default();
        original_config.save_to_file(&config_path).await.unwrap();
        
        let loaded_config = Config::load_from_file(&config_path).await.unwrap();
        assert_eq!(original_config.backup_dir, loaded_config.backup_dir);
    }

    #[test]
    fn test_should_include() {
        let config = Config::default();
        
        // Should include dotfiles
        assert!(config.should_include(&PathBuf::from(".bashrc")));
        
        // Should exclude .git files
        assert!(!config.should_include(&PathBuf::from(".git/config")));
    }

    #[test]
    fn test_glob_match() {
        assert!(glob_match("*.txt", "file.txt"));
        assert!(glob_match(".*", ".bashrc"));
        assert!(!glob_match("*.txt", "file.rs"));
        assert!(glob_match("exact", "exact"));
    }
} 