use anyhow::Result;
use dotman::DotmanContext;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
#[serial_test::serial]
fn test_user_config_workflow() -> Result<()> {
    let temp = tempdir()?;
    let home = temp.path().join("home");
    let repo_path = home.join(".dotman");

    fs::create_dir_all(&home)?;

    // Set HOME for the test
    unsafe {
        std::env::set_var("HOME", &home);
    }

    dotman::commands::init::execute(false)?;

    // Load config and create context
    let config_path = home.join(".config/dotman/config");
    let mut config = dotman::config::Config::load(&config_path)?;

    // Set user configuration
    config.user.name = Some("Test User".to_string());
    config.user.email = Some("test@example.com".to_string());
    config.save(&config_path)?;

    let test_file = home.join("test.txt");
    fs::write(&test_file, "test content")?;

    // Reload config to get updated values
    let config = dotman::config::Config::load(&config_path)?;
    let ctx = DotmanContext {
        repo_path: repo_path.clone(),
        config_path: config_path.clone(),
        config: config.clone(),
        no_pager: true,
    };

    // Add file
    dotman::commands::add::execute(&ctx, &[test_file.to_str().unwrap().to_string()], false)?;

    // Commit with configured user
    dotman::commands::commit::execute(&ctx, "Test commit", false)?;

    // Verify the commit has the right author
    let head_commit = dotman::refs::RefManager::new(repo_path.clone())
        .get_head_commit()?
        .unwrap();

    let snapshot_manager = dotman::storage::snapshots::SnapshotManager::new(
        repo_path.clone(),
        config.core.compression_level,
    );
    let snapshot = snapshot_manager.load_snapshot(&head_commit)?;

    assert_eq!(snapshot.commit.author, "Test User <test@example.com>");

    Ok(())
}

#[test]
#[serial_test::serial]
fn test_config_command_integration() -> Result<()> {
    let temp = tempdir()?;
    let home = temp.path().join("home");
    let repo_path = home.join(".dotman");

    fs::create_dir_all(&home)?;

    // Set HOME for the test
    unsafe {
        std::env::set_var("HOME", &home);
    }

    dotman::commands::init::execute(false)?;

    // Load config and create context
    let config_path = home.join(".config/dotman/config");
    let config = dotman::config::Config::load(&config_path)?;
    let mut ctx = DotmanContext {
        repo_path,
        config_path: config_path.clone(),
        config,
        no_pager: true,
    };

    // Use config command to set user.name
    dotman::commands::config::execute(
        &mut ctx,
        Some("user.name"),
        Some("Jane Doe".to_string()),
        false,
        false,
    )?;

    // Use config command to set user.email
    dotman::commands::config::execute(
        &mut ctx,
        Some("user.email"),
        Some("jane@example.com".to_string()),
        false,
        false,
    )?;

    // Reload config and verify
    let config = dotman::config::Config::load(&config_path)?;
    assert_eq!(config.user.name, Some("Jane Doe".to_string()));
    assert_eq!(config.user.email, Some("jane@example.com".to_string()));

    // Test unsetting
    ctx.config = config;
    dotman::commands::config::execute(&mut ctx, Some("user.name"), None, true, false)?;

    let config = dotman::config::Config::load(&config_path)?;
    assert_eq!(config.user.name, None);
    assert_eq!(config.user.email, Some("jane@example.com".to_string()));

    Ok(())
}

#[test]
#[serial_test::serial]
fn test_mirror_uses_dotman_config() -> Result<()> {
    let temp = tempdir()?;
    let home = temp.path().join("home");
    let repo_path = home.join(".dotman");

    fs::create_dir_all(&home)?;

    // Set HOME for the test
    unsafe {
        std::env::set_var("HOME", &home);
    }

    dotman::commands::init::execute(false)?;

    // Load config and set user info
    let config_path = home.join(".config/dotman/config");
    let mut config = dotman::config::Config::load(&config_path)?;
    config.user.name = Some("Mirror Test User".to_string());
    config.user.email = Some("mirror@test.com".to_string());
    config.save(&config_path)?;

    let mirror = dotman::mirror::GitMirror::new(
        &repo_path,
        "test-remote",
        "https://example.com/repo.git",
        config.clone(),
    );

    mirror.init_mirror()?;

    let mirror_path = repo_path.join("mirrors/test-remote");

    let output = Command::new("git")
        .args(["config", "user.name"])
        .current_dir(&mirror_path)
        .output()?;
    let git_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(git_name, "Mirror Test User");

    let output = Command::new("git")
        .args(["config", "user.email"])
        .current_dir(&mirror_path)
        .output()?;
    let git_email = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(git_email, "mirror@test.com");

    Ok(())
}
