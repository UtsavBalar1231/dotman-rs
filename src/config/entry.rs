use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

use crate::core::error::{DotmanError, Result};

/// Individual configuration entry for a file or directory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigEntry {
    /// Unique identifier for this entry
    pub id: String,
    /// Display name for this configuration
    pub name: String,
    /// Description of what this configuration manages
    pub description: Option<String>,
    /// Source path (where the file actually lives)
    pub source_path: PathBuf,
    /// Target path (where it should be symlinked/copied to)
    pub target_path: PathBuf,
    /// Whether this entry is currently active
    pub enabled: bool,
    /// Entry type (file, directory, symlink)
    pub entry_type: EntryType,
    /// Backup strategy for this entry
    pub backup_strategy: BackupStrategy,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Dependencies (other entries that must be processed first)
    pub dependencies: Vec<String>,
    /// Conflicts (entries that cannot be active at the same time)
    pub conflicts: Vec<String>,
    /// Platform-specific settings
    pub platform_specific: Vec<PlatformSpecific>,
    /// Last backup timestamp
    pub last_backup: Option<DateTime<Utc>>,
    /// Last restore timestamp
    pub last_restore: Option<DateTime<Utc>>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last modified timestamp
    pub modified_at: DateTime<Utc>,
}

/// Type of configuration entry
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryType {
    /// Single file
    File,
    /// Directory and its contents
    Directory,
    /// Symbolic link
    Symlink,
    /// Template file (processed before deployment)
    Template,
}

/// Backup strategy for an entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackupStrategy {
    /// Create full copies
    Copy,
    /// Create symbolic links
    Symlink,
    /// Use hard links where possible
    HardLink,
    /// Skip backup for this entry
    Skip,
}

/// Platform-specific configuration overrides
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformSpecific {
    /// Target platform
    pub platform: Platform,
    /// Platform-specific source path override
    pub source_path: Option<PathBuf>,
    /// Platform-specific target path override
    pub target_path: Option<PathBuf>,
    /// Whether this entry is enabled on this platform
    pub enabled: bool,
}

/// Supported platforms
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Platform {
    Linux,
    MacOS,
    Windows,
    Unix,
    Any,
}

impl ConfigEntry {
    /// Create a new configuration entry
    pub fn new(id: String, name: String, source_path: PathBuf, target_path: PathBuf) -> Self {
        let now = Utc::now();
        Self {
            id,
            name,
            description: None,
            source_path,
            target_path,
            enabled: true,
            entry_type: EntryType::File,
            backup_strategy: BackupStrategy::Copy,
            tags: Vec::new(),
            dependencies: Vec::new(),
            conflicts: Vec::new(),
            platform_specific: Vec::new(),
            last_backup: None,
            last_restore: None,
            created_at: now,
            modified_at: now,
        }
    }

    /// Add a tag to this entry
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            self.modified_at = Utc::now();
        }
    }

    /// Remove a tag from this entry
    pub fn remove_tag(&mut self, tag: &str) {
        if let Some(pos) = self.tags.iter().position(|t| t == tag) {
            self.tags.remove(pos);
            self.modified_at = Utc::now();
        }
    }

    /// Add a dependency
    pub fn add_dependency(&mut self, dependency_id: String) {
        if !self.dependencies.contains(&dependency_id) {
            self.dependencies.push(dependency_id);
            self.modified_at = Utc::now();
        }
    }

    /// Remove a dependency
    pub fn remove_dependency(&mut self, dependency_id: &str) {
        if let Some(pos) = self.dependencies.iter().position(|d| d == dependency_id) {
            self.dependencies.remove(pos);
            self.modified_at = Utc::now();
        }
    }

    /// Add a conflict
    pub fn add_conflict(&mut self, conflict_id: String) {
        if !self.conflicts.contains(&conflict_id) {
            self.conflicts.push(conflict_id);
            self.modified_at = Utc::now();
        }
    }

    /// Check if this entry is compatible with the current platform
    pub fn is_compatible_with_platform(&self, current_platform: &Platform) -> bool {
        // If no platform-specific settings, assume compatible
        if self.platform_specific.is_empty() {
            return true;
        }

        // Check if there's a specific setting for the current platform
        for platform_config in &self.platform_specific {
            if platform_config.platform == *current_platform || platform_config.platform == Platform::Any {
                return platform_config.enabled;
            }
        }

        // If platform isn't explicitly mentioned, check for Unix compatibility
        if *current_platform == Platform::Linux || *current_platform == Platform::MacOS {
            for platform_config in &self.platform_specific {
                if platform_config.platform == Platform::Unix {
                    return platform_config.enabled;
                }
            }
        }

        false
    }

    /// Get effective source path for the current platform
    pub fn get_effective_source_path(&self, current_platform: &Platform) -> PathBuf {
        for platform_config in &self.platform_specific {
            if (platform_config.platform == *current_platform || platform_config.platform == Platform::Any)
                && platform_config.source_path.is_some() {
                return platform_config.source_path.as_ref().unwrap().clone();
            }
        }
        self.source_path.clone()
    }

    /// Get effective target path for the current platform
    pub fn get_effective_target_path(&self, current_platform: &Platform) -> PathBuf {
        for platform_config in &self.platform_specific {
            if (platform_config.platform == *current_platform || platform_config.platform == Platform::Any)
                && platform_config.target_path.is_some() {
                return platform_config.target_path.as_ref().unwrap().clone();
            }
        }
        self.target_path.clone()
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self.modified_at = Utc::now();
        self
    }

    /// Set entry type
    pub fn with_type(mut self, entry_type: EntryType) -> Self {
        self.entry_type = entry_type;
        self.modified_at = Utc::now();
        self
    }

    /// Set backup strategy
    pub fn with_backup_strategy(mut self, strategy: BackupStrategy) -> Self {
        self.backup_strategy = strategy;
        self.modified_at = Utc::now();
        self
    }

    /// Enable or disable this entry
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.modified_at = Utc::now();
    }

    /// Mark as backed up
    pub fn mark_backed_up(&mut self) {
        self.last_backup = Some(Utc::now());
        self.modified_at = Utc::now();
    }

    /// Mark as restored
    pub fn mark_restored(&mut self) {
        self.last_restore = Some(Utc::now());
        self.modified_at = Utc::now();
    }

    /// Validate the entry configuration
    pub fn validate(&self) -> Result<()> {
        if self.id.is_empty() {
            return Err(DotmanError::config("Entry ID cannot be empty".to_string()));
        }

        if self.name.is_empty() {
            return Err(DotmanError::config("Entry name cannot be empty".to_string()));
        }

        if self.source_path.as_os_str().is_empty() {
            return Err(DotmanError::config("Source path cannot be empty".to_string()));
        }

        if self.target_path.as_os_str().is_empty() {
            return Err(DotmanError::config("Target path cannot be empty".to_string()));
        }

        // Check for self-references in dependencies
        if self.dependencies.contains(&self.id) {
            return Err(DotmanError::config("Entry cannot depend on itself".to_string()));
        }

        // Check for self-references in conflicts
        if self.conflicts.contains(&self.id) {
            return Err(DotmanError::config("Entry cannot conflict with itself".to_string()));
        }

        Ok(())
    }
}

impl Platform {
    /// Get the current platform
    pub fn current() -> Self {
        #[cfg(target_os = "linux")]
        return Platform::Linux;
        
        #[cfg(target_os = "macos")]
        return Platform::MacOS;
        
        #[cfg(target_os = "windows")]
        return Platform::Windows;
        
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        return Platform::Unix;
    }

    /// Check if the platform is Unix-like
    pub fn is_unix_like(&self) -> bool {
        matches!(self, Platform::Linux | Platform::MacOS | Platform::Unix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_entry_creation() {
        let entry = ConfigEntry::new(
            "test".to_string(),
            "Test Entry".to_string(),
            PathBuf::from("/home/user/.bashrc"),
            PathBuf::from("/dotfiles/bashrc"),
        );

        assert_eq!(entry.id, "test");
        assert_eq!(entry.name, "Test Entry");
        assert!(entry.enabled);
        assert_eq!(entry.entry_type, EntryType::File);
        assert!(entry.tags.is_empty());
    }

    #[test]
    fn test_tag_management() {
        let mut entry = ConfigEntry::new(
            "test".to_string(),
            "Test".to_string(),
            PathBuf::from("/source"),
            PathBuf::from("/target"),
        );

        entry.add_tag("shell".to_string());
        entry.add_tag("config".to_string());
        assert_eq!(entry.tags.len(), 2);
        assert!(entry.tags.contains(&"shell".to_string()));

        // Adding duplicate tag should not increase count
        entry.add_tag("shell".to_string());
        assert_eq!(entry.tags.len(), 2);

        entry.remove_tag("shell");
        assert_eq!(entry.tags.len(), 1);
        assert!(!entry.tags.contains(&"shell".to_string()));
    }

    #[test]
    fn test_platform_compatibility() {
        let mut entry = ConfigEntry::new(
            "test".to_string(),
            "Test".to_string(),
            PathBuf::from("/source"),
            PathBuf::from("/target"),
        );

        // No platform-specific settings - should be compatible with all
        assert!(entry.is_compatible_with_platform(&Platform::Linux));
        assert!(entry.is_compatible_with_platform(&Platform::MacOS));
        assert!(entry.is_compatible_with_platform(&Platform::Windows));

        // Add Linux-specific setting
        entry.platform_specific.push(PlatformSpecific {
            platform: Platform::Linux,
            source_path: None,
            target_path: None,
            enabled: true,
        });

        assert!(entry.is_compatible_with_platform(&Platform::Linux));
        assert!(!entry.is_compatible_with_platform(&Platform::Windows));
    }

    #[test]
    fn test_entry_validation() {
        let mut entry = ConfigEntry::new(
            "test".to_string(),
            "Test".to_string(),
            PathBuf::from("/source"),
            PathBuf::from("/target"),
        );

        assert!(entry.validate().is_ok());

        // Test self-dependency
        entry.add_dependency("test".to_string());
        assert!(entry.validate().is_err());

        entry.remove_dependency("test");
        assert!(entry.validate().is_ok());

        // Test self-conflict
        entry.add_conflict("test".to_string());
        assert!(entry.validate().is_err());
    }

    #[test]
    fn test_current_platform() {
        let platform = Platform::current();
        // Just ensure we get a valid platform
        assert!(matches!(platform, Platform::Linux | Platform::MacOS | Platform::Windows | Platform::Unix));
    }
} 