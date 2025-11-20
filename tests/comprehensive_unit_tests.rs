#![allow(clippy::too_many_lines)]

use anyhow::Result;
use dotman::commands::context::CommandContext;
use dotman::{DEFAULT_CONFIG_PATH, DEFAULT_REPO_DIR, DotmanContext, NULL_COMMIT_ID};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

mod context_tests {
    use super::*;

    fn setup_test_env() -> Result<(TempDir, PathBuf, PathBuf)> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path().join(DEFAULT_REPO_DIR);
        let config_path = temp_dir.path().join(DEFAULT_CONFIG_PATH);
        Ok((temp_dir, repo_path, config_path))
    }

    #[test]
    fn test_context_new_with_explicit_paths() -> Result<()> {
        let (_temp, repo_path, config_path) = setup_test_env()?;

        let ctx = DotmanContext::new_with_explicit_paths(repo_path.clone(), config_path.clone())?;

        assert_eq!(ctx.repo_path, repo_path);
        assert_eq!(ctx.config_path, config_path);
        assert!(!ctx.no_pager);
        assert!(config_path.exists());

        Ok(())
    }

    #[test]
    fn test_context_load_existing_config() -> Result<()> {
        let (_temp, repo_path, config_path) = setup_test_env()?;

        // Create config first
        let _ctx1 = DotmanContext::new_with_explicit_paths(repo_path.clone(), config_path.clone())?;

        // Load it again
        let ctx2 = DotmanContext::new_with_explicit_paths(repo_path.clone(), config_path)?;

        assert_eq!(ctx2.repo_path, repo_path);
        assert_eq!(ctx2.config.core.repo_path, repo_path);

        Ok(())
    }

    #[test]
    fn test_is_repo_initialized() -> Result<()> {
        let (_temp, repo_path, config_path) = setup_test_env()?;

        let ctx = DotmanContext::new_with_explicit_paths(repo_path, config_path)?;

        // Initially not initialized
        assert!(!ctx.is_repo_initialized());

        // Create repo structure - need all three: repo_path, index.bin, and HEAD
        fs::create_dir_all(&ctx.repo_path)?;
        fs::write(ctx.repo_path.join("HEAD"), "ref: refs/heads/main")?;
        fs::write(ctx.repo_path.join("index.bin"), [])?; // Empty index file

        // Now it should be initialized
        assert!(ctx.is_repo_initialized());

        Ok(())
    }

    #[test]
    fn test_ensure_repo_exists() -> Result<()> {
        let (_temp, repo_path, config_path) = setup_test_env()?;

        let ctx = DotmanContext::new_with_explicit_paths(repo_path, config_path)?;

        ctx.ensure_repo_exists()?;

        // ensure_repo_exists only creates directories, not files
        assert!(ctx.repo_path.exists());
        assert!(ctx.repo_path.join("commits").exists());
        assert!(ctx.repo_path.join("objects").exists());

        // These are NOT created by ensure_repo_exists
        assert!(!ctx.repo_path.join("HEAD").exists());
        assert!(!ctx.repo_path.join("refs/heads").exists());

        Ok(())
    }

    #[test]
    fn test_check_repo_initialized_error() -> Result<()> {
        let (_temp, repo_path, config_path) = setup_test_env()?;

        let ctx = DotmanContext::new_with_explicit_paths(repo_path, config_path)?;

        let result = ctx.check_repo_initialized();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));

        Ok(())
    }

    #[test]
    fn test_get_home_dir() -> Result<()> {
        let (_temp, repo_path, config_path) = setup_test_env()?;

        let ctx = DotmanContext::new_with_explicit_paths(repo_path, config_path)?;

        let home = CommandContext::get_home_dir(&ctx)?;
        assert!(home.exists());
        assert!(home.is_absolute());

        Ok(())
    }

    #[test]
    fn test_null_commit_id_constant() {
        assert_eq!(NULL_COMMIT_ID.len(), 40);
        assert!(NULL_COMMIT_ID.chars().all(|c| c == '0'));
    }
}

mod command_context_tests {
    use super::*;
    use dotman::commands::context::CommandContext;
    use dotman::storage::concurrent_index::ConcurrentIndex;

    #[test]
    fn test_load_concurrent_index() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path().join(DEFAULT_REPO_DIR);
        let config_path = temp_dir.path().join(DEFAULT_CONFIG_PATH);

        let ctx = DotmanContext::new_with_explicit_paths(repo_path, config_path)?;
        ctx.ensure_repo_exists()?;

        let index = ctx.load_concurrent_index()?;
        assert!(index.staged_entries().is_empty());

        Ok(())
    }

    #[test]
    fn test_save_concurrent_index() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path().join(DEFAULT_REPO_DIR);
        let config_path = temp_dir.path().join(DEFAULT_CONFIG_PATH);

        let ctx = DotmanContext::new_with_explicit_paths(repo_path, config_path)?;
        ctx.ensure_repo_exists()?;

        let index = ConcurrentIndex::new();
        index.save(&ctx.repo_path.join("index.bin"))?;

        let index_path = ctx.repo_path.join("index.bin");
        assert!(index_path.exists());

        Ok(())
    }

    #[test]
    fn test_ensure_initialized() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path().join(DEFAULT_REPO_DIR);
        let config_path = temp_dir.path().join(DEFAULT_CONFIG_PATH);

        let ctx = DotmanContext::new_with_explicit_paths(repo_path, config_path)?;

        // Should fail initially
        assert!(ctx.ensure_initialized().is_err());

        // Create repo directories
        ctx.ensure_repo_exists()?;

        // Create index and HEAD files to properly initialize
        let index = dotman::storage::index::Index::new();
        index.save(&ctx.repo_path.join("index.bin"))?;

        let ref_manager = dotman::refs::RefManager::new(ctx.repo_path.clone());
        ref_manager.init()?;

        // Should succeed now
        assert!(ctx.ensure_initialized().is_ok());

        Ok(())
    }

    #[test]
    fn test_get_current_branch() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path().join(DEFAULT_REPO_DIR);
        let config_path = temp_dir.path().join(DEFAULT_CONFIG_PATH);

        let ctx = DotmanContext::new_with_explicit_paths(repo_path, config_path)?;
        ctx.ensure_repo_exists()?;

        // Initialize refs properly
        let ref_manager = dotman::refs::RefManager::new(ctx.repo_path);
        ref_manager.init()?;

        let branch = ref_manager.current_branch()?;
        assert_eq!(branch, Some("main".to_string()));

        Ok(())
    }

    #[test]
    fn test_get_head_commit() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path().join(DEFAULT_REPO_DIR);
        let config_path = temp_dir.path().join(DEFAULT_CONFIG_PATH);

        let ctx = DotmanContext::new_with_explicit_paths(repo_path, config_path)?;
        ctx.ensure_repo_exists()?;

        // No commits yet
        let resolver = ctx.create_ref_resolver();
        let result = resolver.resolve("HEAD");
        assert!(result.is_err() || result.unwrap() == NULL_COMMIT_ID);

        Ok(())
    }
}

mod path_validation_tests {
    use super::*;

    #[test]
    fn test_repo_path_expansion() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let home_dir = dirs::home_dir().expect("Could not find home directory");

        // Test absolute path in config (tilde is NOT expanded during config load)
        let config_path = temp_dir.path().join("config");
        let abs_dotman_path = home_dir.join(".dotman");
        let config_content = format!(
            r#"
            [core]
            repo_path = "{}"
            compression = "zstd"
            compression_level = 3
            "#,
            abs_dotman_path.display()
        );
        fs::write(&config_path, config_content)?;

        let config = dotman::config::Config::load(&config_path)?;
        assert_eq!(config.core.repo_path, abs_dotman_path);

        Ok(())
    }

    #[test]
    fn test_absolute_path_preservation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let abs_path = temp_dir.path().join("my_repo");
        let config_path = temp_dir.path().join("config");

        let ctx = DotmanContext::new_with_explicit_paths(abs_path.clone(), config_path)?;
        assert_eq!(ctx.repo_path, abs_path);
        assert!(ctx.repo_path.is_absolute());

        Ok(())
    }
}

mod repository_initialization_tests {
    use super::*;

    #[test]
    fn test_repo_structure_creation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path().join("test_repo");
        let config_path = temp_dir.path().join("config");

        let ctx = DotmanContext::new_with_explicit_paths(repo_path.clone(), config_path)?;
        ctx.ensure_repo_exists()?;

        // Initialize the repository properly
        let index = dotman::storage::index::Index::new();
        index.save(&ctx.repo_path.join("index.bin"))?;

        let ref_manager = dotman::refs::RefManager::new(ctx.repo_path);
        ref_manager.init()?;

        // Check all required directories
        assert!(repo_path.join("refs/heads").exists());
        assert!(repo_path.join("refs/tags").exists());
        assert!(repo_path.join("refs/remotes").exists());
        assert!(repo_path.join("commits").exists());
        assert!(repo_path.join("objects").exists());

        // Check HEAD file
        let head_content = fs::read_to_string(repo_path.join("HEAD"))?;
        assert_eq!(head_content.trim(), "ref: refs/heads/main");

        // Check main branch ref
        let main_ref = repo_path.join("refs/heads/main");
        assert!(main_ref.exists());
        let main_content = fs::read_to_string(main_ref)?;
        assert_eq!(main_content, NULL_COMMIT_ID);

        Ok(())
    }

    #[test]
    fn test_double_initialization_idempotent() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path().join("test_repo");
        let config_path = temp_dir.path().join("config");

        let ctx = DotmanContext::new_with_explicit_paths(repo_path, config_path)?;

        // First initialization
        ctx.ensure_repo_exists()?;

        // Write some custom content
        let custom_file = ctx.repo_path.join("custom.txt");
        fs::write(&custom_file, "test data")?;

        // Second initialization should not fail
        ctx.ensure_repo_exists()?;

        // Custom file should still exist
        assert!(custom_file.exists());
        assert_eq!(fs::read_to_string(&custom_file)?, "test data");

        Ok(())
    }

    #[test]
    fn test_head_file_permissions() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path().join("test_repo");
        let config_path = temp_dir.path().join("config");

        let ctx = DotmanContext::new_with_explicit_paths(repo_path, config_path)?;
        ctx.ensure_repo_exists()?;

        // Compute head_file path before moving ctx.repo_path
        let head_file = ctx.repo_path.join("HEAD");

        // Initialize refs to create HEAD
        let ref_manager = dotman::refs::RefManager::new(ctx.repo_path);
        ref_manager.init()?;

        assert!(head_file.exists());

        // HEAD file should be readable and writable
        let metadata = fs::metadata(&head_file)?;
        assert!(!metadata.permissions().readonly());

        Ok(())
    }
}

mod error_handling_tests {
    use super::*;

    #[test]
    fn test_invalid_repo_path() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path().join("nonexistent/deeply/nested/path");
        let config_path = temp_dir.path().join("config");

        // This should succeed - the context is created
        let ctx = DotmanContext::new_with_explicit_paths(repo_path, config_path)?;

        // But repo operations should handle the non-existent path gracefully
        assert!(!ctx.is_repo_initialized());

        Ok(())
    }

    #[test]
    fn test_corrupted_head_file() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path().join("test_repo");
        let config_path = temp_dir.path().join("config");

        let ctx = DotmanContext::new_with_explicit_paths(repo_path, config_path)?;
        ctx.ensure_repo_exists()?;

        // Corrupt the HEAD file
        fs::write(ctx.repo_path.join("HEAD"), "invalid content")?;

        // Operations should handle this gracefully
        let ref_manager = dotman::refs::RefManager::new(ctx.repo_path);
        let result = ref_manager.current_branch();
        // Corrupted HEAD file might return None or error
        assert!(result.is_err() || result.unwrap().is_none());

        Ok(())
    }

    #[test]
    fn test_missing_config_directory() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path().join("repo");
        let config_path = temp_dir
            .path()
            .join("deeply/nested/config/path/config.toml");

        // Should create parent directories
        let _ctx = DotmanContext::new_with_explicit_paths(repo_path, config_path.clone())?;

        assert!(config_path.parent().unwrap().exists());
        assert!(config_path.exists());

        Ok(())
    }
}

mod config_validation_tests {
    use super::*;
    use dotman::config::Config;

    #[test]
    #[allow(clippy::unnecessary_wraps)]
    fn test_default_config_values() -> Result<()> {
        let config = Config::default();

        assert!(matches!(
            config.core.compression,
            dotman::config::CompressionType::Zstd
        ));
        assert_eq!(config.core.compression_level, 3);
        assert!(config.performance.parallel_threads > 0);
        assert!(config.performance.use_hard_links);
        assert!(config.tracking.preserve_permissions);
        assert!(!config.tracking.follow_symlinks);

        Ok(())
    }

    #[test]
    fn test_config_save_and_load() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("config.toml");

        let mut config = Config::default();
        config.user.name = Some("Test User".to_string());
        config.user.email = Some("test@example.com".to_string());
        config.core.compression_level = 9;

        config.save(&config_path)?;

        let loaded = Config::load(&config_path)?;
        assert_eq!(loaded.user.name, Some("Test User".to_string()));
        assert_eq!(loaded.user.email, Some("test@example.com".to_string()));
        assert_eq!(loaded.core.compression_level, 9);

        Ok(())
    }

    #[test]
    fn test_invalid_compression_level() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("config.toml");

        // Write config with invalid compression level
        let config_content = r#"
            [core]
            repo_path = "~/.dotman"
            compression = "zstd"
            compression_level = 99
        "#;
        fs::write(&config_path, config_content)?;

        // Should fail validation
        let result = Config::load(&config_path);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Compression level must be between 1 and 22")
        );

        Ok(())
    }
}

mod concurrent_operations_tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_concurrent_context_creation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let repo_path = Arc::new(temp_dir.path().join("repo"));
        let config_path = Arc::new(temp_dir.path().join("config"));

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let repo = repo_path.clone();
                let config = config_path.clone();
                thread::spawn(move || {
                    DotmanContext::new_with_explicit_paths((*repo).clone(), (*config).clone())
                })
            })
            .collect();

        for handle in handles {
            let ctx = handle.join().unwrap()?;
            assert_eq!(ctx.repo_path, *repo_path);
        }

        Ok(())
    }

    #[test]
    fn test_concurrent_repo_initialization() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let repo_path = Arc::new(temp_dir.path().join("repo"));
        let config_path = Arc::new(temp_dir.path().join("config"));

        // Create contexts first
        let ctx =
            DotmanContext::new_with_explicit_paths((*repo_path).clone(), (*config_path).clone())?;
        let ctx = Arc::new(ctx);

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let context = ctx.clone();
                thread::spawn(move || context.ensure_repo_exists())
            })
            .collect();

        for handle in handles {
            handle.join().unwrap()?;
        }

        // Verify repo structure is intact - only directories are created by ensure_repo_exists
        assert!(repo_path.join("commits").exists());
        assert!(repo_path.join("objects").exists());
        // HEAD and refs are NOT created by ensure_repo_exists

        Ok(())
    }
}
