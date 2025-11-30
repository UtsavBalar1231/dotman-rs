use anyhow::Result;
use dotman::DotmanContext;
use dotman::commands::{add, commit, rebase};
use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper function to create a test context with isolated repository
fn setup_test_context() -> Result<(TempDir, TempDir, DotmanContext)> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join(".dotman");
    let config_path = temp_dir.path().join(".config/dotman/config");

    // Create config directory
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write test config with temp directory in allowed_directories
    let config_content = format!(
        r#"[security]
allowed_directories = ["{}"]
enforce_path_validation = true
strip_dangerous_permissions = true
"#,
        temp_dir.path().display()
    );
    fs::write(&config_path, config_content)?;

    let ctx = DotmanContext::new_explicit(repo_path, config_path)?;
    ctx.ensure_repo_exists()?;

    // Initialize the repository properly
    let index = dotman::storage::index::Index::new();
    let index_path = ctx.repo_path.join("index.bin");
    index.save(&index_path)?;

    // Initialize refs structure (HEAD, branches)
    let ref_manager = dotman::refs::RefManager::new(ctx.repo_path.clone());
    ref_manager.init()?;

    // Create a dummy config_dir just to satisfy the return type
    let config_dir = TempDir::new()?;

    Ok((temp_dir, config_dir, ctx))
}

/// Helper function to create a test file with content
fn create_test_file(path: &PathBuf, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

/// Helper function to read file content
fn read_test_file(path: &PathBuf) -> Result<String> {
    Ok(fs::read_to_string(path)?)
}

#[test]
#[serial]
fn test_rebase_simple_linear() -> Result<()> {
    let (temp_dir, _config_dir, ctx) = setup_test_context()?;
    let home = temp_dir.path();

    // Create initial commit on main branch
    let file1 = home.join("test1.txt");
    create_test_file(&file1, "initial content")?;
    add::execute(&ctx, &[file1.to_str().unwrap().to_string()], false, false)?;
    commit::execute(&ctx, "Initial commit", false)?;

    // Create a feature branch
    dotman::commands::branch::create(&ctx, "feature", None)?;
    dotman::commands::checkout::execute(&ctx, "feature", false, false)?;

    // Make a commit on feature branch
    let file2 = home.join("test2.txt");
    create_test_file(&file2, "feature content")?;
    add::execute(&ctx, &[file2.to_str().unwrap().to_string()], false, false)?;
    commit::execute(&ctx, "Feature commit", false)?;

    // Switch back to main and make another commit
    dotman::commands::checkout::execute(&ctx, "main", true, false)?;
    let file3 = home.join("test3.txt");
    create_test_file(&file3, "main content")?;
    add::execute(&ctx, &[file3.to_str().unwrap().to_string()], false, false)?;
    commit::execute(&ctx, "Main commit", false)?;

    // Switch back to feature and rebase onto main
    dotman::commands::checkout::execute(&ctx, "feature", false, false)?;
    rebase::execute(&ctx, Some("main"), None, false, false, false)?;

    // Verify that feature branch now has all three files
    assert!(file1.exists());
    assert!(file2.exists());
    assert!(file3.exists());

    // Clean up
    fs::remove_file(&file1).ok();
    fs::remove_file(&file2).ok();
    fs::remove_file(&file3).ok();

    Ok(())
}

#[test]
#[serial]
fn test_rebase_abort_restores_state() -> Result<()> {
    let (temp_dir, _config_dir, ctx) = setup_test_context()?;
    let home = temp_dir.path();

    // Create initial commit
    let file1 = home.join("test_abort1.txt");
    create_test_file(&file1, "initial")?;
    add::execute(&ctx, &[file1.to_str().unwrap().to_string()], false, false)?;
    commit::execute(&ctx, "Initial", false)?;

    // Create feature branch with a commit
    dotman::commands::branch::create(&ctx, "feature-abort", None)?;
    dotman::commands::checkout::execute(&ctx, "feature-abort", false, false)?;
    create_test_file(&file1, "feature change")?;
    add::execute(&ctx, &[file1.to_str().unwrap().to_string()], false, false)?;
    commit::execute(&ctx, "Feature change", false)?;

    // Switch to main and make conflicting change
    dotman::commands::checkout::execute(&ctx, "main", true, false)?;
    create_test_file(&file1, "main change")?;
    add::execute(&ctx, &[file1.to_str().unwrap().to_string()], false, false)?;
    commit::execute(&ctx, "Main change", false)?;

    // Switch back to feature and attempt rebase (should conflict)
    dotman::commands::checkout::execute(&ctx, "feature-abort", false, false)?;
    let rebase_result = rebase::execute(&ctx, Some("main"), None, false, false, false);

    // Rebase should fail with conflicts
    assert!(rebase_result.is_err());

    // Abort the rebase
    rebase::execute(&ctx, None, None, false, true, false)?;

    // Verify state is restored
    let restored_content = read_test_file(&file1)?;
    assert_eq!(restored_content, "feature change");

    // Clean up
    fs::remove_file(&file1).ok();

    Ok(())
}

#[test]
#[serial]
fn test_rebase_already_up_to_date() -> Result<()> {
    let (temp_dir, _config_dir, ctx) = setup_test_context()?;
    let home = temp_dir.path();

    // Create initial commit
    let file1 = home.join("test_uptodate.txt");
    create_test_file(&file1, "content")?;
    add::execute(&ctx, &[file1.to_str().unwrap().to_string()], false, false)?;
    commit::execute(&ctx, "Initial", false)?;

    // Create feature branch (no new commits)
    dotman::commands::branch::create(&ctx, "feature-uptodate", None)?;
    dotman::commands::checkout::execute(&ctx, "feature-uptodate", false, false)?;

    // Attempt rebase (should report up to date)
    let result = rebase::execute(&ctx, Some("main"), None, false, false, false);

    // Should succeed with "up to date" message
    assert!(result.is_ok());

    // Clean up
    fs::remove_file(&file1).ok();

    Ok(())
}

#[test]
#[serial]
fn test_rebase_skip_commit() -> Result<()> {
    let (temp_dir, _config_dir, ctx) = setup_test_context()?;
    let home = temp_dir.path();

    // Create initial commit
    let file1 = home.join("test_skip.txt");
    create_test_file(&file1, "initial")?;
    add::execute(&ctx, &[file1.to_str().unwrap().to_string()], false, false)?;
    commit::execute(&ctx, "Initial", false)?;

    // Create feature branch with two commits
    dotman::commands::branch::create(&ctx, "feature-skip", None)?;
    dotman::commands::checkout::execute(&ctx, "feature-skip", false, false)?;

    create_test_file(&file1, "feature change 1")?;
    add::execute(&ctx, &[file1.to_str().unwrap().to_string()], false, false)?;
    commit::execute(&ctx, "Feature commit 1", false)?;

    let file2 = home.join("test_skip2.txt");
    create_test_file(&file2, "feature file 2")?;
    add::execute(&ctx, &[file2.to_str().unwrap().to_string()], false, false)?;
    commit::execute(&ctx, "Feature commit 2", false)?;

    // Switch to main and make conflicting change
    dotman::commands::checkout::execute(&ctx, "main", true, false)?;
    create_test_file(&file1, "main change")?;
    add::execute(&ctx, &[file1.to_str().unwrap().to_string()], false, false)?;
    commit::execute(&ctx, "Main change", false)?;

    // Switch back to feature and start rebase
    dotman::commands::checkout::execute(&ctx, "feature-skip", false, false)?;
    let rebase_result = rebase::execute(&ctx, Some("main"), None, false, false, false);

    // Should conflict on first commit
    assert!(rebase_result.is_err());

    // Skip the conflicting commit
    rebase::execute(&ctx, None, None, false, false, true)?;

    // The second commit should be applied, so file2 should exist
    assert!(file2.exists());

    // Clean up
    fs::remove_file(&file1).ok();
    fs::remove_file(&file2).ok();

    Ok(())
}

#[test]
#[serial]
fn test_rebase_no_changes() -> Result<()> {
    let (temp_dir, _config_dir, ctx) = setup_test_context()?;
    let home = temp_dir.path();

    // Create initial commit
    let file1 = home.join("test_nochange.txt");
    create_test_file(&file1, "content")?;
    add::execute(&ctx, &[file1.to_str().unwrap().to_string()], false, false)?;
    commit::execute(&ctx, "Initial", false)?;

    // Create and checkout feature branch
    dotman::commands::branch::create(&ctx, "feature-nochange", None)?;
    dotman::commands::checkout::execute(&ctx, "feature-nochange", false, false)?;

    // Rebase onto main (no commits to replay)
    let result = rebase::execute(&ctx, Some("main"), None, false, false, false);

    // Should succeed with no changes
    assert!(result.is_ok());

    // Clean up
    fs::remove_file(&file1).ok();

    Ok(())
}
