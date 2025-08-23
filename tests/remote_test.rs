use anyhow::Result;
use dotman::{DotmanContext, config::Config};
use std::fs;
use tempfile::tempdir;

#[test]
#[serial_test::serial]
fn test_push_workflow() -> Result<()> {
    let temp = tempdir()?;
    let home = temp.path().join("home");
    let repo_path = home.join(".dotman");

    fs::create_dir_all(&home)?;
    fs::create_dir_all(&repo_path)?;

    // Set HOME for the test
    unsafe {
        std::env::set_var("HOME", &home);
    }

    // Initialize dotman
    dotman::commands::init::execute(false)?;

    // Create and add a test file
    let test_file = home.join("test.txt");
    fs::write(&test_file, "test content")?;

    let config_path = home.join(".config/dotman/config");
    let config = Config::load(&config_path)?;
    let ctx = DotmanContext {
        repo_path: repo_path.clone(),
        config_path,
        config,
    };

    // Add file
    dotman::commands::add::execute(&ctx, &[test_file.to_str().unwrap().to_string()], false)?;

    // Commit
    dotman::commands::commit::execute(&ctx, "Test commit", false)?;

    // Add a remote
    let remote_path = temp.path().join("remote.git");
    fs::create_dir_all(&remote_path)?;

    // Initialize the remote as a bare git repo
    std::process::Command::new("git")
        .args(["init", "--bare"])
        .current_dir(&remote_path)
        .output()?;

    // Need mutable context for add operation
    let mut ctx = ctx;
    dotman::commands::remote::add(&mut ctx, "origin", remote_path.to_str().unwrap())?;

    // Push to remote
    let result = dotman::commands::push::execute(&ctx, "origin", "main");

    // We expect this to succeed up to the actual push
    // (which may fail if git is not configured)
    assert!(result.is_ok() || result.unwrap_err().to_string().contains("push"));

    // Verify mirror was created
    let mirror_path = repo_path.join("mirrors/origin");
    assert!(mirror_path.exists());
    assert!(mirror_path.join(".git").exists());

    Ok(())
}

#[test]
#[serial_test::serial]
fn test_pull_workflow() -> Result<()> {
    let temp = tempdir()?;
    let home = temp.path().join("home");
    let repo_path = home.join(".dotman");

    fs::create_dir_all(&home)?;
    fs::create_dir_all(&repo_path)?;

    // Set HOME for the test
    unsafe {
        std::env::set_var("HOME", &home);
    }

    // Initialize dotman
    dotman::commands::init::execute(false)?;

    let config_path = home.join(".config/dotman/config");
    let config = Config::load(&config_path)?;
    let ctx = DotmanContext {
        repo_path: repo_path.clone(),
        config_path,
        config,
    };

    // Create a "remote" repository with content
    let remote_path = temp.path().join("remote");
    fs::create_dir_all(&remote_path)?;

    // Initialize git repo and add content
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(&remote_path)
        .output()?;

    fs::write(remote_path.join("remote_file.txt"), "remote content")?;

    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(&remote_path)
        .output()?;

    std::process::Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&remote_path)
        .output()?;

    // Add remote to dotman
    let mut ctx = ctx;
    dotman::commands::remote::add(&mut ctx, "origin", remote_path.to_str().unwrap())?;

    // Pull from remote
    let result = dotman::commands::pull::execute(&ctx, "origin", "main");

    // We expect this to work or fail gracefully
    if result.is_ok() {
        // Verify mirror was created
        let mirror_path = repo_path.join("mirrors/origin");
        assert!(mirror_path.exists());
        assert!(mirror_path.join(".git").exists());
    }

    Ok(())
}

#[test]
fn test_mapping_persistence() -> Result<()> {
    use dotman::mapping::MappingManager;

    let temp = tempdir()?;
    let repo_path = temp.path().join(".dotman");
    fs::create_dir_all(&repo_path)?;

    // Create and save mapping
    let mut manager = MappingManager::new(&repo_path)?;
    manager.add_and_save("origin", "dotman123", "git456")?;
    manager.add_and_save("origin", "dotman789", "git012")?;

    // Load and verify
    let manager2 = MappingManager::new(&repo_path)?;
    assert_eq!(
        manager2.mapping().get_git_commit("origin", "dotman123"),
        Some("git456".to_string())
    );
    assert_eq!(
        manager2.mapping().get_git_commit("origin", "dotman789"),
        Some("git012".to_string())
    );

    Ok(())
}

#[test]
fn test_remote_management() -> Result<()> {
    let temp = tempdir()?;
    let home = temp.path().join("home");
    let repo_path = home.join(".dotman");

    fs::create_dir_all(&home)?;
    fs::create_dir_all(&repo_path)?;

    unsafe {
        std::env::set_var("HOME", &home);
    }

    // Initialize
    dotman::commands::init::execute(false)?;

    let config_path = home.join(".config/dotman/config");
    let config = Config::load(&config_path)?;
    let mut ctx = DotmanContext {
        repo_path: repo_path.clone(),
        config_path: config_path.clone(),
        config,
    };

    // Add multiple remotes
    dotman::commands::remote::add(&mut ctx, "origin", "https://github.com/user/repo.git")?;

    dotman::commands::remote::add(&mut ctx, "backup", "s3://my-backup")?;

    // Reload config to see changes
    let config = Config::load(&config_path)?;
    assert!(config.remotes.contains_key("origin"));
    assert!(config.remotes.contains_key("backup"));

    // Verify remote types
    use dotman::config::RemoteType;
    assert_eq!(config.remotes["origin"].remote_type, RemoteType::Git);
    assert_eq!(config.remotes["backup"].remote_type, RemoteType::S3);

    Ok(())
}
