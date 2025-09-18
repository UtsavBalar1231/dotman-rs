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
    unsafe {
        std::env::set_var("DOTMAN_REPO_PATH", temp_dir.path().join(".dotman"));
        std::env::set_var("DOTMAN_CONFIG_PATH", temp_dir.path().join("config.toml"));
    }

    let context = DotmanContext::new_with_pager(false)?;
    assert_eq!(context.no_pager, false);

    let context_no_pager = DotmanContext::new()?;
    assert_eq!(context_no_pager.no_pager, true);

    unsafe {
        std::env::remove_var("DOTMAN_REPO_PATH");
        std::env::remove_var("DOTMAN_CONFIG_PATH");
    }

    Ok(())
}

#[test]
fn test_repo_path_from_env() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let custom_repo = temp_dir.path().join("custom_repo");
    let custom_config = temp_dir.path().join("custom_config.toml");

    unsafe {
        std::env::set_var("DOTMAN_REPO_PATH", &custom_repo);
        std::env::set_var("DOTMAN_CONFIG_PATH", &custom_config);
    }

    let context = DotmanContext::new()?;

    assert_eq!(context.repo_path, custom_repo);
    assert_eq!(context.config_path, custom_config);

    unsafe {
        std::env::remove_var("DOTMAN_REPO_PATH");
        std::env::remove_var("DOTMAN_CONFIG_PATH");
    }

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
