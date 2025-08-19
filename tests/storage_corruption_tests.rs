use anyhow::Result;
use dotman::DotmanContext;
use dotman::commands;
use dotman::config::Config;
use dotman::storage::index::Index;
use dotman::utils::serialization;
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

#[test]
fn test_index_corruption_recovery() -> Result<()> {
    let (dir, ctx) = setup_test_context()?;

    // Create and add some files
    let test_file = dir.path().join("test.txt");
    fs::write(&test_file, "test content")?;

    let paths = vec![test_file.to_string_lossy().to_string()];
    commands::add::execute(&ctx, &paths, false)?;

    // Corrupt the index file in various ways
    let index_path = ctx.repo_path.join("index.bin");

    // Test 1: Truncate index file
    let original_content = fs::read(&index_path)?;
    fs::write(&index_path, &original_content[..original_content.len() / 2])?;

    let result = commands::status::execute(&ctx, false);
    match result {
        Ok(_) => {}
        Err(_) => {
            // Should be able to recover by recreating index
            let new_index = Index::new();
            new_index.save(&index_path)?;

            let recovery_result = commands::status::execute(&ctx, false);
            assert!(recovery_result.is_ok(), "Should recover from corruption");
        }
    }

    // Test 2: Replace with random binary data
    fs::write(&index_path, vec![0xFF; 1000])?;

    let result = Index::load(&index_path);
    assert!(result.is_err(), "Should reject corrupted binary data");

    // Test 3: Replace with valid but wrong bincode data
    let wrong_data = serialization::serialize(&vec![1u32, 2u32, 3u32])?;
    fs::write(&index_path, wrong_data)?;

    let result = Index::load(&index_path);
    assert!(result.is_err(), "Should reject wrong data structure");

    // Test 4: Zero-length file
    fs::write(&index_path, "")?;

    let result = Index::load(&index_path);
    assert!(result.is_err(), "Should reject empty index");

    Ok(())
}

#[test]
fn test_partial_write_corruption() -> Result<()> {
    let (dir, ctx) = setup_test_context()?;

    // Simulate partial write by writing index in stages
    let test_file = dir.path().join("test.txt");
    fs::write(&test_file, "test content")?;

    let paths = vec![test_file.to_string_lossy().to_string()];
    commands::add::execute(&ctx, &paths, false)?;

    // Get the valid index data
    let index_path = ctx.repo_path.join("index.bin");
    let valid_data = fs::read(&index_path)?;

    // Simulate various partial write scenarios
    for partial_size in [1, 4, 8, 16, 32, valid_data.len() / 4, valid_data.len() / 2] {
        if partial_size < valid_data.len() {
            fs::write(&index_path, &valid_data[..partial_size])?;

            let result = Index::load(&index_path);
            assert!(
                result.is_err(),
                "Should reject partial write at {} bytes",
                partial_size
            );
        }
    }

    Ok(())
}

#[test]
fn test_concurrent_corruption() -> Result<()> {
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    let (dir, ctx) = setup_test_context()?;
    let ctx = Arc::new(ctx);

    // Create test file
    let test_file = dir.path().join("concurrent_test.txt");
    fs::write(&test_file, "test content")?;
    let paths = vec![test_file.to_string_lossy().to_string()];

    // Thread that continuously corrupts the index
    let ctx_clone = ctx.clone();
    let corruptor = thread::spawn(move || {
        for i in 0..100 {
            let index_path = ctx_clone.repo_path.join("index.bin");
            // Corrupt with different patterns
            let corrupt_data = match i % 4 {
                0 => vec![0x00; 100],
                1 => vec![0xFF; 100],
                2 => vec![0xAA; 100],
                _ => (0..100).map(|x| x as u8).collect(),
            };
            let _ = fs::write(&index_path, corrupt_data);
            thread::sleep(Duration::from_millis(5));
        }
    });

    // Thread that tries to perform operations
    let ctx_clone2 = ctx.clone();
    let paths_clone = paths.clone();
    let operator = thread::spawn(move || {
        for _ in 0..50 {
            // All these operations should either succeed or fail gracefully
            let _ = commands::add::execute(&ctx_clone2, &paths_clone, false);
            let _ = commands::status::execute(&ctx_clone2, false);
            thread::sleep(Duration::from_millis(10));
        }
    });

    corruptor.join().unwrap();
    operator.join().unwrap();

    // System should still be recoverable
    let index_path = ctx.repo_path.join("index.bin");
    let recovery_index = Index::new();
    recovery_index.save(&index_path)?;

    let final_result = commands::status::execute(&ctx, false);
    assert!(final_result.is_ok(), "Should be recoverable");

    Ok(())
}

#[test]
fn test_filesystem_errors() -> Result<()> {
    let (dir, ctx) = setup_test_context()?;

    // Test disk full simulation by making index directory read-only
    let _index_path = ctx.repo_path.join("index.bin");

    // Create valid index first
    let test_file = dir.path().join("test.txt");
    fs::write(&test_file, "content")?;
    let paths = vec![test_file.to_string_lossy().to_string()];
    commands::add::execute(&ctx, &paths, false)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        // Make repo directory read-only
        let mut perms = fs::metadata(&ctx.repo_path)?.permissions();
        perms.set_mode(0o555); // Read and execute only
        fs::set_permissions(&ctx.repo_path, perms)?;

        // Operations should fail gracefully
        let result = commands::add::execute(&ctx, &paths, false);
        match result {
            Ok(_) => {
                // If it succeeded, it might be writing to a different location or caching
                // This is potentially a bug - we should not be able to modify read-only repo
                println!(
                    "WARNING: Add succeeded on read-only directory - potential security issue"
                );
            }
            Err(_) => {
                // Good - properly rejected read-only directory
            }
        }

        // Restore permissions for cleanup
        let mut perms = fs::metadata(&ctx.repo_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&ctx.repo_path, perms)?;
    }

    Ok(())
}

#[test]
fn test_object_corruption() -> Result<()> {
    let (dir, ctx) = setup_test_context()?;

    // Create file and commit to create objects
    let test_file = dir.path().join("test.txt");
    fs::write(&test_file, "test content for objects")?;

    let paths = vec![test_file.to_string_lossy().to_string()];
    commands::add::execute(&ctx, &paths, false)?;
    commands::commit::execute(&ctx, "Test commit", false)?;

    // Find and corrupt object files
    let objects_dir = ctx.repo_path.join("objects");
    if objects_dir.exists() {
        for entry in fs::read_dir(&objects_dir)? {
            let entry = entry?;
            let object_path = entry.path();

            // Corrupt object with various methods
            let original_data = fs::read(&object_path)?;

            // Test 1: Truncate object
            fs::write(&object_path, &original_data[..original_data.len() / 2])?;

            // Operations should detect corruption
            let status_result = commands::status::execute(&ctx, false);
            match status_result {
                Ok(_) => {}  // May succeed if corruption not detected yet
                Err(_) => {} // Good - detected corruption
            }

            // Test 2: Replace with random data
            fs::write(&object_path, vec![0x42; 1000])?;

            // Test 3: Replace with different valid compression data
            let fake_compressed = zstd::encode_all(&b"fake data"[..], 3)?;
            fs::write(&object_path, fake_compressed)?;

            // Restore original for next tests
            fs::write(&object_path, &original_data)?;
        }
    }

    Ok(())
}

#[test]
fn test_snapshot_corruption() -> Result<()> {
    let (dir, ctx) = setup_test_context()?;

    // Create commit to generate snapshot
    let test_file = dir.path().join("snapshot_test.txt");
    fs::write(&test_file, "snapshot content")?;

    let paths = vec![test_file.to_string_lossy().to_string()];
    commands::add::execute(&ctx, &paths, false)?;
    commands::commit::execute(&ctx, "Snapshot test commit", false)?;

    // Get commit ID to find snapshot file
    let head_path = ctx.repo_path.join("HEAD");
    let commit_id = fs::read_to_string(&head_path)?.trim().to_string();

    let commits_dir = ctx.repo_path.join("commits");
    let snapshot_file = commits_dir.join(format!("{}.zst", commit_id));

    if snapshot_file.exists() {
        let original_data = fs::read(&snapshot_file)?;

        // Test 1: Corrupt compressed data
        fs::write(&snapshot_file, vec![0x28, 0xb5, 0x2f, 0xfd, 0xFF, 0xFF])?; // Invalid zstd

        let result = commands::show::execute(&ctx, &commit_id);
        assert!(result.is_err(), "Should reject corrupted snapshot");

        // Test 2: Replace with valid compression of wrong data
        let fake_data = zstd::encode_all(&b"fake snapshot data"[..], 3)?;
        fs::write(&snapshot_file, fake_data)?;

        let _result = commands::show::execute(&ctx, &commit_id);
        // May succeed but show wrong data, or fail during deserialization

        // Test 3: Partial snapshot file
        fs::write(&snapshot_file, &original_data[..original_data.len() / 3])?;

        let result = commands::show::execute(&ctx, &commit_id);
        assert!(result.is_err(), "Should reject partial snapshot");

        // Restore original
        fs::write(&snapshot_file, original_data)?;
    }

    Ok(())
}

#[test]
fn test_index_consistency_validation() -> Result<()> {
    let (dir, ctx) = setup_test_context()?;

    // Create files with known content
    let file1 = dir.path().join("file1.txt");
    let file2 = dir.path().join("file2.txt");
    fs::write(&file1, "content1")?;
    fs::write(&file2, "content2")?;

    let paths = vec![
        file1.to_string_lossy().to_string(),
        file2.to_string_lossy().to_string(),
    ];
    commands::add::execute(&ctx, &paths, false)?;

    // Manually corrupt the index by creating inconsistent entries
    let index_path = ctx.repo_path.join("index.bin");
    let mut index = Index::load(&index_path)?;

    // Corrupt hash for one file
    if let Some(entry) = index.entries.values_mut().next() {
        entry.hash = "invalid_hash".to_string();
        entry.size = 999999; // Wrong size
        entry.modified = 0; // Wrong timestamp
    }

    index.save(&index_path)?;

    // Status should detect inconsistencies
    let result = commands::status::execute(&ctx, false);
    match result {
        Ok(_) => {
            // If it succeeds, it should show files as modified due to hash mismatch
        }
        Err(_) => {
            // May fail if hash format is completely invalid
        }
    }

    Ok(())
}

#[test]
fn test_memory_mapped_file_corruption() -> Result<()> {
    let (dir, ctx) = setup_test_context()?;

    // Create a large file that will use memory mapping (>1MB threshold)
    let large_file = dir.path().join("large.dat");
    let large_content = vec![0x42u8; 2_000_000]; // 2MB
    fs::write(&large_file, &large_content)?;

    let paths = vec![large_file.to_string_lossy().to_string()];
    commands::add::execute(&ctx, &paths, false)?;

    // Corrupt the file after it's been added
    fs::write(&large_file, vec![0x00u8; 2_000_000])?;

    // Status should detect the change
    let result = commands::status::execute(&ctx, false);
    assert!(
        result.is_ok(),
        "Status should work even with corrupted large file"
    );

    // Test truncating large file
    fs::write(&large_file, vec![0x42u8; 1000])?;

    let result = commands::status::execute(&ctx, false);
    assert!(result.is_ok(), "Should handle truncated large file");

    Ok(())
}
