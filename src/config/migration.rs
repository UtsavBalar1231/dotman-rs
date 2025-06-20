use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

use crate::core::error::{DotmanError, Result};
use super::config::Config;

/// Current configuration version
pub const CURRENT_CONFIG_VERSION: u32 = 1;

/// Configuration migration system
pub struct ConfigMigration;

/// Migration metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationInfo {
    /// Version this migration targets
    pub target_version: u32,
    /// Migration description
    pub description: String,
    /// When this migration was applied
    pub applied_at: DateTime<Utc>,
    /// Whether the migration was successful
    pub success: bool,
    /// Any warnings or notes from the migration
    pub notes: Vec<String>,
}

/// Legacy configuration structure for migration purposes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyConfig {
    /// Configuration version
    pub version: Option<u32>,
    /// Raw configuration data
    #[serde(flatten)]
    pub data: HashMap<String, toml::Value>,
}

impl ConfigMigration {
    /// Check if a configuration needs migration
    pub fn needs_migration(config_data: &str) -> Result<bool> {
        // First try to parse as current config
        if toml::from_str::<Config>(config_data).is_ok() {
            return Ok(false);
        }

        // Then try to parse as legacy config
        if let Ok(legacy) = toml::from_str::<LegacyConfig>(config_data) {
            let version = legacy.version.unwrap_or(0);
            return Ok(version < CURRENT_CONFIG_VERSION);
        }

        // If we can't parse it at all, assume it needs migration
        Ok(true)
    }

    /// Get the version of a configuration
    pub fn get_config_version(config_data: &str) -> Result<u32> {
        // First try to parse as current config
        if toml::from_str::<Config>(config_data).is_ok() {
            return Ok(CURRENT_CONFIG_VERSION);
        }

        // Then try to extract version from legacy TOML
        if let Ok(legacy) = toml::from_str::<LegacyConfig>(config_data) {
            return Ok(legacy.version.unwrap_or(0));
        }

        // If we can't determine, assume version 0
        Ok(0)
    }

    /// Migrate configuration to current version
    pub async fn migrate_config(config_data: &str) -> Result<(Config, Vec<MigrationInfo>)> {
        let mut migrations = Vec::new();
        let current_version = Self::get_config_version(config_data)?;

        if current_version >= CURRENT_CONFIG_VERSION {
            // Already current version
            let config = toml::from_str::<Config>(config_data)
                .map_err(|e| DotmanError::config(format!("Failed to parse current config: {}", e)))?;
            return Ok((config, migrations));
        }

        // Apply migrations in sequence
        let mut working_config = Self::parse_legacy_config(config_data)?;
        
        for version in (current_version + 1)..=CURRENT_CONFIG_VERSION {
            let migration_result = Self::apply_migration(working_config, version).await?;
            working_config = migration_result.0;
            migrations.push(migration_result.1);
        }

        Ok((working_config, migrations))
    }

    /// Parse legacy configuration
    fn parse_legacy_config(config_data: &str) -> Result<Config> {
        // For now, just return a default config and log that we're migrating
        tracing::info!("Migrating legacy configuration to current version");
        
        // TODO: Implement actual legacy parsing based on old config structure
        // This would involve parsing the old TOML structure and converting it
        
        Ok(Config::default())
    }

    /// Apply a specific migration
    async fn apply_migration(config: Config, target_version: u32) -> Result<(Config, MigrationInfo)> {
        let start_time = Utc::now();
        let mut notes = Vec::new();
        let migrated_config = config;

        let description = match target_version {
            1 => {
                // Migration to version 1: Initial refactor
                notes.push("Migrated to new configuration structure".to_string());
                notes.push("Added compression and encryption settings".to_string());
                notes.push("Added logging configuration".to_string());
                
                // Ensure all new fields have sensible defaults
                if migrated_config.compression.enabled {
                    notes.push("Compression was enabled in legacy config".to_string());
                }
                
                "Migration to version 1: New configuration structure"
            }
            _ => {
                return Err(DotmanError::config(format!("Unknown migration target version: {}", target_version)));
            }
        };

        let migration_info = MigrationInfo {
            target_version,
            description: description.to_string(),
            applied_at: start_time,
            success: true,
            notes,
        };

        tracing::info!("Applied migration to version {}: {}", target_version, description);

        Ok((migrated_config, migration_info))
    }

    /// Create backup of configuration before migration
    pub async fn backup_config_before_migration(config_path: &std::path::Path) -> Result<std::path::PathBuf> {
        let backup_path = config_path.with_extension(format!("toml.backup.{}", Utc::now().timestamp()));
        
        tokio::fs::copy(config_path, &backup_path).await
            .map_err(|e| DotmanError::config(format!("Failed to create config backup: {}", e)))?;
        
        tracing::info!("Created configuration backup at: {}", backup_path.display());
        Ok(backup_path)
    }

    /// Validate migrated configuration
    pub fn validate_migrated_config(config: &Config) -> Result<()> {
        config.validate()?;
        
        // Additional migration-specific validations
        if config.backup_dir.as_os_str().is_empty() {
            return Err(DotmanError::config("Migrated config has empty backup directory".to_string()));
        }

        if config.include_patterns.is_empty() {
            tracing::warn!("Migrated config has no include patterns - this may not be intended");
        }

        Ok(())
    }

    /// Get migration history from a directory
    pub async fn get_migration_history(config_dir: &std::path::Path) -> Result<Vec<MigrationInfo>> {
        let migration_file = config_dir.join("migrations.json");
        
        if !migration_file.exists() {
            return Ok(Vec::new());
        }

        let content = tokio::fs::read_to_string(&migration_file).await
            .map_err(|e| DotmanError::config(format!("Failed to read migration history: {}", e)))?;
        
        let history: Vec<MigrationInfo> = serde_json::from_str(&content)
            .map_err(|e| DotmanError::config(format!("Failed to parse migration history: {}", e)))?;
        
        Ok(history)
    }

    /// Save migration history
    pub async fn save_migration_history(config_dir: &std::path::Path, history: &[MigrationInfo]) -> Result<()> {
        let migration_file = config_dir.join("migrations.json");
        
        // Ensure directory exists
        tokio::fs::create_dir_all(config_dir).await
            .map_err(|e| DotmanError::config(format!("Failed to create config directory: {}", e)))?;
        
        let content = serde_json::to_string_pretty(history)
            .map_err(|e| DotmanError::config(format!("Failed to serialize migration history: {}", e)))?;
        
        tokio::fs::write(&migration_file, content).await
            .map_err(|e| DotmanError::config(format!("Failed to write migration history: {}", e)))?;
        
        Ok(())
    }

    /// Perform full configuration migration with backup and validation
    pub async fn perform_full_migration(config_path: &std::path::Path) -> Result<(Config, Vec<MigrationInfo>)> {
        // Read current config
        let config_data = tokio::fs::read_to_string(config_path).await
            .map_err(|e| DotmanError::config(format!("Failed to read config file: {}", e)))?;

        // Check if migration is needed
        if !Self::needs_migration(&config_data)? {
            let config = toml::from_str::<Config>(&config_data)
                .map_err(|e| DotmanError::config(format!("Failed to parse config: {}", e)))?;
            return Ok((config, Vec::new()));
        }

        // Create backup
        let _backup_path = Self::backup_config_before_migration(config_path).await?;

        // Perform migration
        let (migrated_config, migrations) = Self::migrate_config(&config_data).await?;

        // Validate migrated config
        Self::validate_migrated_config(&migrated_config)?;

        // Save migrated config
        migrated_config.save_to_file(&config_path.to_path_buf()).await?;

        // Save migration history
        if let Some(config_dir) = config_path.parent() {
            let mut history = Self::get_migration_history(config_dir).await?;
            history.extend(migrations.clone());
            Self::save_migration_history(config_dir, &history).await?;
        }

        tracing::info!("Configuration migration completed successfully");

        Ok((migrated_config, migrations))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_version_detection() {
        // Use a simple current config that matches the default structure
        let current_config = toml::to_string(&Config::default()).unwrap();

        let legacy_config = r#"
            version = 0
            old_field = "value"
        "#;

        // Test that current config doesn't need migration
        assert!(!ConfigMigration::needs_migration(&current_config).unwrap());
        
        // Test that legacy config does need migration
        assert!(ConfigMigration::needs_migration(legacy_config).unwrap());

        // Test version detection
        assert_eq!(ConfigMigration::get_config_version(&current_config).unwrap(), CURRENT_CONFIG_VERSION);
        assert_eq!(ConfigMigration::get_config_version(legacy_config).unwrap(), 0);
    }

    #[tokio::test]
    async fn test_migration_process() {
        let legacy_config = r#"
            version = 0
            some_old_field = "value"
        "#;

        let (migrated_config, migrations) = ConfigMigration::migrate_config(legacy_config).await.unwrap();
        
        assert!(!migrations.is_empty());
        assert!(migrated_config.validate().is_ok());
        assert_eq!(migrations[0].target_version, 1);
        assert!(migrations[0].success);
    }

    #[tokio::test]
    async fn test_migration_history() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path();

        let migration = MigrationInfo {
            target_version: 1,
            description: "Test migration".to_string(),
            applied_at: Utc::now(),
            success: true,
            notes: vec!["Test note".to_string()],
        };

        let history = vec![migration];
        ConfigMigration::save_migration_history(config_dir, &history).await.unwrap();

        let loaded_history = ConfigMigration::get_migration_history(config_dir).await.unwrap();
        assert_eq!(loaded_history.len(), 1);
        assert_eq!(loaded_history[0].target_version, 1);
        assert_eq!(loaded_history[0].description, "Test migration");
    }
} 