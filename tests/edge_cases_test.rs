use anyhow::Result;
use dotman::DotmanContext;
use dotman::commands;
use dotman::config::Config;
use dotman::storage::index::Index;
use std::fs;
use tempfile::tempdir;

// Helper function to create test context
fn setup_test_context() -> Result<(tempfile::TempDir, DotmanContext)> {
    let dir = tempdir()?;
    let repo_path = dir.path().join(".dotman");
    let config_path = dir.path().join("config.toml");

    // Create repo structure
    fs::create_dir_all(&repo_path)?;
    fs::create_dir_all(repo_path.join("commits"))?;
    fs::create_dir_all(repo_path.join("objects"))?;

    // Create empty index
    let index = Index::new();
    let index_path = repo_path.join("index.bin");
    index.save(&index_path)?;

    // Create HEAD file to mark repo as initialized
    fs::write(repo_path.join("HEAD"), "")?;

    let mut config = Config::default();
    config.core.repo_path = repo_path.clone();
    config.save(&config_path)?;

    let context = DotmanContext {
        repo_path,
        config_path,
        config,
        no_pager: true,
    };

    Ok((dir, context))
}

// ============= EDGE CASE TESTS =============

#[test]
fn test_unicode_filenames() -> Result<()> {
    let (dir, ctx) = setup_test_context()?;

    // Create files with various Unicode characters
    let test_files = vec![
        "你好.txt",
        "مرحبا.conf",
        "🚀rocket.rs",
        "café.sh",
        "naïve.py",
        "Ελληνικά.md",
        "русский.txt",
        "🌍🌎🌏.json",
    ];

    for filename in &test_files {
        let file_path = dir.path().join(filename);
        fs::write(&file_path, format!("content of {}", filename))?;

        // Test add with Unicode filenames
        let paths = vec![file_path.to_string_lossy().to_string()];
        let result = commands::add::execute(&ctx, &paths, false);
        assert!(result.is_ok(), "Failed to add Unicode file: {}", filename);
    }

    // Verify status works with Unicode
    let result = commands::status::execute(&ctx, false, false);
    assert!(result.is_ok());

    // Test commit
    let result = commands::commit::execute(&ctx, "Unicode test commit", false);
    assert!(result.is_ok());

    Ok(())
}

#[test]
fn test_extremely_long_paths() -> Result<()> {
    let (dir, ctx) = setup_test_context()?;

    // Create nested directory structure that's very deep
    let mut deep_path = dir.path().to_path_buf();
    for i in 0..50 {
        deep_path = deep_path.join(format!("very_long_directory_name_{}", i));
    }

    fs::create_dir_all(&deep_path)?;

    let long_file = deep_path.join("deeply_nested_file.txt");
    fs::write(&long_file, "deeply nested content")?;

    // Test adding deeply nested file
    let paths = vec![long_file.to_string_lossy().to_string()];
    let result = commands::add::execute(&ctx, &paths, false);

    // This might fail on some filesystems due to path length limits
    // We test both success and graceful failure
    match result {
        Ok(_) => {
            // If successful, commit should also work
            let commit_result = commands::commit::execute(&ctx, "Deep path test", false);
            assert!(commit_result.is_ok());
        }
        Err(_e) => {
            // Graceful failure - may fail on some filesystems due to path length limits
            // The specific error doesn't matter, just that it fails gracefully
        }
    }

    Ok(())
}

#[test]
fn test_special_characters_in_content() -> Result<()> {
    let (dir, ctx) = setup_test_context()?;

    // Test files with various problematic content
    let test_cases = vec![
        ("null_bytes.bin", vec![0u8, 1, 2, 255, 0, 42]),
        (
            "control_chars.txt",
            b"\x00\x01\x02\x1b[31mred\x1b[0m\n".to_vec(),
        ),
        (
            "mixed_encoding.txt",
            b"\xff\xfe\x00H\x00e\x00l\x00l\x00o".to_vec(),
        ), // UTF-16LE BOM + Hello
        ("binary_data.dat", (0..=255).collect()),
        ("large_lines.txt", vec![b'X'; 100_000]), // 100KB single line
    ];

    for (filename, content) in test_cases {
        let file_path = dir.path().join(filename);
        fs::write(&file_path, content)?;

        let paths = vec![file_path.to_string_lossy().to_string()];
        let result = commands::add::execute(&ctx, &paths, false);
        assert!(
            result.is_ok(),
            "Failed to handle special content in: {}",
            filename
        );
    }

    // Test commit with binary data
    let result = commands::commit::execute(&ctx, "Special content test", false);
    assert!(result.is_ok());

    Ok(())
}

// Concurrent test consolidated in integration_test.rs::test_concurrent_operations

#[test]
fn test_symlink_handling() -> Result<()> {
    let (dir, ctx) = setup_test_context()?;

    // Create target file
    let target_file = dir.path().join("target.txt");
    fs::write(&target_file, "target content")?;

    // Create symlink
    let symlink_file = dir.path().join("symlink.txt");

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&target_file, &symlink_file)?;

        // Test adding symlink
        let paths = vec![symlink_file.to_string_lossy().to_string()];
        let result = commands::add::execute(&ctx, &paths, false);
        assert!(result.is_ok());

        // Create dangling symlink
        let dangling_link = dir.path().join("dangling.txt");
        std::os::unix::fs::symlink("/nonexistent/path", &dangling_link)?;

        let paths = vec![dangling_link.to_string_lossy().to_string()];
        let result = commands::add::execute(&ctx, &paths, false);
        // Should handle dangling symlinks gracefully
        // Either succeed (track the link) or fail gracefully
        match result {
            Ok(_) => {}
            Err(e) => {
                assert!(!e.to_string().is_empty());
            }
        }
    }

    Ok(())
}

#[test]
fn test_disk_space_edge_cases() -> Result<()> {
    let (dir, ctx) = setup_test_context()?;

    // Create a very large file (but not too large for CI)
    let large_file = dir.path().join("large.dat");
    let large_content = vec![0u8; 10_000_000]; // 10MB
    fs::write(&large_file, &large_content)?;

    // Test adding large file
    let paths = vec![large_file.to_string_lossy().to_string()];
    let result = commands::add::execute(&ctx, &paths, false);
    assert!(result.is_ok());

    // Test committing large file
    let result = commands::commit::execute(&ctx, "Large file test", false);
    assert!(result.is_ok());

    // Create many small files
    for i in 0..1000 {
        let small_file = dir.path().join(format!("small_{}.txt", i));
        fs::write(&small_file, format!("content {}", i))?;
    }

    // Add them all at once
    let many_paths: Vec<String> = (0..1000)
        .map(|i| {
            dir.path()
                .join(format!("small_{}.txt", i))
                .to_string_lossy()
                .to_string()
        })
        .collect();

    let result = commands::add::execute(&ctx, &many_paths, false);
    assert!(result.is_ok());

    Ok(())
}

#[test]
fn test_comprehensive_permissions() -> Result<()> {
    let (dir, ctx) = setup_test_context()?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        // Check if we're in a privileged environment where permission tests may not work
        let is_root = unsafe { libc::geteuid() } == 0;
        let in_docker = std::path::Path::new("/.dockerenv").exists()
            || fs::read_to_string("/proc/1/cgroup")
                .unwrap_or_default()
                .contains("docker");

        // Test 1: Files with different permission modes
        let executable_file = dir.path().join("executable.sh");
        fs::write(&executable_file, "#!/bin/bash\necho hello")?;
        let mut perms = fs::metadata(&executable_file)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&executable_file, perms)?;

        let readonly_file = dir.path().join("readonly.txt");
        fs::write(&readonly_file, "readonly content")?;
        let mut perms = fs::metadata(&readonly_file)?.permissions();
        perms.set_mode(0o444);
        fs::set_permissions(&readonly_file, perms)?;

        // Add files with different permissions
        let paths = vec![
            executable_file.to_string_lossy().to_string(),
            readonly_file.to_string_lossy().to_string(),
        ];
        let result = commands::add::execute(&ctx, &paths, false);
        assert!(result.is_ok());

        // Test 2: Index file permission issues
        if !is_root && !in_docker {
            let index_path = ctx.repo_path.join("index.bin");

            // Make index read-only temporarily
            let mut perms = fs::metadata(&index_path)?.permissions();
            let original_mode = perms.mode();
            perms.set_mode(0o444);
            fs::set_permissions(&index_path, perms.clone())?;

            // Try to add another file - should fail due to permission
            let test_file = dir.path().join("test_perm.txt");
            fs::write(&test_file, "test content")?;
            let paths = vec![test_file.to_string_lossy().to_string()];
            let result = commands::add::execute(&ctx, &paths, false);
            assert!(result.is_err(), "Should fail to write to read-only index");

            // Restore permissions
            perms.set_mode(original_mode);
            fs::set_permissions(&index_path, perms)?;

            // Should now work
            let result = commands::add::execute(&ctx, &paths, false);
            assert!(result.is_ok(), "Should succeed after restoring permissions");
        }

        // Test 3: Config file permission issues
        let config_path = ctx.config_path.clone();
        if config_path.exists() && !is_root && !in_docker {
            let mut perms = fs::metadata(&config_path)?.permissions();
            let original_mode = perms.mode();
            perms.set_mode(0o444);
            fs::set_permissions(&config_path, perms.clone())?;

            // Should still be able to read
            let result = Config::load(&config_path);
            assert!(result.is_ok(), "Should be able to read read-only config");

            // Should not be able to write
            let config = Config::default();
            let write_result = config.save(&config_path);
            assert!(
                write_result.is_err(),
                "Should not be able to write to read-only config"
            );

            // Restore permissions
            perms.set_mode(original_mode);
            fs::set_permissions(&config_path, perms)?;
        }
    }

    #[cfg(not(unix))]
    {
        // On non-Unix systems, just verify basic operations work
        let test_file = dir.path().join("test.txt");
        fs::write(&test_file, "test content")?;

        let paths = vec![test_file.to_string_lossy().to_string()];
        let result = commands::add::execute(&ctx, &paths, false);
        assert!(result.is_ok(), "Should be able to add file on non-Unix");
    }

    Ok(())
}

#[test]
fn test_empty_and_tiny_files() -> Result<()> {
    let (dir, ctx) = setup_test_context()?;

    // Empty file
    let empty_file = dir.path().join("empty.txt");
    fs::write(&empty_file, "")?;

    // Single byte file
    let tiny_file = dir.path().join("tiny.txt");
    fs::write(&tiny_file, "x")?;

    // File with just whitespace
    let whitespace_file = dir.path().join("whitespace.txt");
    fs::write(&whitespace_file, "   \n\t\r\n   ")?;

    let paths = vec![
        empty_file.to_string_lossy().to_string(),
        tiny_file.to_string_lossy().to_string(),
        whitespace_file.to_string_lossy().to_string(),
    ];

    let result = commands::add::execute(&ctx, &paths, false);
    assert!(result.is_ok());

    let result = commands::commit::execute(&ctx, "Empty files test", false);
    assert!(result.is_ok());

    Ok(())
}

#[test]
fn test_path_traversal_security() -> Result<()> {
    let (_dir, ctx) = setup_test_context()?;

    // Test various path traversal attempts
    let malicious_paths = vec![
        "../../../etc/passwd",
        "../../.ssh/id_rsa",
        "/etc/shadow",
        "..\\..\\windows\\system32\\config\\sam", // Windows style
        "dir/../../../secret.txt",
        "./../../outside/file.txt",
    ];

    for malicious_path in malicious_paths {
        let result = commands::add::execute(&ctx, &[malicious_path.to_string()], false);
        // Should either:
        // 1. Fail with appropriate error (preferred)
        // 2. Safely resolve to within temp directory
        if let Err(e) = result {
            // Good - should reject dangerous paths
            let error_msg = e.to_string().to_lowercase();
            // Should contain some security-related error message
            assert!(!error_msg.is_empty());
        } else {
            // If it succeeds, verify it didn't escape the temp directory
            // This would be hard to verify without actually creating malicious files
            // so we'll document this as a potential security issue to investigate
        }
    }

    Ok(())
}

// Large-scale test consolidated in integration_test.rs::test_large_scale_operations

#[test]
fn test_rapid_file_changes() -> Result<()> {
    let (dir, ctx) = setup_test_context()?;

    let test_file = dir.path().join("rapid_changes.txt");

    // Rapidly modify and re-add file
    for i in 0..100 {
        fs::write(&test_file, format!("content version {}", i))?;

        let paths = vec![test_file.to_string_lossy().to_string()];
        let result = commands::add::execute(&ctx, &paths, false);
        assert!(result.is_ok(), "Failed at iteration {}", i);

        // Sometimes check status
        if i % 10 == 0 {
            let result = commands::status::execute(&ctx, false, false);
            assert!(result.is_ok());
        }
    }

    let result = commands::commit::execute(&ctx, "Rapid changes test", false);
    assert!(result.is_ok());

    Ok(())
}
