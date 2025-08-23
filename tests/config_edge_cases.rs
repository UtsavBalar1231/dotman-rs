use anyhow::Result;
use dotman::config::{
    CompressionType, Config, CoreConfig, PerformanceConfig, RemoteConfig, RemoteType,
    TrackingConfig,
};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_malformed_toml_configs() -> Result<()> {
    let dir = tempdir()?;

    let malformed_configs = vec![
        // Invalid TOML syntax
        ("invalid_syntax.toml", "invalid toml content {{ broken"),
        ("unclosed_brackets.toml", "[core\nrepo_path = \"test\""),
        (
            "invalid_quotes.toml",
            "[core]\nrepo_path = \"unclosed string",
        ),
        (
            "invalid_escapes.toml",
            "[core]\nrepo_path = \"\\invalid\\escape\"",
        ),
        // Valid TOML but invalid values
        (
            "negative_compression.toml",
            "[core]\ncompression_level = -1",
        ),
        ("huge_compression.toml", "[core]\ncompression_level = 999"),
        ("zero_threads.toml", "[performance]\nparallel_threads = 0"),
        (
            "negative_threads.toml",
            "[performance]\nparallel_threads = -5",
        ),
        (
            "huge_cache.toml",
            "[performance]\ncache_size = 999999999999",
        ),
        // Type mismatches
        (
            "string_compression.toml",
            "[core]\ncompression_level = \"high\"",
        ),
        (
            "bool_threads.toml",
            "[performance]\nparallel_threads = true",
        ),
        (
            "array_repo_path.toml",
            "[core]\nrepo_path = [\"path1\", \"path2\"]",
        ),
        // Missing required sections
        ("empty.toml", ""),
        ("partial.toml", "[performance]\nparallel_threads = 4"),
        // Extremely long values
        (
            "long_patterns.toml",
            "[tracking]\nignore_patterns = [\"pattern1\", \"pattern2\", \"pattern3\", \"pattern4\", \"pattern5\", \"pattern6\", \"pattern7\", \"pattern8\", \"pattern9\", \"pattern10\", \"pattern11\", \"pattern12\"]",
        ),
    ];

    // Add long path test separately to avoid lifetime issues
    let long_path_content = format!("[core]\nrepo_path = \"{}\"", "x".repeat(10000));
    let config_path = dir.path().join("long_path.toml");
    fs::write(&config_path, &long_path_content)?;

    let result = Config::load(&config_path);
    match result {
        Ok(_) => {}
        Err(_) => {
            // Should be able to recover with default config
            let default_config = Config::default();
            let save_result = default_config.save(&config_path);
            assert!(save_result.is_ok());
        }
    }

    for (filename, content) in malformed_configs {
        let config_path = dir.path().join(filename);
        fs::write(&config_path, content)?;

        let result = Config::load(&config_path);
        match result {
            Ok(_) => {
                // If it loads successfully, verify values are sane
                // Some configs might load with defaults filled in
            }
            Err(_) => {
                // Good - should reject malformed configs
                // Verify we can recover by creating a default config
                let default_config = Config::default();
                let save_result = default_config.save(&config_path);
                assert!(
                    save_result.is_ok(),
                    "Should be able to save default config to recover"
                );

                let recovered = Config::load(&config_path);
                assert!(recovered.is_ok(), "Should be able to load default config");
            }
        }
    }

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
        branches: Default::default(),
        performance: PerformanceConfig {
            parallel_threads: 1024, // Very high thread count
            mmap_threshold: 1,      // Everything uses mmap
            cache_size: 10000,      // 10GB cache (max allowed)
            use_hard_links: true,
        },
        tracking: TrackingConfig {
            ignore_patterns: (0..10000).map(|i| format!("pattern_{}", i)).collect(), // Many patterns
            follow_symlinks: true,
            preserve_permissions: true,
        },
    };

    // Should be able to save extreme config
    let result = extreme_config.save(&config_path);
    assert!(result.is_ok());

    // Should be able to load it back
    let loaded = Config::load(&config_path);
    match &loaded {
        Ok(_) => {}
        Err(e) => {
            println!("Failed to load extreme config: {}", e);
            panic!("Config load failed");
        }
    }

    let loaded_config = loaded.unwrap();
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
            repo_path: dir.path().join("🚀dotman"),
            default_branch: "主分支".to_string(), // Chinese for "main branch"
            compression: CompressionType::Zstd,
            compression_level: 3,
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
        branches: Default::default(),
        performance: PerformanceConfig {
            parallel_threads: 8,
            mmap_threshold: 1048576,
            cache_size: 100,
            use_hard_links: true,
        },
        tracking: TrackingConfig {
            ignore_patterns: vec![
                "*.log".to_string(),
                "🗑️temp".to_string(),     // Emoji
                "naïve_file".to_string(), // Accented characters
                "файл*.txt".to_string(),  // Cyrillic
            ],
            follow_symlinks: false,
            preserve_permissions: true,
        },
    };

    // Should handle Unicode in config
    let result = unicode_config.save(&config_path);
    assert!(result.is_ok());

    let loaded = Config::load(&config_path);
    assert!(loaded.is_ok());

    let loaded_config = loaded.unwrap();
    assert_eq!(loaded_config.core.default_branch, "主分支");
    assert!(
        loaded_config
            .tracking
            .ignore_patterns
            .contains(&"🗑️temp".to_string())
    );

    Ok(())
}

#[test]
fn test_concurrent_config_access() -> Result<()> {
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    let dir = tempdir()?;
    let config_path = Arc::new(dir.path().join("concurrent.toml"));

    // Create initial config
    let initial_config = Config::default();
    initial_config.save(&config_path)?;

    // Spawn multiple threads that read/write config simultaneously
    let handles: Vec<_> = (0..10)
        .map(|thread_id| {
            let config_path = config_path.clone();
            thread::spawn(move || {
                for i in 0..50 {
                    // Read config
                    let _loaded = Config::load(&config_path);

                    // Modify and save config
                    let mut config = Config::default();
                    config.core.compression_level = (thread_id + i) % 22 + 1;
                    let _result = config.save(&config_path);

                    thread::sleep(Duration::from_millis(1));
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    // Config should still be loadable after concurrent access
    let final_config = Config::load(&config_path);
    assert!(final_config.is_ok());

    Ok(())
}

#[test]
fn test_config_file_corruption() -> Result<()> {
    let dir = tempdir()?;
    let config_path = dir.path().join("corrupted.toml");

    // Create valid config first
    let config = Config::default();
    config.save(&config_path)?;

    // Corrupt the file by truncating it
    let original_content = fs::read(&config_path)?;
    let truncated = &original_content[..original_content.len() / 2];
    fs::write(&config_path, truncated)?;

    // Should handle corruption gracefully
    let result = Config::load(&config_path);
    assert!(result.is_err());

    // Corrupt with binary data
    fs::write(&config_path, vec![0u8; 1000])?;
    let result = Config::load(&config_path);
    assert!(result.is_err());

    // Corrupt with mixed valid/invalid content
    let mixed_content = format!(
        "{}\n\x00\x01\x02invalid binary data",
        "[core]\nrepo_path = \"/tmp\""
    );
    fs::write(&config_path, mixed_content)?;
    let result = Config::load(&config_path);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_config_permission_issues() -> Result<()> {
    let dir = tempdir()?;
    let config_path = dir.path().join("permissions.toml");

    let config = Config::default();
    config.save(&config_path)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        // Make config read-only
        let mut perms = fs::metadata(&config_path)?.permissions();
        perms.set_mode(0o444);
        fs::set_permissions(&config_path, perms)?;

        // Should still be able to read
        let result = Config::load(&config_path);
        assert!(result.is_ok());

        // Should not be able to write
        let write_result = config.save(&config_path);
        assert!(write_result.is_err());

        // Restore permissions for cleanup
        let mut perms = fs::metadata(&config_path)?.permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&config_path, perms)?;
    }

    Ok(())
}

#[test]
fn test_config_large_file_handling() -> Result<()> {
    let dir = tempdir()?;
    let config_path = dir.path().join("large.toml");

    // Create very large config with many ignore patterns
    let mut large_config = Config::default();
    large_config.tracking.ignore_patterns = (0..100000)
        .map(|i| format!("pattern_{}_with_some_longer_content_to_make_it_bigger", i))
        .collect();

    // Should handle large config
    let result = large_config.save(&config_path);
    assert!(result.is_ok());

    // Check file size
    let metadata = fs::metadata(&config_path)?;
    assert!(metadata.len() > 1_000_000); // Should be over 1MB

    // Should be able to load large config
    let loaded = Config::load(&config_path);
    match &loaded {
        Ok(_) => {}
        Err(e) => {
            println!("Failed to load extreme config: {}", e);
            panic!("Config load failed");
        }
    }

    let loaded_config = loaded.unwrap();
    assert_eq!(loaded_config.tracking.ignore_patterns.len(), 100000);

    Ok(())
}

#[test]
fn test_config_validation_limits() -> Result<()> {
    let dir = tempdir()?;
    let config_path = dir.path().join("validation.toml");

    // Test that config validation properly rejects values outside allowed ranges
    let invalid_configs = vec![
        "[performance]\ncache_size = 100000",  // Over 10GB limit
        "[performance]\ncache_size = -1",      // Negative
        "[core]\ncompression_level = 99",      // Too high
        "[core]\ncompression_level = -1",      // Negative
        "[performance]\nparallel_threads = 0", // Zero threads
    ];

    for invalid_config in invalid_configs {
        fs::write(&config_path, invalid_config)?;
        let result = Config::load(&config_path);
        assert!(
            result.is_err(),
            "Should reject invalid config: {}",
            invalid_config
        );
    }

    Ok(())
}
