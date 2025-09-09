use anyhow::Result;
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

mod common;
use common::TestEnvironment;

#[test]
fn test_stash_push_and_pop() -> Result<()> {
    let env = TestEnvironment::new()?;
    let home = env.home_dir.as_path();

    Command::cargo_bin("dot")?
        .env("HOME", home)
        .arg("init")
        .assert()
        .success();

    // Create and add a file
    let test_file = home.join("test.txt");
    fs::write(&test_file, "initial content")?;

    Command::cargo_bin("dot")?
        .env("HOME", home)
        .args(["add", test_file.to_str().unwrap()])
        .assert()
        .success();

    // Commit the file
    Command::cargo_bin("dot")?
        .env("HOME", home)
        .args(["commit", "-m", "Initial commit"])
        .assert()
        .success();

    // Modify the file
    fs::write(&test_file, "modified content")?;

    // Stash the changes
    Command::cargo_bin("dot")?
        .env("HOME", home)
        .args(["stash", "push", "-m", "Test stash"])
        .assert()
        .success();

    // File should be back to original
    assert_eq!(fs::read_to_string(&test_file)?, "initial content");

    // Pop the stash
    Command::cargo_bin("dot")?
        .env("HOME", home)
        .args(["stash", "pop"])
        .assert()
        .success();

    // File should have modified content
    assert_eq!(fs::read_to_string(&test_file)?, "modified content");

    Ok(())
}

#[test]
fn test_stash_list() -> Result<()> {
    let env = TestEnvironment::new()?;
    let home = env.home_dir.as_path();

    Command::cargo_bin("dot")?
        .env("HOME", home)
        .arg("init")
        .assert()
        .success();

    // Create and commit a file
    let test_file = home.join("test.txt");
    fs::write(&test_file, "content")?;

    Command::cargo_bin("dot")?
        .env("HOME", home)
        .args(["add", test_file.to_str().unwrap()])
        .assert()
        .success();

    Command::cargo_bin("dot")?
        .env("HOME", home)
        .args(["commit", "-m", "Initial"])
        .assert()
        .success();

    // Create first stash
    fs::write(&test_file, "stash 1")?;
    Command::cargo_bin("dot")?
        .env("HOME", home)
        .args(["stash", "push", "-m", "First stash"])
        .assert()
        .success();

    // Create second stash
    fs::write(&test_file, "stash 2")?;
    Command::cargo_bin("dot")?
        .env("HOME", home)
        .args(["stash", "push", "-m", "Second stash"])
        .assert()
        .success();

    // List stashes
    Command::cargo_bin("dot")?
        .env("HOME", home)
        .args(["stash", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("First stash"))
        .stdout(predicate::str::contains("Second stash"));

    Ok(())
}

#[test]
fn test_stash_apply() -> Result<()> {
    let env = TestEnvironment::new()?;
    let home = env.home_dir.as_path();

    Command::cargo_bin("dot")?
        .env("HOME", home)
        .arg("init")
        .assert()
        .success();

    // Create and commit a file
    let test_file = home.join("test.txt");
    fs::write(&test_file, "original")?;

    Command::cargo_bin("dot")?
        .env("HOME", home)
        .args(["add", test_file.to_str().unwrap()])
        .assert()
        .success();

    Command::cargo_bin("dot")?
        .env("HOME", home)
        .args(["commit", "-m", "Initial"])
        .assert()
        .success();

    // Modify and stash
    fs::write(&test_file, "modified")?;

    Command::cargo_bin("dot")?
        .env("HOME", home)
        .args(["stash", "push"])
        .assert()
        .success();

    assert_eq!(fs::read_to_string(&test_file)?, "original");

    // Apply stash (keeps it in stash list)
    Command::cargo_bin("dot")?
        .env("HOME", home)
        .args(["stash", "apply"])
        .assert()
        .success();

    assert_eq!(fs::read_to_string(&test_file)?, "modified");

    // Stash should still be in list
    Command::cargo_bin("dot")?
        .env("HOME", home)
        .args(["stash", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("stash@{0}"));

    Ok(())
}

#[test]
fn test_stash_drop() -> Result<()> {
    let env = TestEnvironment::new()?;
    let home = env.home_dir.as_path();

    Command::cargo_bin("dot")?
        .env("HOME", home)
        .arg("init")
        .assert()
        .success();

    // Create and commit a file
    let test_file = home.join("test.txt");
    fs::write(&test_file, "content")?;

    Command::cargo_bin("dot")?
        .env("HOME", home)
        .args(["add", test_file.to_str().unwrap()])
        .assert()
        .success();

    Command::cargo_bin("dot")?
        .env("HOME", home)
        .args(["commit", "-m", "Initial"])
        .assert()
        .success();

    // Create stashes
    for i in 1..=3 {
        fs::write(&test_file, format!("stash {i}"))?;
        Command::cargo_bin("dot")?
            .env("HOME", home)
            .args(["stash", "push", "-m", &format!("Stash {i}")])
            .assert()
            .success();
    }

    // Drop the middle stash (stash@{1})
    Command::cargo_bin("dot")?
        .env("HOME", home)
        .args(["stash", "drop", "stash@{1}"])
        .assert()
        .success();

    // List should not contain "Stash 2" but should have 1 and 3
    Command::cargo_bin("dot")?
        .env("HOME", home)
        .args(["stash", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Stash 3"))
        .stdout(predicate::str::contains("Stash 1"));

    Ok(())
}
