use std::path::PathBuf;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

use crate::core::error::{DotmanError, Result};
use super::config::Config;

/// Configuration profile for different environments/use cases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// Profile name
    pub name: String,
    /// Profile description
    pub description: Option<String>,
    /// Base configuration
    pub config: Config,
    /// Environment-specific overrides
    pub environment_overrides: HashMap<String, ConfigOverride>,
    /// Whether this profile is active
    pub is_active: bool,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last modified timestamp
    pub modified_at: chrono::DateTime<chrono::Utc>,
}

/// Configuration overrides for specific environments
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigOverride {
    /// Override backup directory
    pub backup_dir: Option<PathBuf>,
    /// Override include patterns
    pub include_patterns: Option<Vec<String>>,
    /// Override exclude patterns
    pub exclude_patterns: Option<Vec<String>>,
    /// Override symlink following
    pub follow_symlinks: Option<bool>,
    /// Override permission preservation
    pub preserve_permissions: Option<bool>,
    /// Override backup creation
    pub create_backups: Option<bool>,
    /// Override integrity verification
    pub verify_integrity: Option<bool>,
    /// Override max versions
    pub max_versions: Option<Option<u32>>,
    /// Override max backup versions
    pub max_backup_versions: Option<Option<u32>>,
}

/// Profile manager for handling multiple configuration profiles
pub struct ProfileManager {
    /// Available profiles
    profiles: HashMap<String, Profile>,
    /// Currently active profile name
    active_profile: Option<String>,
    /// Profiles directory
    profiles_dir: PathBuf,
}

impl Profile {
    /// Create a new profile
    pub fn new(name: String, config: Config) -> Self {
        let now = chrono::Utc::now();
        Self {
            name: name.clone(),
            description: None,
            config,
            environment_overrides: HashMap::new(),
            is_active: false,
            created_at: now,
            modified_at: now,
        }
    }

    /// Set profile description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self.modified_at = chrono::Utc::now();
        self
    }

    /// Add environment override
    pub fn add_environment_override(&mut self, environment: String, override_config: ConfigOverride) {
        self.environment_overrides.insert(environment, override_config);
        self.modified_at = chrono::Utc::now();
    }

    /// Get effective configuration for an environment
    pub fn get_effective_config(&self, environment: Option<&str>) -> Config {
        let mut effective_config = self.config.clone();
        
        if let Some(env) = environment {
            if let Some(override_config) = self.environment_overrides.get(env) {
                self.apply_override(&mut effective_config, override_config);
            }
        }
        
        effective_config
    }

    /// Apply configuration override
    fn apply_override(&self, config: &mut Config, override_config: &ConfigOverride) {
        if let Some(backup_dir) = &override_config.backup_dir {
            config.backup_dir = backup_dir.clone();
        }
        if let Some(include_patterns) = &override_config.include_patterns {
            config.include_patterns = include_patterns.clone();
        }
        if let Some(exclude_patterns) = &override_config.exclude_patterns {
            config.exclude_patterns = exclude_patterns.clone();
        }
        if let Some(follow_symlinks) = override_config.follow_symlinks {
            config.follow_symlinks = follow_symlinks;
        }
        if let Some(preserve_permissions) = override_config.preserve_permissions {
            config.preserve_permissions = preserve_permissions;
        }
        if let Some(create_backups) = override_config.create_backups {
            config.create_backups = create_backups;
        }
        if let Some(verify_integrity) = override_config.verify_integrity {
            config.verify_integrity = verify_integrity;
        }
        if let Some(max_versions) = override_config.max_backup_versions {
            config.max_backup_versions = max_versions.unwrap_or(config.max_backup_versions);
        }
    }

    /// Activate this profile
    pub fn activate(&mut self) {
        self.is_active = true;
        self.modified_at = chrono::Utc::now();
    }

    /// Deactivate this profile
    pub fn deactivate(&mut self) {
        self.is_active = false;
        self.modified_at = chrono::Utc::now();
    }
}

impl ProfileManager {
    /// Create a new profile manager
    pub fn new(profiles_dir: PathBuf) -> Self {
        Self {
            profiles: HashMap::new(),
            active_profile: None,
            profiles_dir,
        }
    }

    /// Load profiles from disk
    pub async fn load_profiles(&mut self) -> Result<()> {
        if !self.profiles_dir.exists() {
            tokio::fs::create_dir_all(&self.profiles_dir).await
                .map_err(|e| DotmanError::config(format!("Failed to create profiles directory: {}", e)))?;
            return Ok(());
        }

        let mut entries = tokio::fs::read_dir(&self.profiles_dir).await
            .map_err(|e| DotmanError::config(format!("Failed to read profiles directory: {}", e)))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| DotmanError::config(format!("Failed to read directory entry: {}", e)))? {
            
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("toml") {
                match self.load_profile_from_file(&path).await {
                    Ok(profile) => {
                        self.profiles.insert(profile.name.clone(), profile);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load profile from {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Load a single profile from file
    async fn load_profile_from_file(&self, path: &PathBuf) -> Result<Profile> {
        let content = tokio::fs::read_to_string(path).await
            .map_err(|e| DotmanError::config(format!("Failed to read profile file: {}", e)))?;
        
        let profile: Profile = toml::from_str(&content)
            .map_err(|e| DotmanError::config(format!("Failed to parse profile file: {}", e)))?;
        
        Ok(profile)
    }

    /// Save all profiles to disk
    pub async fn save_profiles(&self) -> Result<()> {
        tokio::fs::create_dir_all(&self.profiles_dir).await
            .map_err(|e| DotmanError::config(format!("Failed to create profiles directory: {}", e)))?;

        for profile in self.profiles.values() {
            self.save_profile(profile).await?;
        }

        Ok(())
    }

    /// Save a single profile to disk
    async fn save_profile(&self, profile: &Profile) -> Result<()> {
        let filename = format!("{}.toml", profile.name);
        let path = self.profiles_dir.join(filename);
        
        let content = toml::to_string_pretty(profile)
            .map_err(|e| DotmanError::config(format!("Failed to serialize profile: {}", e)))?;
        
        tokio::fs::write(path, content).await
            .map_err(|e| DotmanError::config(format!("Failed to write profile file: {}", e)))
    }

    /// Add a new profile
    pub async fn add_profile(&mut self, profile: Profile) -> Result<()> {
        let name = profile.name.clone();
        self.profiles.insert(name.clone(), profile);
        self.save_profile(&self.profiles[&name]).await
    }

    /// Remove a profile
    pub async fn remove_profile(&mut self, name: &str) -> Result<()> {
        if self.active_profile.as_ref() == Some(&name.to_string()) {
            self.active_profile = None;
        }
        
        self.profiles.remove(name);
        
        let filename = format!("{}.toml", name);
        let path = self.profiles_dir.join(filename);
        
        if path.exists() {
            tokio::fs::remove_file(path).await
                .map_err(|e| DotmanError::config(format!("Failed to remove profile file: {}", e)))?;
        }
        
        Ok(())
    }

    /// Set active profile
    pub async fn set_active_profile(&mut self, name: &str) -> Result<()> {
        if !self.profiles.contains_key(name) {
            return Err(DotmanError::config(format!("Profile '{}' does not exist", name)));
        }

        // Deactivate current active profile
        if let Some(current_active) = &self.active_profile {
            if let Some(profile) = self.profiles.get_mut(current_active) {
                profile.deactivate();
            }
        }

        // Activate new profile
        if let Some(profile) = self.profiles.get_mut(name) {
            profile.activate();
        }

        self.active_profile = Some(name.to_string());
        self.save_profiles().await
    }

    /// Get active profile
    pub fn get_active_profile(&self) -> Option<&Profile> {
        self.active_profile.as_ref()
            .and_then(|name| self.profiles.get(name))
    }

    /// Get profile by name
    pub fn get_profile(&self, name: &str) -> Option<&Profile> {
        self.profiles.get(name)
    }

    /// List all profile names
    pub fn list_profiles(&self) -> Vec<String> {
        self.profiles.keys().cloned().collect()
    }

    /// Rename a profile
    pub async fn rename_profile(&mut self, old_name: &str, new_name: &str) -> Result<()> {
        // Check if old profile exists
        if !self.profiles.contains_key(old_name) {
            return Err(DotmanError::config(format!("Profile '{}' does not exist", old_name)));
        }

        // Check if new name already exists
        if self.profiles.contains_key(new_name) {
            return Err(DotmanError::config(format!("Profile '{}' already exists", new_name)));
        }

        // Remove old profile
        let mut profile = self.profiles.remove(old_name).unwrap();
        
        // Update profile name
        profile.name = new_name.to_string();
        profile.modified_at = chrono::Utc::now();

        // Update active profile if needed
        if self.active_profile.as_ref() == Some(&old_name.to_string()) {
            self.active_profile = Some(new_name.to_string());
        }

        // Add profile with new name
        self.profiles.insert(new_name.to_string(), profile);

        // Remove old file
        let old_filename = format!("{}.toml", old_name);
        let old_path = self.profiles_dir.join(old_filename);
        if old_path.exists() {
            tokio::fs::remove_file(old_path).await
                .map_err(|e| DotmanError::config(format!("Failed to remove old profile file: {}", e)))?;
        }

        // Save new profile
        self.save_profile(&self.profiles[new_name]).await
    }

    /// Create default profiles
    pub async fn create_default_profiles(&mut self) -> Result<()> {
        // Development profile
        let mut dev_config = Config::default();
        dev_config.verify_integrity = false; // Faster for development
        dev_config.max_backup_versions = 3;
        
        let dev_profile = Profile::new("development".to_string(), dev_config)
            .with_description("Development environment profile with faster operations".to_string());
        
        // Production profile
        let mut prod_config = Config::default();
        prod_config.verify_integrity = true;
        prod_config.max_backup_versions = 10;
        prod_config.compression.enabled = true;
        
        let prod_profile = Profile::new("production".to_string(), prod_config)
            .with_description("Production environment profile with full verification".to_string());
        
        // Server profile
        let mut server_config = Config::default();
        server_config.include_patterns = vec![
            "/etc/*".to_string(),
            "/home/*/.bashrc".to_string(),
            "/home/*/.vimrc".to_string(),
        ];
        server_config.max_backup_versions = 20;
        
        let server_profile = Profile::new("server".to_string(), server_config)
            .with_description("Server configuration profile for system files".to_string());

        self.add_profile(dev_profile).await?;
        self.add_profile(prod_profile).await?;
        self.add_profile(server_profile).await?;

        // Set development as default active profile
        self.set_active_profile("development").await?;

        Ok(())
    }

    /// Create a new profile with the given name and configuration
    pub async fn create_profile(&mut self, name: String, profile: Profile) -> Result<()> {
        if self.profiles.contains_key(&name) {
            return Err(DotmanError::config(format!("Profile '{}' already exists", name)));
        }
        
        self.profiles.insert(name.clone(), profile);
        self.save_profiles().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_profile_creation() {
        let config = Config::default();
        let profile = Profile::new("test".to_string(), config);
        
        assert_eq!(profile.name, "test");
        assert!(!profile.is_active);
        assert!(profile.environment_overrides.is_empty());
    }

    #[test]
    fn test_profile_environment_override() {
        let config = Config::default();
        let mut profile = Profile::new("test".to_string(), config);
        
        let override_config = ConfigOverride {
            backup_dir: Some(PathBuf::from("/tmp/backup")),
            include_patterns: None,
            exclude_patterns: None,
            follow_symlinks: Some(false),
            preserve_permissions: None,
            create_backups: None,
            verify_integrity: None,
            max_versions: None,
            max_backup_versions: None,
        };
        
        profile.add_environment_override("testing".to_string(), override_config);
        
        let effective_config = profile.get_effective_config(Some("testing"));
        assert_eq!(effective_config.backup_dir, PathBuf::from("/tmp/backup"));
        assert!(!effective_config.follow_symlinks);
        
        // Test without environment
        let base_config = profile.get_effective_config(None);
        assert_ne!(base_config.backup_dir, PathBuf::from("/tmp/backup"));
    }

    #[tokio::test]
    async fn test_profile_manager() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = ProfileManager::new(temp_dir.path().to_path_buf());
        
        // Create and add a profile
        let config = Config::default();
        let profile = Profile::new("test-profile".to_string(), config);
        
        manager.add_profile(profile).await.unwrap();
        assert!(manager.get_profile("test-profile").is_some());
        
        // Set as active
        manager.set_active_profile("test-profile").await.unwrap();
        assert!(manager.get_active_profile().is_some());
        assert_eq!(manager.get_active_profile().unwrap().name, "test-profile");
        
        // Test persistence
        let mut new_manager = ProfileManager::new(temp_dir.path().to_path_buf());
        new_manager.load_profiles().await.unwrap();
        assert!(new_manager.get_profile("test-profile").is_some());
    }
} 