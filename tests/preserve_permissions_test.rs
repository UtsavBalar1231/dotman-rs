use anyhow::Result;
use dotman::DotmanContext;
use dotman::config::Config;
use std::fs;

mod common;
use common::TestEnvironment;

#[test]
fn test_preserve_permissions_config() -> Result<()> {
    let env = TestEnvironment::new()?;
    let repo_path = env.repo_dir.clone();
    let config_path = env.home_dir.join("config.toml");

    // Create a config with preserve_permissions = false
    let mut config = Config::default();
    config.tracking.preserve_permissions = false;
    config.save(&config_path)?;

    // Create context
    let ctx = DotmanContext {
        repo_path,
        config_path: config_path.clone(),
        config: config.clone(),
        no_pager: true,
    };

    // Initialize repository
    ctx.ensure_repo_exists()?;

    // Verify config was loaded with preserve_permissions = false
    assert!(!ctx.config.tracking.preserve_permissions);

    // Set it to true and save
    let mut new_config = config;
    new_config.tracking.preserve_permissions = true;
    new_config.save(&config_path)?;

    // Reload config
    let reloaded_config = Config::load(&config_path)?;
    assert!(reloaded_config.tracking.preserve_permissions);

    Ok(())
}

#[test]
#[cfg(unix)]
fn test_permissions_preserved_on_unix() -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let env = TestEnvironment::new()?;
    let home = &env.home_dir;
    let repo_path = env.repo_dir.clone();
    let config_path = env.home_dir.join("config.toml");

    // Create config with preserve_permissions = true
    let mut config = Config::default();
    config.tracking.preserve_permissions = true;
    config.core.repo_path = repo_path.clone();
    config.save(&config_path)?;

    let ctx = DotmanContext {
        repo_path: repo_path.clone(),
        config_path,
        config,
        no_pager: true,
    };

    // Initialize the repository (don't use the global init command)
    ctx.ensure_repo_exists()?;
    let index = dotman::storage::index::Index::new();
    index.save(&ctx.repo_path.join(dotman::INDEX_FILE))?;
    let ref_manager = dotman::refs::RefManager::new(ctx.repo_path.clone());
    ref_manager.init()?;

    // Create a test file with specific permissions
    let test_file = home.join("test_perms.sh");
    fs::write(&test_file, "#!/bin/bash\necho 'test'")?;

    // Make it executable (0o755)
    let perms = fs::Permissions::from_mode(0o755);
    fs::set_permissions(&test_file, perms)?;

    // Verify permissions were set
    let metadata = fs::metadata(&test_file)?;
    assert_eq!(metadata.permissions().mode() & 0o777, 0o755);

    // Add the file to dotman
    dotman::commands::add::execute(&ctx, &[test_file.to_string_lossy().to_string()], false)?;

    // Create a commit
    dotman::commands::commit::execute(&ctx, "Test permissions", false)?;

    // Delete the original file
    fs::remove_file(&test_file)?;
    assert!(!test_file.exists());

    // Restore from the latest commit
    let commit_id = fs::read_to_string(repo_path.join("HEAD"))?;
    dotman::commands::checkout::execute(&ctx, &commit_id, true)?;

    // Check if file was restored with correct permissions
    assert!(test_file.exists());
    let restored_metadata = fs::metadata(&test_file)?;
    let restored_mode = restored_metadata.permissions().mode() & 0o777;

    // Permissions should be preserved
    assert_eq!(
        restored_mode, 0o755,
        "Permissions were not preserved correctly"
    );

    Ok(())
}

#[test]
#[cfg(unix)]
fn test_permissions_not_preserved_when_disabled() -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let env = TestEnvironment::new()?;
    let home = &env.home_dir;
    let repo_path = env.repo_dir.clone();
    let config_path = env.home_dir.join("config.toml");

    // Create config with preserve_permissions = false
    let mut config = Config::default();
    config.tracking.preserve_permissions = false;
    config.core.repo_path = repo_path.clone();
    config.save(&config_path)?;

    let ctx = DotmanContext {
        repo_path: repo_path.clone(),
        config_path,
        config,
        no_pager: true,
    };

    // Initialize the repository (don't use the global init command)
    ctx.ensure_repo_exists()?;
    let index = dotman::storage::index::Index::new();
    index.save(&ctx.repo_path.join(dotman::INDEX_FILE))?;
    let ref_manager = dotman::refs::RefManager::new(ctx.repo_path.clone());
    ref_manager.init()?;

    // Create a test file with specific permissions
    let test_file = home.join("test_no_perms.sh");
    fs::write(&test_file, "#!/bin/bash\necho 'test'")?;

    // Make it executable (0o755)
    let perms = fs::Permissions::from_mode(0o755);
    fs::set_permissions(&test_file, perms)?;

    // Add and commit
    dotman::commands::add::execute(&ctx, &[test_file.to_string_lossy().to_string()], false)?;
    dotman::commands::commit::execute(&ctx, "Test no permissions", false)?;

    // Delete the original file
    fs::remove_file(&test_file)?;

    // Restore from the latest commit
    let commit_id = fs::read_to_string(repo_path.join("HEAD"))?;
    dotman::commands::checkout::execute(&ctx, &commit_id, true)?;

    // File should be restored but permissions might not be preserved when disabled
    assert!(test_file.exists());

    // Note: When preserve_permissions is false, the behavior is platform-dependent
    // We just verify the file exists and is readable
    let content = fs::read_to_string(&test_file)?;
    assert!(content.contains("echo 'test'"));

    Ok(())
}

#[test]
#[cfg(windows)]
fn test_windows_readonly_preservation() -> Result<()> {
    let env = TestEnvironment::new()?;
    let home = &env.home_dir;
    let repo_path = env.repo_dir.clone();
    let config_path = env.home_dir.join("config.toml");

    // Create config with preserve_permissions = true
    let mut config = Config::default();
    config.tracking.preserve_permissions = true;
    config.save(&config_path)?;

    let ctx = DotmanContext {
        repo_path: repo_path.clone(),
        config_path: config_path.clone(),
        config: config.clone(),
        no_pager: true,
    };

    ctx.ensure_repo_exists()?;

    // Create a read-only file
    let test_file = home.join("readonly.txt");
    fs::write(&test_file, "Read-only content")?;

    // Make it read-only
    let metadata = fs::metadata(&test_file)?;
    let mut perms = metadata.permissions();
    perms.set_readonly(true);
    fs::set_permissions(&test_file, perms)?;

    // Verify it's read-only
    assert!(fs::metadata(&test_file)?.permissions().readonly());

    // Add and commit
    dotman::commands::add::execute(&ctx, &vec![test_file.to_string_lossy().to_string()], false)?;
    dotman::commands::commit::execute(&ctx, "Test Windows readonly", false)?;

    // Delete the file
    // On Windows, we need to remove read-only flag before deleting
    let mut perms = fs::metadata(&test_file)?.permissions();
    perms.set_readonly(false);
    fs::set_permissions(&test_file, perms)?;
    fs::remove_file(&test_file)?;

    // Restore from commit
    let commit_id = fs::read_to_string(repo_path.join("HEAD"))?;
    dotman::commands::checkout::execute(&ctx, &commit_id, true)?;

    // Check if read-only flag was preserved
    assert!(test_file.exists());
    assert!(
        fs::metadata(&test_file)?.permissions().readonly(),
        "Read-only flag was not preserved"
    );

    Ok(())
}
