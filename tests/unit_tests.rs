use dotman_rs::config::{Config, profile::ConfigOverride};
use dotman_rs::config::config::PackageConfig;
use dotman_rs::core::types::*;
use dotman_rs::core::error::DotmanError;
use dotman_rs::core::traits::{FileSystem, ProgressReporter};
use dotman_rs::filesystem::FileSystemImpl;
use dotman_rs::backup::BackupSession;
use dotman_rs::cli::commands::CliProgressReporter;
use dotman_rs::cli::args::{DotmanArgs, Command, BackupArgs, RestoreArgs, PackageArgs, PackageAction};
use std::path::PathBuf;
use tempfile::TempDir;
use chrono::Utc;
use uuid::Uuid;
use std::collections::HashMap;
use tempfile::tempdir;

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        
        assert!(!config.include_patterns.is_empty());
        assert!(!config.exclude_patterns.is_empty());
        assert!(config.preserve_permissions);
        assert!(config.verify_integrity);
        assert_eq!(config.max_backup_versions, 5);
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        
        // Valid config should pass
        assert!(config.validate().is_ok());
        
        // Invalid max backup versions should fail
        config.max_backup_versions = 0;
        assert!(config.validate().is_err());
        
        // Reset and test relative backup directory should fail
        config = Config::default();
        config.backup_dir = PathBuf::from("relative/path");
        assert!(config.validate().is_err());
        
        // Reset and test invalid log level should fail
        config = Config::default();
        config.log_level = "invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_should_include() {
        let mut config = Config::default();
        config.include_patterns = vec!["*.txt".to_string(), "*.conf".to_string()];
        config.exclude_patterns = vec!["*.log".to_string(), "temp*".to_string()];

        // Should include matching patterns
        assert!(config.should_include(&PathBuf::from("test.txt")));
        assert!(config.should_include(&PathBuf::from("config.conf")));
        
        // Should exclude matching exclude patterns
        assert!(!config.should_include(&PathBuf::from("debug.log")));
        assert!(!config.should_include(&PathBuf::from("temp_file.txt")));
        
        // Should exclude non-matching include patterns
        assert!(!config.should_include(&PathBuf::from("readme.md")));
    }

    #[test]
    fn test_config_merge() {
        let mut base_config = Config::default();
        base_config.max_backup_versions = 10;
        base_config.verify_integrity = false;
        
        let override_config = ConfigOverride {
            max_backup_versions: Some(Some(15)),
            verify_integrity: Some(true),
            preserve_permissions: None,
            ..Default::default()
        };
        
        base_config.merge(override_config);
        
        assert_eq!(base_config.max_backup_versions, 15);
        assert!(base_config.verify_integrity);
        // preserve_permissions should remain unchanged
        assert!(base_config.preserve_permissions);
    }

    #[tokio::test]
    async fn test_config_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        
        let mut original_config = Config::default();
        original_config.max_backup_versions = 42;
        original_config.log_level = "debug".to_string();
        
        // Save config
        original_config.save(&config_path).await.unwrap();
        assert!(config_path.exists());
        
        // Load config
        let loaded_config = Config::load(&config_path).await.unwrap();
        assert_eq!(loaded_config.max_backup_versions, 42);
        assert_eq!(loaded_config.log_level, "debug");
    }

    #[test]
    fn test_package_config_creation() {
        let package = PackageConfig::new(
            "nvim".to_string(),
            "Neovim configuration".to_string(),
            vec![PathBuf::from("~/.config/nvim")],
        );

        assert_eq!(package.name, "nvim");
        assert_eq!(package.description, "Neovim configuration");
        assert_eq!(package.paths.len(), 1);
        assert!(package.exclude_patterns.is_empty());
        assert!(package.include_patterns.is_empty());
    }

    #[test]
    fn test_package_config_with_patterns() {
        let mut package = PackageConfig::new(
            "nvim".to_string(),
            "Neovim configuration".to_string(),
            vec![PathBuf::from("~/.config/nvim")],
        );
        
        package.exclude_patterns = vec!["*.log".to_string(), "cache/*".to_string()];
        package.include_patterns = vec!["*.lua".to_string(), "*.vim".to_string()];

        assert_eq!(package.exclude_patterns.len(), 2);
        assert_eq!(package.include_patterns.len(), 2);
        assert!(package.exclude_patterns.contains(&"*.log".to_string()));
        assert!(package.include_patterns.contains(&"*.lua".to_string()));
    }

    #[test]
    fn test_config_package_management() {
        let mut config = Config::default();
        
        // Test adding package
        let package = PackageConfig::new(
            "nvim".to_string(),
            "Neovim configuration".to_string(),
            vec![PathBuf::from("~/.config/nvim")],
        );
        config.set_package(package);
        
        // Test getting package
        let retrieved = config.get_package("nvim");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "nvim");
        
        // Test adding another package
        let zsh_package = PackageConfig::new(
            "zsh".to_string(),
            "Zsh configuration".to_string(),
            vec![PathBuf::from("~/.zshrc"), PathBuf::from("~/.config/zsh")],
        );
        config.set_package(zsh_package);
        
        // Test listing packages
        let packages = config.list_packages();
        assert_eq!(packages.len(), 2);
        assert!(packages.contains(&&"nvim".to_string()));
        assert!(packages.contains(&&"zsh".to_string()));
        
        // Test removing package
        config.remove_package("nvim");
        let packages = config.list_packages();
        assert_eq!(packages.len(), 1);
        assert!(packages.contains(&&"zsh".to_string()));
    }

    #[test]
    fn test_config_default_empty_packages() {
        let config = Config::default();
        assert!(config.packages.is_empty());
        assert_eq!(config.list_packages().len(), 0);
    }

    #[test]
    fn test_config_multiple_packages() {
        let mut config = Config::default();
        
        let nvim_package = PackageConfig::new(
            "nvim".to_string(),
            "Neovim configuration".to_string(),
            vec![PathBuf::from("~/.config/nvim")],
        );
        
        let zsh_package = PackageConfig::new(
            "zsh".to_string(),
            "Zsh configuration".to_string(),
            vec![PathBuf::from("~/.zshrc")],
        );

        config.set_package(nvim_package);
        config.set_package(zsh_package);

        let packages = config.list_packages();
        assert_eq!(packages.len(), 2);
        assert!(packages.contains(&&"nvim".to_string()));
        assert!(packages.contains(&&"zsh".to_string()));
    }

    #[tokio::test]
    async fn test_config_save_load_with_packages() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let mut original_config = Config::default();
        
        // Add some packages
        let nvim_package = PackageConfig::new(
            "nvim".to_string(),
            "Neovim configuration".to_string(),
            vec![PathBuf::from("~/.config/nvim")],
        );
        original_config.set_package(nvim_package);

        let mut zsh_package = PackageConfig::new(
            "zsh".to_string(),
            "Zsh configuration".to_string(),
            vec![PathBuf::from("~/.config/zsh"), PathBuf::from("~/.zshrc")],
        );
        zsh_package.exclude_patterns = vec!["*.log".to_string()];
        original_config.set_package(zsh_package);

        // Save config
        original_config.save(&config_path).await.unwrap();

        // Load config
        let loaded_config = Config::load(&config_path).await.unwrap();

        // Verify packages were saved and loaded correctly
        assert_eq!(loaded_config.packages.len(), 2);
        
        let nvim = loaded_config.get_package("nvim").unwrap();
        assert_eq!(nvim.name, "nvim");
        assert_eq!(nvim.description, "Neovim configuration");
        assert_eq!(nvim.paths.len(), 1);

        let zsh = loaded_config.get_package("zsh").unwrap();
        assert_eq!(zsh.name, "zsh");
        assert_eq!(zsh.paths.len(), 2);
        assert_eq!(zsh.exclude_patterns.len(), 1);
        assert!(zsh.exclude_patterns.contains(&"*.log".to_string()));
    }
}

#[cfg(test)]
mod types_tests {
    use super::*;

    #[test]
    fn test_file_metadata() {
        let metadata = FileMetadata {
            path: PathBuf::from("/test/path"),
            size: 1024,
            modified: Utc::now(),
            accessed: Utc::now(),
            created: Some(Utc::now()),
            file_type: FileType::File,
            permissions: 0o644,
            uid: 1000,
            gid: 1000,
            content_hash: None,
            directory_hash: None,
            extended_attributes: std::collections::HashMap::new(),
            requires_privileges: false,
        };
        
        assert_eq!(metadata.size, 1024);
        assert_eq!(metadata.file_type, FileType::File);
        assert!(!metadata.is_symlink());
        assert!(metadata.symlink_target().is_none());
    }

    #[test]
    fn test_operation_result() {
        let result = OperationResult {
            operation_type: OperationType::Backup,
            success: true,
            path: PathBuf::from("/test/path"),
            error: None,
            details: Some("Test operation".to_string()),
            required_privileges: false,
            duration: Some(std::time::Duration::from_millis(100)),
            bytes_processed: Some(1024),
        };
        
        assert!(result.success);
        assert_eq!(result.bytes_processed, Some(1024));
        assert!(result.error.is_none());
    }

    #[test]
    fn test_backup_session() {
        let session = BackupSession {
            id: Uuid::new_v4(),
            started_at: Utc::now(),
            source_paths: vec![PathBuf::from("/test")],
            backup_dir: PathBuf::from("/backup"),
            total_files: 5,
            processed_files: 3,
            total_size: 2048,
            processed_size: 1024,
            errors: vec!["Test error".to_string()],
        };
        
        assert_eq!(session.total_files, 5);
        assert_eq!(session.processed_files, 3);
        assert_eq!(session.errors.len(), 1);
    }

    #[test]
    fn test_conflict_types() {
        let metadata = FileMetadata::new(PathBuf::from("/test"), FileType::File);
        let conflict = Conflict {
            path: PathBuf::from("/test/conflict"),
            conflict_type: ConflictType::ContentMismatch,
            backup_metadata: metadata.clone(),
            current_metadata: metadata,
            suggested_resolution: ConflictResolution::AskUser,
            resolution: Some(ConflictResolution::AskUser),
        };
        
        assert_eq!(conflict.conflict_type, ConflictType::ContentMismatch);
        assert_eq!(conflict.resolution, Some(ConflictResolution::AskUser));
    }
}

#[cfg(test)]
mod error_tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let dotman_error = DotmanError::from(io_error);
        
        match dotman_error {
            DotmanError::Io(_) => {}, // Expected
            _ => panic!("Expected Io error variant"),
        }
    }

    #[test]
    fn test_file_not_found_error() {
        let path = PathBuf::from("/nonexistent/file");
        let error = DotmanError::file_not_found(path.clone());
        
        match error {
            DotmanError::FileNotFound { path: p } => assert_eq!(p, path),
            _ => panic!("Expected FileNotFound variant"),
        }
    }

    #[test]
    fn test_validation_error() {
        let message = "Invalid configuration";
        let error = DotmanError::validation(message.to_string());
        
        match error {
            DotmanError::Validation { message: msg } => assert_eq!(msg, message),
            _ => panic!("Expected Validation variant"),
        }
    }

    #[test]
    fn test_backup_error() {
        let backup_id = "backup-123";
        let error = DotmanError::backup(format!("Backup not found: {}", backup_id));
        
        match error {
            DotmanError::Backup { message } => assert!(message.contains(backup_id)),
            _ => panic!("Expected Backup variant"),
        }
    }
}

#[cfg(test)]
mod pattern_matching_tests {
    use super::*;
    use glob::Pattern;

    #[test]
    fn test_glob_patterns() {
        let pattern = Pattern::new("*.txt").unwrap();
        assert!(pattern.matches("test.txt"));
        assert!(!pattern.matches("test.log"));
    }

    #[test]
    fn test_directory_patterns() {
        let pattern = Pattern::new("**/*.conf").unwrap();
        assert!(pattern.matches("config/app.conf"));
        assert!(pattern.matches("deep/nested/dir/settings.conf"));
    }

    #[test]
    fn test_recursive_patterns() {
        let pattern = Pattern::new("**/target/**").unwrap();
        assert!(pattern.matches("project/target/debug/file"));
        assert!(pattern.matches("nested/project/target/release/binary"));
    }

    #[test]
    fn test_negation_patterns() {
        let include_pattern = Pattern::new("*.rs").unwrap();
        let exclude_pattern = Pattern::new("**/target/**").unwrap();
        
        let file1 = "src/main.rs";
        let file2 = "target/debug/main.rs";
        
        assert!(include_pattern.matches(file1));
        assert!(include_pattern.matches(file2));
        assert!(!exclude_pattern.matches(file1));
        assert!(exclude_pattern.matches(file2));
        
        // Simulate include/exclude logic
        let should_include_file1 = include_pattern.matches(file1) && !exclude_pattern.matches(file1);
        let should_include_file2 = include_pattern.matches(file2) && !exclude_pattern.matches(file2);
        
        assert!(should_include_file1);
        assert!(!should_include_file2);
    }
}

#[cfg(test)]
mod filesystem_tests {
    use super::*;

    #[tokio::test]
    async fn test_filesystem_operations() {
        let temp_dir = TempDir::new().unwrap();
        let filesystem = FileSystemImpl::new();
        
        let test_file = temp_dir.path().join("test.txt");
        let content = b"Hello, World!";
        
        // Write file
        filesystem.write_file(&test_file, content).await.unwrap();
        assert!(filesystem.exists(&test_file).await.unwrap());
        
        // Read file
        let read_content = filesystem.read_file(&test_file).await.unwrap();
        assert_eq!(read_content, content);
        
        // Get metadata
        let metadata = filesystem.metadata(&test_file).await.unwrap();
        assert_eq!(metadata.size, content.len() as u64);
        
        // Test directory operations
        let test_dir = temp_dir.path().join("test_dir");
        filesystem.create_dir_all(&test_dir).await.unwrap();
        assert!(filesystem.exists(&test_dir).await.unwrap());
        
        let dir_metadata = filesystem.metadata(&test_dir).await.unwrap();
        assert!(dir_metadata.is_directory());
    }

    #[tokio::test]
    async fn test_dry_run_filesystem() {
        let temp_dir = TempDir::new().unwrap();
        let filesystem = FileSystemImpl::new_dry_run();
        
        let test_file = temp_dir.path().join("dry_run_test.txt");
        
        // In dry run mode, operations should succeed but not actually modify the filesystem
        let result = filesystem.write_file(&test_file, b"dry run content").await;
        assert!(result.is_ok());
        
        // File should not actually exist
        let std_fs_exists = test_file.exists();
        assert!(!std_fs_exists);
        
        // But the dry run filesystem should report it exists
        assert!(filesystem.exists(&test_file).await.unwrap());
    }
}

#[cfg(test)]
mod progress_tests {
    use super::*;

    #[test]
    fn test_progress_reporter_creation() {
        let reporter = CliProgressReporter::new(true);
        // Just test that it can be created - actual progress reporting is tested in integration tests
        let progress = ProgressInfo::new(50, 100, "Test progress".to_string());
        reporter.report_progress(&progress);
    }

    #[tokio::test]
    async fn test_progress_reporting() {
        let reporter = CliProgressReporter::new(false);
        
        let progress1 = ProgressInfo::new(0, 10, "Starting".to_string());
        let progress2 = ProgressInfo::new(5, 10, "Halfway".to_string());
        let progress3 = ProgressInfo::new(10, 10, "Complete".to_string());
        
        reporter.report_progress(&progress1);
        reporter.report_progress(&progress2);
        reporter.report_progress(&progress3);
        
        assert_eq!(progress1.percentage(), 0.0);
        assert_eq!(progress2.percentage(), 50.0);
        assert_eq!(progress3.percentage(), 100.0);
        assert!(progress3.is_complete());
    }
}

#[cfg(test)]
mod cli_tests {
    use super::*;
    use dotman_rs::cli::args::{InitArgs, ListArgs, ListTarget};

    #[test]
    fn test_package_args_parsing() {
        // Test PackageAction::Add
        let add_action = PackageAction::Add {
            name: "nvim".to_string(),
            description: Some("Neovim config".to_string()),
            paths: vec![PathBuf::from("~/.config/nvim")],
            exclude: vec!["*.log".to_string()],
            include: vec!["*.lua".to_string()],
        };

        match add_action {
            PackageAction::Add { name, description, paths, exclude, include } => {
                assert_eq!(name, "nvim");
                assert_eq!(description, Some("Neovim config".to_string()));
                assert_eq!(paths.len(), 1);
                assert_eq!(exclude.len(), 1);
                assert_eq!(include.len(), 1);
            }
            _ => panic!("Expected PackageAction::Add"),
        }
    }

    #[test]
    fn test_package_args_remove() {
        let remove_action = PackageAction::Remove {
            name: "nvim".to_string(),
            force: true,
        };

        match remove_action {
            PackageAction::Remove { name, force } => {
                assert_eq!(name, "nvim");
                assert!(force);
            }
            _ => panic!("Expected PackageAction::Remove"),
        }
    }

    #[test]
    fn test_package_args_show() {
        let show_action = PackageAction::Show {
            name: "nvim".to_string(),
        };

        match show_action {
            PackageAction::Show { name } => {
                assert_eq!(name, "nvim");
            }
            _ => panic!("Expected PackageAction::Show"),
        }
    }

    #[test]
    fn test_package_args_edit() {
        let edit_action = PackageAction::Edit {
            name: "nvim".to_string(),
            description: Some("Updated description".to_string()),
            add_paths: vec![PathBuf::from("~/.vimrc")],
            remove_paths: vec![],
            add_exclude: vec!["*.tmp".to_string()],
            remove_exclude: vec![],
            add_include: vec!["*.vim".to_string()],
            remove_include: vec![],
        };

        match edit_action {
            PackageAction::Edit { 
                name, 
                description, 
                add_paths, 
                remove_paths: _,
                add_exclude, 
                remove_exclude: _,
                add_include, 
                remove_include: _ 
            } => {
                assert_eq!(name, "nvim");
                assert_eq!(description, Some("Updated description".to_string()));
                assert_eq!(add_paths.len(), 1);
                assert_eq!(add_exclude.len(), 1);
                assert_eq!(add_include.len(), 1);
            }
            _ => panic!("Expected PackageAction::Edit"),
        }
    }

    #[test]
    fn test_list_target_packages() {
        let list_target = ListTarget::Packages;
        match list_target {
            ListTarget::Packages => {
                // This is what we expect
            }
            _ => panic!("Expected ListTarget::Packages"),
        }
    }

    #[test]
    fn test_dotman_args_structure() {
        // This test verifies that the CLI args structure compiles correctly
        // We're mainly testing the enum variants and structure
        
        // Test package actions can be created
        let add_action = PackageAction::Add {
            name: "test".to_string(),
            description: None,
            paths: vec![PathBuf::from("/test")],
            exclude: vec![],
            include: vec![],
        };
        
        match add_action {
            PackageAction::Add { name, .. } => {
                assert_eq!(name, "test");
            }
            _ => panic!("Expected PackageAction::Add"),
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use dotman_rs::Config;

    #[tokio::test]
    async fn test_package_workflow() {
        let temp_dir = tempdir().unwrap();
        let mut config = Config::default();
        config.config_dir = temp_dir.path().to_path_buf();
        config.backup_dir = temp_dir.path().join("backups");

        // Create a package
        let package = PackageConfig::new(
            "test-package".to_string(),
            "Test package for integration testing".to_string(),
            vec![temp_dir.path().join("test-config")],
        );

        // Add package to config
        config.set_package(package);

        // Verify package was added
        assert!(config.get_package("test-package").is_some());
        let retrieved_package = config.get_package("test-package").unwrap();
        assert_eq!(retrieved_package.name, "test-package");
        assert_eq!(retrieved_package.description, "Test package for integration testing");
        assert_eq!(retrieved_package.paths.len(), 1);

        // Test package listing
        let packages = config.list_packages();
        assert!(packages.contains(&&"test-package".to_string()));

        // Test package removal
        config.remove_package("test-package");
        assert!(config.get_package("test-package").is_none());
    }

    #[test]
    fn test_package_config_serialization() {
        let package = PackageConfig::new(
            "serialization-test".to_string(),
            "Test package for serialization".to_string(),
            vec![PathBuf::from("/test/path")],
        );

        // Basic verification that the package can be created and accessed
        assert_eq!(package.name, "serialization-test");
        assert_eq!(package.description, "Test package for serialization");
        assert_eq!(package.paths.len(), 1);
        assert_eq!(package.paths[0], PathBuf::from("/test/path"));
    }
}

// Additional tests for edge cases
#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_package_with_empty_paths() {
        let package = PackageConfig::new(
            "empty".to_string(),
            "Empty package".to_string(),
            vec![],
        );

        assert_eq!(package.paths.len(), 0);
        assert_eq!(package.name, "empty");
    }

    #[test]
    fn test_package_duplicate_paths() {
        let mut package = PackageConfig::new(
            "test".to_string(),
            "Test package".to_string(),
            vec![
                PathBuf::from("/path1"),
                PathBuf::from("/path1"), // Duplicate
                PathBuf::from("/path2"),
            ],
        );

        // The implementation should handle duplicates (or we should add logic for it)
        assert_eq!(package.paths.len(), 3); // Currently allows duplicates
    }

    #[test]
    fn test_package_config_serialization() {
        let mut package = PackageConfig::new(
            "nvim".to_string(),
            "Neovim configuration".to_string(),
            vec![PathBuf::from("~/.config/nvim")],
        );
        package.exclude_patterns = vec!["*.log".to_string()];
        package.include_patterns = vec!["*.lua".to_string()];

        // Test serialization
        let serialized = serde_json::to_string(&package).unwrap();
        let deserialized: PackageConfig = serde_json::from_str(&serialized).unwrap();

        assert_eq!(package.name, deserialized.name);
        assert_eq!(package.description, deserialized.description);
        assert_eq!(package.paths, deserialized.paths);
        assert_eq!(package.exclude_patterns, deserialized.exclude_patterns);
        assert_eq!(package.include_patterns, deserialized.include_patterns);
    }
} 