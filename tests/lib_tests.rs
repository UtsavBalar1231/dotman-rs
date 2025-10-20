use anyhow::Result;
use dotman::DotmanContext;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_context_new_with_explicit_paths() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("repo");
    let config_path = temp_dir.path().join("config.toml");

    let context = DotmanContext::new_with_explicit_paths(repo_path.clone(), config_path.clone())?;

    assert_eq!(context.repo_path, repo_path);
    assert_eq!(context.config_path, config_path);
    assert!(config_path.exists());

    Ok(())
}

#[test]
fn test_repo_initialization_check() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("repo");
    let config_path = temp_dir.path().join("config.toml");

    let context = DotmanContext::new_with_explicit_paths(repo_path.clone(), config_path)?;

    // Initially not initialized
    assert!(!context.is_repo_initialized());
    assert!(context.check_repo_initialized().is_err());

    // Create required files for initialization
    context.ensure_repo_exists()?;
    fs::write(repo_path.join("index.bin"), b"")?;
    fs::write(repo_path.join("HEAD"), b"ref: refs/heads/main")?;

    assert!(context.is_repo_initialized());
    assert!(context.check_repo_initialized().is_ok());

    Ok(())
}

#[test]
fn test_ensure_repo_exists() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("repo");
    let config_path = temp_dir.path().join("config.toml");

    let context = DotmanContext::new_with_explicit_paths(repo_path.clone(), config_path)?;

    context.ensure_repo_exists()?;

    assert!(repo_path.exists());
    assert!(repo_path.join("commits").exists());
    assert!(repo_path.join("objects").exists());

    Ok(())
}

#[test]
fn test_context_with_pager() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join(".dotman");
    let config_path = temp_dir.path().join("config.toml");

    // Test with pager disabled (no_pager = true)
    let context = DotmanContext::new_with_explicit_paths(repo_path.clone(), config_path.clone())?;
    let context = DotmanContext {
        no_pager: true,
        ..context
    };
    assert!(context.no_pager);

    // Test with pager enabled (no_pager = false, default)
    let context_with_pager = DotmanContext::new_with_explicit_paths(repo_path, config_path)?;
    assert!(!context_with_pager.no_pager);

    Ok(())
}

#[test]
fn test_repo_path_explicit() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let custom_repo = temp_dir.path().join("custom_repo");
    let custom_config = temp_dir.path().join("custom_config.toml");

    // Use explicit paths instead of environment variables to avoid test isolation issues
    let context =
        DotmanContext::new_with_explicit_paths(custom_repo.clone(), custom_config.clone())?;

    assert_eq!(context.repo_path, custom_repo);
    assert_eq!(context.config_path, custom_config);

    Ok(())
}

#[test]
fn test_repo_not_initialized_error() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("repo");
    let config_path = temp_dir.path().join("config.toml");

    let context = DotmanContext::new_with_explicit_paths(repo_path, config_path)?;

    let result = context.check_repo_initialized();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Repository not initialized")
    );

    Ok(())
}
