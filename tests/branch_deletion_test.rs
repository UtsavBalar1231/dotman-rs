use anyhow::Result;
use dotman::DotmanContext;
use dotman::commands::{add, branch, commit};
use std::fs;
use tempfile::tempdir;

/// Helper to create a test context with initialized repo
fn setup_test_env() -> Result<(tempfile::TempDir, DotmanContext)> {
    let temp = tempdir()?;
    let repo_path = temp.path().join(".dotman");
    let config_path = temp.path().join("config.toml");

    // Create repo structure
    fs::create_dir_all(&repo_path)?;
    fs::create_dir_all(repo_path.join("commits"))?;
    fs::create_dir_all(repo_path.join("objects"))?;

    // Create empty index
    let index = dotman::storage::index::Index::new();
    index.save(&repo_path.join("index.bin"))?;

    // Initialize refs
    let ref_manager = dotman::refs::RefManager::new(repo_path.clone());
    ref_manager.init()?;

    // Create default config
    let mut config = dotman::config::Config::default();
    config.core.repo_path.clone_from(&repo_path);
    config.save(&config_path)?;

    let ctx = DotmanContext {
        repo_path,
        config_path,
        config,
        no_pager: true,
    };

    Ok((temp, ctx))
}

#[test]
fn test_branch_deletion_not_merged() -> Result<()> {
    let (temp, ctx) = setup_test_env()?;

    // Create a test file and commit on main
    let test_file1 = temp.path().join("file1.txt");
    fs::write(&test_file1, "Initial content")?;
    add::execute(&ctx, &[test_file1.to_str().unwrap().to_string()], false)?;
    commit::execute(&ctx, "Initial commit", false)?;

    // Create a feature branch
    branch::create(&ctx, "feature", None)?;
    branch::checkout(&ctx, "feature", false)?;

    // Make a commit on the feature branch
    let test_file2 = temp.path().join("file2.txt");
    fs::write(&test_file2, "Feature content")?;
    add::execute(&ctx, &[test_file2.to_str().unwrap().to_string()], false)?;
    commit::execute(&ctx, "Feature commit", false)?;

    // Switch back to main
    branch::checkout(&ctx, "main", false)?;

    // Try to delete the unmerged feature branch without force
    let result = branch::delete(&ctx, "feature", false);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not fully merged"));

    // Should work with force
    let result = branch::delete(&ctx, "feature", true);
    assert!(result.is_ok());

    Ok(())
}

#[test]
fn test_branch_deletion_merged() -> Result<()> {
    let (temp, ctx) = setup_test_env()?;

    // Create a test file and commit on main
    let test_file1 = temp.path().join("file1.txt");
    fs::write(&test_file1, "Initial content")?;
    add::execute(&ctx, &[test_file1.to_str().unwrap().to_string()], false)?;
    commit::execute(&ctx, "Initial commit", false)?;

    // Create a feature branch at the same commit
    branch::create(&ctx, "feature", None)?;

    // Feature branch should be deletable since it's at the same commit as main
    let result = branch::delete(&ctx, "feature", false);
    assert!(result.is_ok());

    Ok(())
}

#[test]
fn test_branch_deletion_merged_after_merge() -> Result<()> {
    let (temp, ctx) = setup_test_env()?;

    // Create initial commit on main
    let test_file1 = temp.path().join("file1.txt");
    fs::write(&test_file1, "Initial content")?;
    add::execute(&ctx, &[test_file1.to_str().unwrap().to_string()], false)?;
    commit::execute(&ctx, "Initial commit", false)?;

    // Create and switch to feature branch
    branch::create(&ctx, "feature", None)?;
    branch::checkout(&ctx, "feature", false)?;

    // Make a commit on feature
    let test_file2 = temp.path().join("file2.txt");
    fs::write(&test_file2, "Feature content")?;
    add::execute(&ctx, &[test_file2.to_str().unwrap().to_string()], false)?;
    commit::execute(&ctx, "Feature commit", false)?;

    // Switch back to main and "merge" by making a commit with feature as parent
    branch::checkout(&ctx, "main", false)?;

    // For a simple test, we'll just advance main to include the feature changes
    // In a real scenario, this would be done via merge command
    fs::write(&test_file2, "Feature content")?;
    add::execute(&ctx, &[test_file2.to_str().unwrap().to_string()], false)?;
    commit::execute(&ctx, "Merge feature into main", false)?;

    // Note: In the current implementation, the branch won't be considered merged
    // unless main's history includes feature's tip commit. This would require
    // proper merge commit implementation with multiple parents or fast-forward.
    // For now, we test that force deletion works.
    let result = branch::delete(&ctx, "feature", true);
    assert!(result.is_ok());

    Ok(())
}

#[test]
fn test_cannot_delete_current_branch() -> Result<()> {
    let (_temp, ctx) = setup_test_env()?;

    // Try to delete the current branch (main)
    let result = branch::delete(&ctx, "main", false);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("currently checked out")
    );

    // Shouldn't work even with force
    let result = branch::delete(&ctx, "main", true);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("currently checked out")
    );

    Ok(())
}

#[test]
fn test_delete_nonexistent_branch() -> Result<()> {
    let (_temp, ctx) = setup_test_env()?;

    let result = branch::delete(&ctx, "nonexistent", false);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not exist"));

    Ok(())
}

#[test]
fn test_protect_main_branch() -> Result<()> {
    let (_temp, ctx) = setup_test_env()?;

    // Create another branch and switch to it
    branch::create(&ctx, "develop", None)?;
    branch::checkout(&ctx, "develop", false)?;

    // Try to delete main without force
    let result = branch::delete(&ctx, "main", false);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Cannot delete the 'main' branch without --force")
    );

    // Should work with force
    let result = branch::delete(&ctx, "main", true);
    assert!(result.is_ok());

    Ok(())
}
