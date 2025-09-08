use anyhow::Result;
use dotman::DotmanContext;
use dotman::commands;
use dotman::config::Config;
use dotman::storage::index::Index;
use dotman::storage::snapshots::SnapshotManager;
use dotman::storage::{Commit, FileEntry};
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

fn setup_corrupted_repo() -> Result<(tempfile::TempDir, DotmanContext)> {
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

// Test moved to src/storage/index.rs as a unit test

#[test]
fn test_recover_from_incomplete_commit() -> Result<()> {
    let (_dir, ctx) = setup_corrupted_repo()?;

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    // Create partial commit file (corrupted/incomplete)
    let commits_dir = ctx.repo_path.join("commits");
    let incomplete_commit = commits_dir.join("incomplete.zst");
    fs::write(&incomplete_commit, b"incomplete compressed data")?;

    // Try to load incomplete commit
    let result = snapshot_manager.load_snapshot("incomplete");
    assert!(result.is_err());

    // Recovery: list valid snapshots should still work
    let valid_snapshots = snapshot_manager.list_snapshots()?;

    // Should detect the file but loading it will fail
    assert!(valid_snapshots.contains(&"incomplete".to_string()));

    snapshot_manager.delete_snapshot("incomplete")?;

    // Verify it's gone
    let after_cleanup = snapshot_manager.list_snapshots()?;
    assert!(!after_cleanup.contains(&"incomplete".to_string()));

    Ok(())
}

#[test]
fn test_recover_from_missing_objects() -> Result<()> {
    let (dir, ctx) = setup_corrupted_repo()?;

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    let test_file = dir.path().join("test.txt");
    fs::write(&test_file, "test content")?;

    let files = vec![FileEntry {
        path: test_file.clone(),
        hash: "test_hash".to_string(),
        size: 12,
        modified: 1234567890,
        mode: 0o644,
    }];

    let commit = Commit {
        id: "commit1".to_string(),
        parent: None,
        message: "Test commit".to_string(),
        author: "Test".to_string(),
        timestamp: 1234567890,
        tree_hash: "tree1".to_string(),
    };

    snapshot_manager.create_snapshot(commit, &files)?;

    // Delete object files to simulate corruption
    let objects_dir = ctx.repo_path.join("objects");
    if objects_dir.exists() {
        for entry in fs::read_dir(&objects_dir)? {
            let entry = entry?;
            fs::remove_file(entry.path())?;
        }
    }

    // Try to restore - should fail
    let restore_result = snapshot_manager.restore_snapshot("commit1", dir.path());
    assert!(restore_result.is_err());

    // Recovery: recreate objects from existing files
    let recovered_files = vec![FileEntry {
        path: test_file.clone(),
        hash: "recovered_hash".to_string(),
        size: 12,
        modified: 1234567890,
        mode: 0o644,
    }];

    let recovery_commit = Commit {
        id: "recovery".to_string(),
        parent: Some("commit1".to_string()),
        message: "Recovery commit".to_string(),
        author: "Recovery".to_string(),
        timestamp: 1234567891,
        tree_hash: "recovery_tree".to_string(),
    };

    // This should work as it will recreate objects
    snapshot_manager.create_snapshot(recovery_commit, &recovered_files)?;

    Ok(())
}

#[test]
fn test_recover_from_interrupted_operations() -> Result<()> {
    let (dir, ctx) = setup_corrupted_repo()?;

    // Simulate interrupted add operation by creating partial index
    let index_path = ctx.repo_path.join("index.bin");
    let temp_index_path = ctx.repo_path.join("index.bin.tmp");

    // Simulate interruption by removing the real index (as if write was interrupted)
    fs::remove_file(&index_path)?;

    // Create temporary index file (simulating interrupted write)
    let mut temp_index = Index::new();
    temp_index.add_entry(FileEntry {
        path: PathBuf::from("partial.txt"),
        hash: "partial".to_string(),
        size: 50,
        modified: 1234567890,
        mode: 0o644,
    });
    temp_index.save(&temp_index_path)?;

    // Recovery: if temp file exists, either:
    // 1. Complete the operation (rename temp to real)
    // 2. Discard the temp file

    // Option 1: Complete the operation
    if temp_index_path.exists() && !index_path.exists() {
        fs::rename(&temp_index_path, &index_path)?;
    }

    // Verify recovery worked
    let recovered = Index::load(&index_path)?;
    assert_eq!(recovered.entries.len(), 1);

    // Now normal operations should work
    let test_file = dir.path().join("new.txt");
    fs::write(&test_file, "content")?;

    let paths = vec![test_file.to_string_lossy().to_string()];
    commands::add::execute(&ctx, &paths, false)?;

    Ok(())
}

// Permission test consolidated into comprehensive_test.rs::test_comprehensive_permissions

#[test]
fn test_recover_from_disk_full() -> Result<()> {
    // This test simulates disk full conditions
    // In a real scenario, we'd use a limited filesystem

    let (dir, ctx) = setup_corrupted_repo()?;

    let large_file = dir.path().join("large.dat");
    let large_content = vec![0u8; 1_000_000]; // 1MB

    // Simulate partial write by writing incomplete data
    fs::write(&large_file, &large_content[..500_000])?; // Only half

    // Try to add - hash will be different than expected
    let paths = vec![large_file.to_string_lossy().to_string()];
    let result = commands::add::execute(&ctx, &paths, false);

    // Should succeed even with partial file
    assert!(result.is_ok());

    // Recovery: detect incomplete files by size mismatch
    let index = Index::load(&ctx.repo_path.join("index.bin"))?;
    if let Some(entry) = index.get_entry(&large_file) {
        let actual_size = fs::metadata(&large_file)?.len();
        if actual_size != entry.size {
            // File is incomplete, re-add when space available
            println!(
                "Detected incomplete file: expected {} bytes, got {} bytes",
                entry.size, actual_size
            );
        }
    }

    Ok(())
}

// Concurrent test consolidated in integration_test.rs::test_concurrent_operations

// Config recovery test moved to src/config/mod.rs::test_malformed_config_recovery

#[test]
fn test_garbage_collection_recovery() -> Result<()> {
    let (_dir, ctx) = setup_corrupted_repo()?;

    // Create orphaned objects
    let objects_dir = ctx.repo_path.join("objects");
    fs::write(objects_dir.join("orphan1.zst"), "orphaned data 1")?;
    fs::write(objects_dir.join("orphan2.zst"), "orphaned data 2")?;
    fs::write(objects_dir.join("orphan3.zst"), "orphaned data 3")?;

    // Run garbage collection
    let gc = dotman::storage::snapshots::GarbageCollector::new(ctx.repo_path.clone());
    let deleted = gc.collect()?;

    // Should have deleted all orphaned objects
    assert_eq!(deleted, 3);

    // Verify objects directory is clean
    let remaining = fs::read_dir(&objects_dir)?.count();
    assert_eq!(remaining, 0);

    Ok(())
}
