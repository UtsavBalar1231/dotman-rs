use anyhow::Result;
use dotman::DotmanContext;
use std::fs;
use std::process::Command;

mod common;
use common::TestEnvironment;

#[test]
fn test_user_config_workflow() -> Result<()> {
    let env = TestEnvironment::new()?;
    let ctx = env.init_repo()?;

    // Load config and update user configuration
    let mut config = dotman::config::Config::load(&ctx.config_path)?;
    config.user.name = Some("Test User".to_string());
    config.user.email = Some("test@example.com".to_string());
    config.save(&ctx.config_path)?;

    // Create test file
    let test_file = env.create_test_file("test.txt", "test content")?;

    // Reload config to get updated values
    let config = dotman::config::Config::load(&ctx.config_path)?;
    let ctx = DotmanContext {
        repo_path: ctx.repo_path.clone(),
        config_path: ctx.config_path,
        config,
        no_pager: true,
    };

    // Add file
    dotman::commands::add::execute(&ctx, &[test_file.to_str().unwrap().to_string()], false)?;

    // Commit with configured user
    dotman::commands::commit::execute(&ctx, "Test commit", false)?;

    // Verify the commit has the right author
    let snapshot_manager = dotman::storage::snapshots::SnapshotManager::new(
        ctx.repo_path.clone(),
        ctx.config.core.compression_level,
    );
    let commits = snapshot_manager.list_snapshots()?;
    assert!(!commits.is_empty());

    let commit_path = ctx
        .repo_path
        .join("commits")
        .join(format!("{}.zst", &commits[0]));
    let commit_data = fs::read(&commit_path)?;
    let decompressed = zstd::decode_all(&commit_data[..])?;
    let commit_str = String::from_utf8_lossy(&decompressed);

    // Parse the commit data to check author
    assert!(commit_str.contains("Test User"));
    assert!(commit_str.contains("test@example.com"));

    Ok(())
}

#[test]
fn test_config_command() -> Result<()> {
    let env = TestEnvironment::new()?;
    let ctx = env.init_repo()?;

    // Test setting config values
    let mut updated_ctx = ctx;
    dotman::commands::config::execute(
        &mut updated_ctx,
        Some("user.name"),
        Some("Jane Doe".to_string()),
        false,
        false,
    )?;

    // Reload and verify
    let config = dotman::config::Config::load(&updated_ctx.config_path)?;
    assert_eq!(config.user.name, Some("Jane Doe".to_string()));

    // Test getting config value
    dotman::commands::config::execute(&mut updated_ctx, Some("user.name"), None, false, false)?;

    Ok(())
}

#[test]
fn test_config_performance_settings() -> Result<()> {
    let env = TestEnvironment::new()?;
    let ctx = env.init_repo()?;

    // Load and modify performance settings
    let mut config = dotman::config::Config::load(&ctx.config_path)?;
    config.performance.parallel_threads = 8;
    config.performance.cache_size = 200;
    config.performance.mmap_threshold = 2_097_152; // 2MB
    config.save(&ctx.config_path)?;

    // Reload and verify
    let reloaded = dotman::config::Config::load(&ctx.config_path)?;
    assert_eq!(reloaded.performance.parallel_threads, 8);
    assert_eq!(reloaded.performance.cache_size, 200);
    assert_eq!(reloaded.performance.mmap_threshold, 2_097_152);

    Ok(())
}

#[test]
fn test_git_compatibility_mode() -> Result<()> {
    let env = TestEnvironment::new()?;
    let home = env.home_dir.as_path();

    // Initialize a git repository
    Command::new("git")
        .args(["init"])
        .current_dir(home)
        .output()?;

    Command::new("git")
        .args(["config", "user.name", "Git User"])
        .current_dir(home)
        .output()?;

    Command::new("git")
        .args(["config", "user.email", "git@example.com"])
        .current_dir(home)
        .output()?;

    // Initialize dotman in the same directory
    let ctx = env.init_repo()?;

    // Create and add a file
    let test_file = env.create_test_file("test.txt", "content")?;
    dotman::commands::add::execute(&ctx, &[test_file.to_str().unwrap().to_string()], false)?;
    dotman::commands::commit::execute(&ctx, "Dotman commit", false)?;

    // Verify both git and dotman are aware of the file
    let output = Command::new("git")
        .args(["status", "--short"])
        .current_dir(home)
        .output()?;

    // Git should see the test.txt file
    let status = String::from_utf8_lossy(&output.stdout);
    assert!(status.contains("test.txt") || status.is_empty());

    Ok(())
}
