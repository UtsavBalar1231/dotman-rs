use anyhow::Result;
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

#[test]
#[serial_test::serial]
fn test_stash_push_and_pop() -> Result<()> {
    let temp = tempdir()?;
    let home = temp.path();

    // Set HOME to temp directory
    unsafe {
        std::env::set_var("HOME", home);
    }

    Command::cargo_bin("dot")?.arg("init").assert().success();

    // Create and add a file
    let test_file = home.join("test.txt");
    fs::write(&test_file, "initial content")?;

    Command::cargo_bin("dot")?
        .args(["add", test_file.to_str().unwrap()])
        .assert()
        .success();

    // Commit the file
    Command::cargo_bin("dot")?
        .args(["commit", "-m", "Initial commit"])
        .assert()
        .success();

    // Modify the file
    fs::write(&test_file, "modified content")?;

    // Stash the changes
    Command::cargo_bin("dot")?
        .args(["stash", "push", "-m", "Test stash"])
        .assert()
        .success();

    // Check file is back to original
    let content = fs::read_to_string(&test_file)?;
    assert_eq!(content, "initial content");

    // Pop the stash
    Command::cargo_bin("dot")?
        .args(["stash", "pop"])
        .assert()
        .success();

    // Check file has the modified content
    let content = fs::read_to_string(&test_file)?;
    assert_eq!(content, "modified content");

    Ok(())
}

#[test]
#[serial_test::serial]
fn test_stash_list() -> Result<()> {
    let temp = tempdir()?;
    let home = temp.path();

    unsafe {
        std::env::set_var("HOME", home);
    }

    Command::cargo_bin("dot")?.arg("init").assert().success();

    // Create and commit a file
    let test_file = home.join("test.txt");
    fs::write(&test_file, "initial")?;

    Command::cargo_bin("dot")?
        .args(["add", test_file.to_str().unwrap()])
        .assert()
        .success();

    Command::cargo_bin("dot")?
        .args(["commit", "-m", "Initial"])
        .assert()
        .success();

    // Create stashes
    fs::write(&test_file, "change1")?;
    Command::cargo_bin("dot")?
        .args(["stash", "push", "-m", "Stash 1"])
        .assert()
        .success();

    fs::write(&test_file, "change2")?;
    Command::cargo_bin("dot")?
        .args(["stash", "push", "-m", "Stash 2"])
        .assert()
        .success();

    // List stashes
    Command::cargo_bin("dot")?
        .args(["stash", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Stash 1"))
        .stdout(predicate::str::contains("Stash 2"));

    Ok(())
}

#[test]
#[serial_test::serial]
fn test_stash_with_untracked_files() -> Result<()> {
    let temp = tempdir()?;
    let home = temp.path();

    unsafe {
        std::env::set_var("HOME", home);
    }

    Command::cargo_bin("dot")?.arg("init").assert().success();

    // Create and commit a file
    let tracked = home.join("tracked.txt");
    fs::write(&tracked, "tracked")?;

    Command::cargo_bin("dot")?
        .args(["add", tracked.to_str().unwrap()])
        .assert()
        .success();

    Command::cargo_bin("dot")?
        .args(["commit", "-m", "Initial"])
        .assert()
        .success();

    // Create untracked file
    let untracked = home.join("untracked.txt");
    fs::write(&untracked, "untracked content")?;

    // Stash without -u should not include untracked
    Command::cargo_bin("dot")?
        .args(["stash", "push"])
        .assert()
        .success();

    assert!(untracked.exists());

    // Stash with -u should include untracked
    Command::cargo_bin("dot")?
        .args(["stash", "push", "-u"])
        .assert()
        .success();

    assert!(!untracked.exists());

    // Pop should restore untracked file
    Command::cargo_bin("dot")?
        .args(["stash", "pop"])
        .assert()
        .success();

    assert!(untracked.exists());

    Ok(())
}

#[test]
#[serial_test::serial]
fn test_stash_clear() -> Result<()> {
    let temp = tempdir()?;
    let home = temp.path();

    unsafe {
        std::env::set_var("HOME", home);
    }

    Command::cargo_bin("dot")?.arg("init").assert().success();

    // Create and commit a file
    let test_file = home.join("test.txt");
    fs::write(&test_file, "initial")?;

    Command::cargo_bin("dot")?
        .args(["add", test_file.to_str().unwrap()])
        .assert()
        .success();

    Command::cargo_bin("dot")?
        .args(["commit", "-m", "Initial"])
        .assert()
        .success();

    // Create multiple stashes
    for i in 1..=3 {
        fs::write(&test_file, format!("change{}", i))?;
        Command::cargo_bin("dot")?
            .args(["stash", "push", "-m", &format!("Stash {}", i)])
            .assert()
            .success();
    }

    // Clear all stashes
    Command::cargo_bin("dot")?
        .args(["stash", "clear"])
        .assert()
        .success();

    // List should show no stashes
    Command::cargo_bin("dot")?
        .args(["stash", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No stash entries found"));

    Ok(())
}
