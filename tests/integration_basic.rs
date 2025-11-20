use anyhow::Result;
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_init_command() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join(".dotman");

    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .arg("init")
        .assert()
        .success()
        .stderr(predicate::str::contains("Initialized"));

    assert!(repo_path.exists());
    assert!(repo_path.join("index.bin").exists());
    assert!(repo_path.join("HEAD").exists());

    Ok(())
}

#[test]
fn test_init_already_initialized() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join(".dotman");

    // First init
    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .arg("init")
        .assert()
        .success();

    // Second init should fail
    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .arg("init")
        .assert()
        .failure()
        .stderr(predicate::str::contains("already initialized"));

    Ok(())
}

#[test]
fn test_add_and_status() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join(".dotman");

    // Create test file
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, b"test content")?;

    // Initialize repo
    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .arg("init")
        .assert()
        .success();

    // Add file
    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .args(["add", test_file.to_str().unwrap()])
        .assert()
        .success();

    // Check status
    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("test.txt"));

    Ok(())
}

#[test]
fn test_add_directory() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join(".dotman");

    // Create test directory with files
    let test_dir = temp_dir.path().join("config");
    fs::create_dir_all(&test_dir)?;
    fs::write(test_dir.join("file1.conf"), b"config 1")?;
    fs::write(test_dir.join("file2.conf"), b"config 2")?;

    // Initialize repo
    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .arg("init")
        .assert()
        .success();

    // Add directory
    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .args(["add", test_dir.to_str().unwrap()])
        .assert()
        .success();

    // Check status - should show both files
    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("file1.conf"))
        .stdout(predicate::str::contains("file2.conf"));

    Ok(())
}

#[test]
fn test_commit_workflow() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join(".dotman");

    // Setup
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, b"test content")?;

    // Initialize
    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .arg("init")
        .assert()
        .success();

    // Add
    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .args(["add", test_file.to_str().unwrap()])
        .assert()
        .success();

    // Commit
    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .args(["commit", "-m", "Initial commit"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Committed"));

    // Verify clean status
    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("nothing to commit"));

    Ok(())
}

#[test]
fn test_commit_without_changes() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join(".dotman");

    // Initialize
    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .arg("init")
        .assert()
        .success();

    // Commit without staged changes should fail
    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .args(["commit", "-m", "Empty commit"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No files tracked"));

    Ok(())
}

#[test]
fn test_file_modification_tracking() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join(".dotman");

    // Create and commit initial file
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, b"initial content")?;

    // Initialize
    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .arg("init")
        .assert()
        .success();

    // Add and commit
    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .args(["add", test_file.to_str().unwrap()])
        .assert()
        .success();

    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .args(["commit", "-m", "Initial"])
        .assert()
        .success();

    // Modify file
    fs::write(&test_file, b"modified content")?;

    // Status should show modification
    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("modified"));

    Ok(())
}

#[test]
fn test_log_command() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join(".dotman");

    // Setup and make commits
    let test_file = temp_dir.path().join("test.txt");

    // Initialize
    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .arg("init")
        .assert()
        .success();

    // First commit
    fs::write(&test_file, b"content 1")?;
    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .args(["add", test_file.to_str().unwrap()])
        .assert()
        .success();
    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .args(["commit", "-m", "First commit"])
        .assert()
        .success();

    // Second commit
    fs::write(&test_file, b"content 2")?;
    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .args(["add", test_file.to_str().unwrap()])
        .assert()
        .success();
    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .args(["commit", "-m", "Second commit"])
        .assert()
        .success();

    // Check log
    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .arg("log")
        .assert()
        .success()
        .stdout(predicate::str::contains("First commit"))
        .stdout(predicate::str::contains("Second commit"));

    Ok(())
}

#[test]
fn test_remove_command() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join(".dotman");

    // Create and add file
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, b"test content")?;

    // Initialize
    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .arg("init")
        .assert()
        .success();

    // Add and commit
    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .args(["add", test_file.to_str().unwrap()])
        .assert()
        .success();
    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .args(["commit", "-m", "Add file"])
        .assert()
        .success();

    // Remove file
    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .args(["rm", test_file.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("removed"));

    // Status should show the deletion
    Command::cargo_bin("dot")?
        .env("HOME", temp_dir.path())
        .env("DOTMAN_REPO_PATH", &repo_path)
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("deleted"));

    Ok(())
}
