use anyhow::Result;
use dotman::DotmanContext;
use dotman::commands;
use dotman::config::Config;
use dotman::storage::index::Index;
use serial_test::serial;
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

// ============= ADD COMMAND TESTS =============

#[test]
fn test_add_single_file() -> Result<()> {
    let (dir, ctx) = setup_test_context()?;

    // Create a test file
    let test_file = dir.path().join("test.txt");
    fs::write(&test_file, "test content")?;

    // Add the file
    let paths = vec![test_file.to_string_lossy().to_string()];
    commands::add::execute(&ctx, &paths, false)?;

    // Verify file was added to index
    let index_path = ctx.repo_path.join("index.bin");
    let index = Index::load(&index_path)?;
    assert_eq!(index.entries.len(), 1);
    assert!(index.get_entry(&test_file).is_some());

    Ok(())
}

#[test]
fn test_add_nonexistent_file_without_force() -> Result<()> {
    let (_dir, ctx) = setup_test_context()?;

    let paths = vec!["/nonexistent/file.txt".to_string()];
    let result = commands::add::execute(&ctx, &paths, false);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not exist"));

    Ok(())
}

#[test]
fn test_add_nonexistent_file_with_force() -> Result<()> {
    let (_dir, ctx) = setup_test_context()?;

    let paths = vec!["/nonexistent/file.txt".to_string()];
    let result = commands::add::execute(&ctx, &paths, true);

    assert!(result.is_ok()); // Should skip with warning, not error

    Ok(())
}

#[test]
fn test_add_directory_recursive() -> Result<()> {
    let (dir, ctx) = setup_test_context()?;

    // Create directory structure
    let test_dir = dir.path().join("testdir");
    fs::create_dir_all(&test_dir)?;
    fs::write(test_dir.join("file1.txt"), "content1")?;
    fs::write(test_dir.join("file2.txt"), "content2")?;

    let subdir = test_dir.join("subdir");
    fs::create_dir_all(&subdir)?;
    fs::write(subdir.join("file3.txt"), "content3")?;

    // Add the directory
    let paths = vec![test_dir.to_string_lossy().to_string()];
    commands::add::execute(&ctx, &paths, false)?;

    // Verify all files were added
    let index_path = ctx.repo_path.join("index.bin");
    let index = Index::load(&index_path)?;
    assert_eq!(index.entries.len(), 3);

    Ok(())
}

#[test]
fn test_add_already_tracked_file() -> Result<()> {
    let (dir, ctx) = setup_test_context()?;

    let test_file = dir.path().join("test.txt");
    fs::write(&test_file, "original content")?;

    // Add file first time
    let paths = vec![test_file.to_string_lossy().to_string()];
    commands::add::execute(&ctx, &paths, false)?;

    // Modify file
    fs::write(&test_file, "modified content")?;

    // Add file again
    commands::add::execute(&ctx, &paths, false)?;

    // Verify file was updated
    let index_path = ctx.repo_path.join("index.bin");
    let index = Index::load(&index_path)?;
    assert_eq!(index.entries.len(), 1); // Still just one entry

    Ok(())
}

#[test]
fn test_add_ignored_patterns() -> Result<()> {
    let (dir, mut ctx) = setup_test_context()?;

    // Set ignore patterns
    ctx.config.tracking.ignore_patterns = vec!["*.swp".to_string(), "*.tmp".to_string()];

    // Create files
    let good_file = dir.path().join("good.txt");
    let swap_file = dir.path().join("file.swp");
    let temp_file = dir.path().join("file.tmp");

    fs::write(&good_file, "content")?;
    fs::write(&swap_file, "swap")?;
    fs::write(&temp_file, "temp")?;

    // Create a directory with mixed files
    let test_dir = dir.path().join("testdir");
    fs::create_dir_all(&test_dir)?;
    fs::write(test_dir.join("good.txt"), "good")?;
    fs::write(test_dir.join("bad.swp"), "swap")?;

    // Add directory
    let paths = vec![test_dir.to_string_lossy().to_string()];
    commands::add::execute(&ctx, &paths, false)?;

    // Verify only non-ignored files were added
    let index_path = ctx.repo_path.join("index.bin");
    let index = Index::load(&index_path)?;
    assert_eq!(index.entries.len(), 1); // Only good.txt

    Ok(())
}

#[test]
fn test_add_empty_file() -> Result<()> {
    let (dir, ctx) = setup_test_context()?;

    let empty_file = dir.path().join("empty.txt");
    fs::write(&empty_file, "")?;

    let paths = vec![empty_file.to_string_lossy().to_string()];
    commands::add::execute(&ctx, &paths, false)?;

    let index_path = ctx.repo_path.join("index.bin");
    let index = Index::load(&index_path)?;
    assert_eq!(index.entries.len(), 1);

    let entry = index.get_entry(&empty_file).unwrap();
    assert_eq!(entry.size, 0);

    Ok(())
}

// ============= STATUS COMMAND TESTS =============

#[test]
fn test_status_empty_repo() -> Result<()> {
    let (_dir, ctx) = setup_test_context()?;

    // Status on empty repo should work
    let result = commands::status::execute(&ctx, false);
    assert!(result.is_ok());

    Ok(())
}

#[test]
fn test_status_clean_working_directory() -> Result<()> {
    let (dir, ctx) = setup_test_context()?;

    // Add and track a file
    let test_file = dir.path().join("test.txt");
    fs::write(&test_file, "content")?;

    let paths = vec![test_file.to_string_lossy().to_string()];
    commands::add::execute(&ctx, &paths, false)?;

    // Status should show no changes
    let result = commands::status::execute(&ctx, false);
    assert!(result.is_ok());

    Ok(())
}

// ============= COMMIT COMMAND TESTS =============

#[test]
fn test_commit_empty_index() -> Result<()> {
    let (_dir, ctx) = setup_test_context()?;

    // Try to commit with empty index
    let result = commands::commit::execute(&ctx, "Empty commit", false);
    assert!(result.is_ok()); // Should show warning but not error

    Ok(())
}

#[test]
fn test_commit_with_files() -> Result<()> {
    let (dir, ctx) = setup_test_context()?;

    // Add files
    let file1 = dir.path().join("file1.txt");
    let file2 = dir.path().join("file2.txt");
    fs::write(&file1, "content1")?;
    fs::write(&file2, "content2")?;

    let paths = vec![
        file1.to_string_lossy().to_string(),
        file2.to_string_lossy().to_string(),
    ];
    commands::add::execute(&ctx, &paths, false)?;

    // Commit
    let result = commands::commit::execute(&ctx, "Test commit", false);
    assert!(result.is_ok());

    // Verify HEAD was updated
    let head_path = ctx.repo_path.join("HEAD");
    assert!(head_path.exists());

    Ok(())
}

#[test]
fn test_commit_with_all_flag() -> Result<()> {
    let (dir, ctx) = setup_test_context()?;

    // Add a file
    let test_file = dir.path().join("test.txt");
    fs::write(&test_file, "original")?;

    let paths = vec![test_file.to_string_lossy().to_string()];
    commands::add::execute(&ctx, &paths, false)?;

    // Modify the file
    fs::write(&test_file, "modified")?;

    // Commit with --all flag
    let result = commands::commit::execute(&ctx, "Commit all changes", true);
    assert!(result.is_ok());

    Ok(())
}

// ============= INIT COMMAND TESTS =============

#[test]
#[serial]
fn test_init_new_repo() -> Result<()> {
    let dir = tempdir()?;

    // Change HOME to temp dir for this test
    unsafe {
        std::env::set_var("HOME", dir.path());
    }

    let result = commands::init::execute(false);
    assert!(result.is_ok());

    // Verify repository structure was created
    let repo_path = dir.path().join(".dotman");
    assert!(repo_path.exists());
    assert!(repo_path.join("commits").exists());
    assert!(repo_path.join("objects").exists());
    assert!(repo_path.join("index.bin").exists());

    Ok(())
}

#[test]
#[serial]
fn test_init_already_exists() -> Result<()> {
    let dir = tempdir()?;
    unsafe {
        std::env::set_var("HOME", dir.path());
    }

    // Initialize once
    commands::init::execute(false)?;

    // Try to initialize again
    let result = commands::init::execute(false);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("already initialized")
    );

    Ok(())
}

// ============= CHECKOUT COMMAND TESTS =============

#[test]
fn test_checkout_nonexistent_commit() -> Result<()> {
    let (_dir, ctx) = setup_test_context()?;

    // Use force flag to bypass uncommitted changes check and test actual commit loading
    let result = commands::checkout::execute(&ctx, "nonexistent", true);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Failed to load commit"),
        "Expected error message to contain 'Failed to load commit', got: {}",
        err_msg
    );

    Ok(())
}

// ============= RESET COMMAND TESTS =============

#[test]
fn test_reset_no_commits() -> Result<()> {
    let (_dir, ctx) = setup_test_context()?;

    let result = commands::reset::execute(&ctx, "HEAD", false, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("No commits yet"));

    Ok(())
}

#[test]
fn test_reset_hard_soft_conflict() -> Result<()> {
    let (_dir, ctx) = setup_test_context()?;

    let result = commands::reset::execute(&ctx, "HEAD", true, true);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Cannot use both"));

    Ok(())
}

// ============= LOG COMMAND TESTS =============

#[test]
fn test_log_empty_repo() -> Result<()> {
    let (_dir, ctx) = setup_test_context()?;

    let result = commands::log::execute(&ctx, 10, false);
    assert!(result.is_ok()); // Should show "No commits yet"

    Ok(())
}

// ============= SHOW COMMAND TESTS =============

#[test]
fn test_show_invalid_commit() -> Result<()> {
    let (_dir, ctx) = setup_test_context()?;

    let result = commands::show::execute(&ctx, "invalid");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Failed to load object")
    );

    Ok(())
}

// ============= DIFF COMMAND TESTS =============

#[test]
fn test_diff_working_vs_index() -> Result<()> {
    let (_dir, ctx) = setup_test_context()?;

    // Diff with no arguments compares working dir to index
    let result = commands::diff::execute(&ctx, None, None);
    assert!(result.is_ok());

    Ok(())
}

// ============= RM COMMAND TESTS =============

#[test]
fn test_rm_untracked_file() -> Result<()> {
    let (dir, ctx) = setup_test_context()?;

    let test_file = dir.path().join("test.txt");
    fs::write(&test_file, "content")?;

    // Try to remove untracked file
    let paths = vec![test_file.to_string_lossy().to_string()];
    let result = commands::rm::execute(&ctx, &paths, false, false);
    assert!(result.is_ok()); // Should warn but not error

    Ok(())
}

#[test]
fn test_rm_tracked_file() -> Result<()> {
    let (dir, ctx) = setup_test_context()?;

    let test_file = dir.path().join("test.txt");
    fs::write(&test_file, "content")?;

    // Add file first
    let paths = vec![test_file.to_string_lossy().to_string()];
    commands::add::execute(&ctx, &paths, false)?;

    // Remove from tracking
    let result = commands::rm::execute(&ctx, &paths, true, false); // --cached
    assert!(result.is_ok());

    // Verify file still exists on disk
    assert!(test_file.exists());

    // Verify file was removed from index
    let index_path = ctx.repo_path.join("index.bin");
    let index = Index::load(&index_path)?;
    assert_eq!(index.entries.len(), 0);

    Ok(())
}

// ============= PUSH/PULL COMMAND TESTS =============

#[test]
fn test_push_no_remote() -> Result<()> {
    let (_dir, ctx) = setup_test_context()?;

    let result = commands::push::execute(&ctx, "origin", "main");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("No remote configured")
    );

    Ok(())
}

#[test]
fn test_pull_no_remote() -> Result<()> {
    let (_dir, ctx) = setup_test_context()?;

    let result = commands::pull::execute(&ctx, "origin", "main");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("No remote configured")
    );

    Ok(())
}

// ============= SYMLINK TESTS =============

#[test]
#[cfg(unix)]
fn test_add_symlink() -> Result<()> {
    let (dir, ctx) = setup_test_context()?;

    let target = dir.path().join("target.txt");
    let symlink = dir.path().join("link.txt");

    fs::write(&target, "target content")?;
    std::os::unix::fs::symlink(&target, &symlink)?;

    // Add symlink
    let paths = vec![symlink.to_string_lossy().to_string()];
    let result = commands::add::execute(&ctx, &paths, false);

    // Should handle symlink appropriately based on follow_symlinks config
    assert!(result.is_ok());

    Ok(())
}

// ============= PERMISSION TESTS =============

#[test]
#[cfg(unix)]
fn test_add_no_read_permission() -> Result<()> {
    let (dir, ctx) = setup_test_context()?;

    let test_file = dir.path().join("noperm.txt");
    fs::write(&test_file, "content")?;

    // Remove read permission
    let mut perms = fs::metadata(&test_file)?.permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&test_file, perms)?;

    // Try to add file
    let paths = vec![test_file.to_string_lossy().to_string()];
    let result = commands::add::execute(&ctx, &paths, false);

    // Should fail due to permission denied
    assert!(result.is_err());

    // Restore permissions for cleanup
    let mut perms = fs::metadata(&test_file)?.permissions();
    perms.set_mode(0o644);
    fs::set_permissions(&test_file, perms)?;

    Ok(())
}

// ============= LARGE SCALE TESTS =============

#[test]
fn test_add_many_files() -> Result<()> {
    let (dir, ctx) = setup_test_context()?;

    // Create 100 files
    let mut paths = Vec::new();
    for i in 0..100 {
        let file = dir.path().join(format!("file_{}.txt", i));
        fs::write(&file, format!("content {}", i))?;
        paths.push(file.to_string_lossy().to_string());
    }

    // Add all files
    let result = commands::add::execute(&ctx, &paths, false);
    assert!(result.is_ok());

    // Verify all were added
    let index_path = ctx.repo_path.join("index.bin");
    let index = Index::load(&index_path)?;
    assert_eq!(index.entries.len(), 100);

    Ok(())
}
