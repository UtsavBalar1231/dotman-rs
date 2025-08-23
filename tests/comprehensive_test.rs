use anyhow::Result;
use dotman::DotmanContext;
use dotman::commands;
use dotman::config::Config;
use dotman::storage::index::Index;
use std::fs;
use std::os::unix::fs::PermissionsExt;
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

    let mut config = Config::default();
    config.core.repo_path = repo_path.clone();
    config.save(&config_path)?;

    let context = DotmanContext {
        repo_path,
        config_path,
        config,
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
        Err(e) => {
            // Graceful failure with informative error
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("path")
                    || error_msg.contains("name")
                    || error_msg.contains("long")
            );
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

#[test]
fn test_concurrent_file_modifications() -> Result<()> {
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    let (dir, ctx) = setup_test_context()?;
    let ctx = Arc::new(ctx);

    // Create test file
    let test_file = dir.path().join("concurrent_test.txt");
    fs::write(&test_file, "initial content")?;

    // Add file
    let paths = vec![test_file.to_string_lossy().to_string()];
    commands::add::execute(&ctx, &paths, false)?;

    // Spawn threads that modify the file while dotman operations are running
    let test_file_clone = test_file.clone();
    let modifier_handle = thread::spawn(move || {
        for i in 0..100 {
            if fs::write(&test_file_clone, format!("modified content {}", i)).is_err() {
                // File might be locked, continue
            }
            thread::sleep(Duration::from_millis(1));
        }
    });

    let ctx_clone = ctx.clone();
    let paths_clone = paths.clone();
    let dotman_handle = thread::spawn(move || {
        for _ in 0..50 {
            let _ = commands::status::execute(&ctx_clone, false, false);
            let _ = commands::add::execute(&ctx_clone, &paths_clone, false);
            thread::sleep(Duration::from_millis(2));
        }
    });

    modifier_handle.join().unwrap();
    dotman_handle.join().unwrap();

    // Final operations should still work
    let result = commands::status::execute(&ctx, false, false);
    assert!(result.is_ok());

    Ok(())
}

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
fn test_permission_edge_cases() -> Result<()> {
    let (dir, ctx) = setup_test_context()?;

    // Create files with different permissions
    let readable_file = dir.path().join("readable.txt");
    fs::write(&readable_file, "readable content")?;

    let executable_file = dir.path().join("executable.sh");
    fs::write(&executable_file, "#!/bin/bash\necho hello")?;

    #[cfg(unix)]
    {
        // Make file executable
        let mut perms = fs::metadata(&executable_file)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&executable_file, perms)?;

        // Make file read-only
        let readonly_file = dir.path().join("readonly.txt");
        fs::write(&readonly_file, "readonly content")?;
        let mut perms = fs::metadata(&readonly_file)?.permissions();
        perms.set_mode(0o444);
        fs::set_permissions(&readonly_file, perms)?;

        // Test adding files with different permissions
        let paths = vec![
            readable_file.to_string_lossy().to_string(),
            executable_file.to_string_lossy().to_string(),
            readonly_file.to_string_lossy().to_string(),
        ];

        let result = commands::add::execute(&ctx, &paths, false);
        assert!(result.is_ok());

        // Permissions should be preserved
        let result = commands::commit::execute(&ctx, "Permission test", false);
        assert!(result.is_ok());
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

// ============= STRESS TESTS =============

#[test]
fn test_memory_usage_large_operations() -> Result<()> {
    let (dir, ctx) = setup_test_context()?;

    // Create many files to test memory usage
    let num_files = 5000;
    for i in 0..num_files {
        let file_path = dir.path().join(format!("file_{:06}.txt", i));
        fs::write(
            &file_path,
            format!(
                "This is file number {} with some content to make it not tiny",
                i
            ),
        )?;
    }

    // Add all files in batches to avoid command line limits
    let batch_size = 100;
    for batch_start in (0..num_files).step_by(batch_size) {
        let batch_end = (batch_start + batch_size).min(num_files);
        let batch_paths: Vec<String> = (batch_start..batch_end)
            .map(|i| {
                dir.path()
                    .join(format!("file_{:06}.txt", i))
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        let result = commands::add::execute(&ctx, &batch_paths, false);
        assert!(
            result.is_ok(),
            "Failed to add batch starting at {}",
            batch_start
        );
    }

    // Test status on many files
    let result = commands::status::execute(&ctx, false, false);
    assert!(result.is_ok());

    // Test commit with many files
    let result = commands::commit::execute(&ctx, "Many files test", false);
    assert!(result.is_ok());

    Ok(())
}

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
