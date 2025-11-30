//! Security-focused tests for path validation and permission sanitization
//!
//! Tests for Issues #5 and #6:
//! - Path traversal vulnerability prevention
//! - Dangerous permission bits (setuid/setgid/sticky) sanitization

use anyhow::Result;
use dotman::DotmanContext;
use dotman::commands::context::CommandContext;
use serial_test::serial;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Setup test environment with isolated home and repo directories
fn setup_test_env() -> Result<(TempDir, TempDir, DotmanContext)> {
    let home_dir = TempDir::new()?;
    let repo_dir = TempDir::new()?;

    // Create config file with security settings
    let config_dir = TempDir::new()?;
    let config_path = config_dir.path().join("config");

    let config_content = format!(
        r#"[security]
allowed_directories = ["{}"]
enforce_path_validation = true
strip_dangerous_permissions = true
max_file_mode = 0o777
"#,
        home_dir.path().display()
    );

    std::fs::write(&config_path, config_content)?;

    // Create context with config file
    let ctx = DotmanContext::new_explicit(repo_dir.path().to_path_buf(), config_path)?;

    // Initialize repository structure
    std::fs::create_dir_all(ctx.repo_path.clone())?;
    std::fs::create_dir_all(ctx.repo_path.join("refs/heads"))?;
    std::fs::create_dir_all(ctx.repo_path.join("refs/tags"))?;
    std::fs::create_dir_all(ctx.repo_path.join("commits"))?;
    std::fs::create_dir_all(ctx.repo_path.join("objects"))?;

    // Create index
    let index_path = ctx.repo_path.join("index.bin");
    let index = dotman::storage::index::Index::new();
    index.save(&index_path)?;

    // Create HEAD file (required for ensure_initialized check)
    std::fs::write(ctx.repo_path.join("HEAD"), "ref: refs/heads/main\n")?;

    // Create main branch (pointing to nothing initially - will be created on first commit)
    std::fs::write(
        ctx.repo_path.join("refs/heads/main"),
        "0000000000000000000000000000000000000000\n",
    )?;

    Ok((home_dir, repo_dir, ctx))
}

// ============================================================================
// Path Traversal Tests (Issue #5)
// ============================================================================

#[test]
#[serial]
fn test_rejects_parent_directory_traversal() -> Result<()> {
    let (home_dir, _repo_dir, ctx) = setup_test_env()?;

    // Create a file outside home directory
    let outside_file = home_dir.path().parent().unwrap().join("outside.txt");
    fs::write(&outside_file, "should not track")?;

    // Try to use ../../ pattern to escape
    let traversal_path = home_dir.path().join("..").join("outside.txt");

    // Validation should reject this
    let result = ctx.validate_user_path(&traversal_path);
    assert!(result.is_err(), "Should reject parent directory traversal");

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("outside allowed directories"),
        "Error should mention allowed directories: {err_msg}"
    );

    Ok(())
}

#[test]
#[serial]
fn test_rejects_tilde_bypass_pattern() -> Result<()> {
    let (_home_dir, _repo_dir, _ctx) = setup_test_env()?;

    // Try to use ~/../../../etc pattern
    let result = dotman::utils::paths::expand_tilde(Path::new("~/../../../etc/passwd"));

    assert!(result.is_err(), "Should reject tilde bypass pattern");

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Path traversal detected"),
        "Error should mention path traversal: {err_msg}"
    );

    Ok(())
}

#[test]
#[serial]
fn test_rejects_absolute_path_outside_allowed() -> Result<()> {
    let (_home_dir, _repo_dir, ctx) = setup_test_env()?;

    // Try to track /etc/passwd
    let result = ctx.validate_user_path(Path::new("/etc/passwd"));

    assert!(
        result.is_err(),
        "Should reject absolute path outside allowed directories"
    );

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("outside allowed directories"),
        "Error should mention allowed directories: {err_msg}"
    );

    Ok(())
}

#[test]
#[serial]
fn test_symlink_escape_prevention() -> Result<()> {
    let (home_dir, _repo_dir, ctx) = setup_test_env()?;

    // Create symlink pointing outside allowed directory
    let link_path = home_dir.path().join("escape_link");
    let target_path = PathBuf::from("/etc/passwd");

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&target_path, &link_path)?;

        // Validation should reject following symlink outside allowed dirs
        let result = ctx.validate_user_path(&link_path);

        assert!(
            result.is_err(),
            "Should reject symlink pointing outside allowed directories"
        );

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("outside allowed directories")
                || err_msg.contains("Failed to canonicalize"),
            "Error should mention path validation failure: {err_msg}"
        );
    }

    Ok(())
}

#[test]
#[serial]
fn test_allows_valid_paths_within_home() -> Result<()> {
    let (home_dir, _repo_dir, ctx) = setup_test_env()?;

    // Create valid file within home directory
    let valid_file = home_dir.path().join("valid.txt");
    fs::write(&valid_file, "allowed")?;

    // Validation should accept this
    let result = ctx.validate_user_path(&valid_file);
    assert!(result.is_ok(), "Should allow valid paths within home");

    Ok(())
}

#[test]
#[serial]
fn test_allows_nested_directories_within_home() -> Result<()> {
    let (home_dir, _repo_dir, ctx) = setup_test_env()?;

    // Create nested directory structure
    let nested_dir = home_dir.path().join(".config").join("nvim");
    fs::create_dir_all(&nested_dir)?;

    let nested_file = nested_dir.join("init.vim");
    fs::write(&nested_file, "config")?;

    // Validation should accept nested paths
    let result = ctx.validate_user_path(&nested_file);
    assert!(
        result.is_ok(),
        "Should allow nested directories within home"
    );

    Ok(())
}

#[test]
#[serial]
fn test_validation_respects_enforce_flag() -> Result<()> {
    let (home_dir, repo_dir, _ctx) = setup_test_env()?;

    // Create config file with validation disabled
    let config_dir = TempDir::new()?;
    let config_path = config_dir.path().join("config");

    let config_content = format!(
        r#"[security]
allowed_directories = ["{}"]
enforce_path_validation = false
strip_dangerous_permissions = true
max_file_mode = 0o777
"#,
        home_dir.path().display()
    );

    std::fs::write(&config_path, config_content)?;

    let ctx_no_enforce = DotmanContext::new_explicit(repo_dir.path().to_path_buf(), config_path)?;

    // Initialize repository structure
    std::fs::create_dir_all(ctx_no_enforce.repo_path.clone())?;
    std::fs::create_dir_all(ctx_no_enforce.repo_path.join("refs/heads"))?;
    std::fs::create_dir_all(ctx_no_enforce.repo_path.join("refs/tags"))?;
    std::fs::create_dir_all(ctx_no_enforce.repo_path.join("commits"))?;
    std::fs::create_dir_all(ctx_no_enforce.repo_path.join("objects"))?;

    let index_path = ctx_no_enforce.repo_path.join("index.bin");
    let index = dotman::storage::index::Index::new();
    index.save(&index_path)?;

    std::fs::write(
        ctx_no_enforce.repo_path.join("HEAD"),
        "ref: refs/heads/main\n",
    )?;

    // Try path outside allowed directories
    let outside_file = home_dir.path().parent().unwrap().join("outside.txt");
    fs::write(&outside_file, "test")?;

    // Should NOT reject when enforcement is disabled
    let result = ctx_no_enforce.validate_user_path(&outside_file);
    assert!(
        result.is_ok(),
        "Should allow paths when enforce_path_validation=false"
    );

    Ok(())
}

// ============================================================================
// Permission Sanitization Tests (Issue #6)
// ============================================================================

#[test]
#[cfg(unix)]
fn test_strips_setuid_bit() -> Result<()> {
    use dotman::utils::permissions::FilePermissions;

    let (_home_dir, _repo_dir, _ctx) = setup_test_env()?;

    // Create permissions with setuid bit (0o4755)
    let dangerous_perms = FilePermissions::from_mode(0o4755);
    assert!(
        dangerous_perms.has_dangerous_bits(),
        "Should detect setuid bit"
    );

    // Strip dangerous bits
    let safe_perms = dangerous_perms.sanitized();
    assert_eq!(
        safe_perms.mode(),
        0o755,
        "Should strip setuid bit (0o4755 → 0o755)"
    );
    assert!(
        !safe_perms.has_dangerous_bits(),
        "Sanitized permissions should have no dangerous bits"
    );

    Ok(())
}

#[test]
#[cfg(unix)]
fn test_strips_setgid_bit() -> Result<()> {
    use dotman::utils::permissions::FilePermissions;

    let (_home_dir, _repo_dir, _ctx) = setup_test_env()?;

    // Create permissions with setgid bit (0o2755)
    let dangerous_perms = FilePermissions::from_mode(0o2755);
    assert!(
        dangerous_perms.has_dangerous_bits(),
        "Should detect setgid bit"
    );

    // Strip dangerous bits
    let safe_perms = dangerous_perms.sanitized();
    assert_eq!(
        safe_perms.mode(),
        0o755,
        "Should strip setgid bit (0o2755 → 0o755)"
    );

    Ok(())
}

#[test]
#[cfg(unix)]
fn test_strips_sticky_bit() -> Result<()> {
    use dotman::utils::permissions::FilePermissions;

    let (_home_dir, _repo_dir, _ctx) = setup_test_env()?;

    // Create permissions with sticky bit (0o1755)
    let dangerous_perms = FilePermissions::from_mode(0o1755);
    assert!(
        dangerous_perms.has_dangerous_bits(),
        "Should detect sticky bit"
    );

    // Strip dangerous bits
    let safe_perms = dangerous_perms.sanitized();
    assert_eq!(
        safe_perms.mode(),
        0o755,
        "Should strip sticky bit (0o1755 → 0o755)"
    );

    Ok(())
}

#[test]
#[cfg(unix)]
fn test_strips_all_dangerous_bits() -> Result<()> {
    use dotman::utils::permissions::FilePermissions;

    let (_home_dir, _repo_dir, _ctx) = setup_test_env()?;

    // Create permissions with all dangerous bits (0o7755)
    let dangerous_perms = FilePermissions::from_mode(0o7755);
    assert!(
        dangerous_perms.has_dangerous_bits(),
        "Should detect all dangerous bits"
    );

    let dangerous_bits = dangerous_perms.get_dangerous_bits();
    assert_eq!(
        dangerous_bits.len(),
        3,
        "Should detect all 3 dangerous bits"
    );

    // Strip dangerous bits
    let safe_perms = dangerous_perms.sanitized();
    assert_eq!(
        safe_perms.mode(),
        0o755,
        "Should strip all dangerous bits (0o7755 → 0o755)"
    );

    Ok(())
}

#[test]
#[cfg(unix)]
fn test_preserves_normal_permissions() -> Result<()> {
    use dotman::utils::permissions::FilePermissions;

    let (_home_dir, _repo_dir, _ctx) = setup_test_env()?;

    // Test various normal permission modes
    let test_modes = vec![0o644, 0o755, 0o600, 0o700, 0o666];

    for mode in test_modes {
        let perms = FilePermissions::from_mode(mode);
        assert!(
            !perms.has_dangerous_bits(),
            "Mode 0o{mode:o} should not have dangerous bits"
        );

        let safe_perms = perms.sanitized();
        assert_eq!(
            safe_perms.mode(),
            mode,
            "Normal permissions 0o{mode:o} should be preserved"
        );
    }

    Ok(())
}

#[test]
#[cfg(unix)]
fn test_from_path_strips_dangerous_when_enabled() -> Result<()> {
    use dotman::utils::permissions::FilePermissions;

    let (home_dir, _repo_dir, _ctx) = setup_test_env()?;

    // Create file with dangerous permissions
    let test_file = home_dir.path().join("test_setuid");
    fs::write(&test_file, "test")?;
    fs::set_permissions(&test_file, fs::Permissions::from_mode(0o4755))?;

    // Read with stripping enabled
    let perms = FilePermissions::from_path(&test_file, true)?;
    assert_eq!(
        perms.mode(),
        0o755,
        "from_path should strip dangerous bits when strip_dangerous=true"
    );

    Ok(())
}

#[test]
#[cfg(unix)]
fn test_from_path_preserves_dangerous_when_disabled() -> Result<()> {
    use dotman::utils::permissions::FilePermissions;

    let (home_dir, _repo_dir, _ctx) = setup_test_env()?;

    // Create file with dangerous permissions
    let test_file = home_dir.path().join("test_setuid");
    fs::write(&test_file, "test")?;
    fs::set_permissions(&test_file, fs::Permissions::from_mode(0o4755))?;

    // Read WITHOUT stripping
    let perms = FilePermissions::from_path(&test_file, false)?;
    // Mask with 0o7777 to get only permission bits (file type bits are in higher bits)
    let mode_bits = perms.mode() & 0o7777;
    assert_eq!(
        mode_bits, 0o4755,
        "from_path should preserve dangerous bits when strip_dangerous=false"
    );

    Ok(())
}

#[test]
#[cfg(unix)]
fn test_apply_to_path_strips_dangerous_when_disallowed() -> Result<()> {
    use dotman::utils::permissions::FilePermissions;

    let (home_dir, _repo_dir, _ctx) = setup_test_env()?;

    // Create test file
    let test_file = home_dir.path().join("test_apply");
    fs::write(&test_file, "test")?;

    // Apply dangerous permissions with allow_dangerous=false
    let dangerous_perms = FilePermissions::from_mode(0o4755);
    dangerous_perms.apply_to_path(&test_file, true, false)?;

    // Verify dangerous bits were NOT applied
    let metadata = fs::metadata(&test_file)?;
    let actual_mode = metadata.permissions().mode() & 0o7777;
    assert_eq!(
        actual_mode, 0o755,
        "apply_to_path should strip dangerous bits when allow_dangerous=false"
    );

    Ok(())
}

// ============================================================================
// Integration Tests (add → commit → restore cycle)
// ============================================================================

#[test]
#[serial]
#[cfg(unix)]
fn test_add_commit_restore_strips_dangerous_permissions() -> Result<()> {
    let (home_dir, _repo_dir, ctx) = setup_test_env()?;

    // Create file with dangerous permissions
    let test_file = home_dir.path().join("dangerous_file");
    fs::write(&test_file, "sensitive content")?;
    fs::set_permissions(&test_file, fs::Permissions::from_mode(0o4755))?;

    // Add the file (should strip dangerous bits and warn)
    let test_file_str = test_file.to_str().unwrap().to_string();
    dotman::commands::add::execute(&ctx, std::slice::from_ref(&test_file_str), false, false)?;

    // Commit
    dotman::commands::commit::execute(&ctx, "Test dangerous perms", false)?;

    // Modify the file
    fs::write(&test_file, "modified")?;

    // Restore from commit (should restore with safe permissions only)
    let resolver = ctx.create_ref_resolver();
    let commit_id = resolver.resolve("HEAD")?;
    dotman::commands::restore::execute(&ctx, &[test_file_str], Some(&commit_id), false)?;

    // Verify restored permissions are safe
    let metadata = fs::metadata(&test_file)?;
    let restored_mode = metadata.permissions().mode() & 0o7777;
    assert_eq!(
        restored_mode, 0o755,
        "Restored file should have safe permissions (dangerous bits stripped)"
    );

    Ok(())
}

#[test]
#[serial]
#[cfg(unix)]
fn test_normal_permissions_preserved_through_cycle() -> Result<()> {
    let (home_dir, _repo_dir, ctx) = setup_test_env()?;

    // Create file with normal permissions
    let test_file = home_dir.path().join("normal_file");
    fs::write(&test_file, "normal content")?;
    fs::set_permissions(&test_file, fs::Permissions::from_mode(0o644))?;

    // Add the file
    let test_file_str = test_file.to_str().unwrap().to_string();
    dotman::commands::add::execute(&ctx, std::slice::from_ref(&test_file_str), false, false)?;

    // Commit
    dotman::commands::commit::execute(&ctx, "Test normal perms", false)?;

    // Modify the file
    fs::write(&test_file, "modified")?;
    fs::set_permissions(&test_file, fs::Permissions::from_mode(0o777))?; // Change perms

    // Restore from commit
    let resolver = ctx.create_ref_resolver();
    let commit_id = resolver.resolve("HEAD")?;
    dotman::commands::restore::execute(&ctx, &[test_file_str], Some(&commit_id), false)?;

    // Verify normal permissions are preserved
    let metadata = fs::metadata(&test_file)?;
    let restored_mode = metadata.permissions().mode() & 0o777;
    assert_eq!(
        restored_mode, 0o644,
        "Normal permissions should be preserved through add→commit→restore cycle"
    );

    Ok(())
}

#[test]
#[serial]
fn test_config_allows_disabling_permission_stripping() -> Result<()> {
    let (home_dir, repo_dir, _ctx) = setup_test_env()?;

    // Create config file with stripping disabled
    let config_dir = TempDir::new()?;
    let config_path = config_dir.path().join("config");

    let config_content = format!(
        r#"[security]
allowed_directories = ["{}"]
enforce_path_validation = true
strip_dangerous_permissions = false
max_file_mode = 0o777
"#,
        home_dir.path().display()
    );

    std::fs::write(&config_path, config_content)?;

    let ctx_no_strip = DotmanContext::new_explicit(repo_dir.path().to_path_buf(), config_path)?;

    // Initialize repository structure
    std::fs::create_dir_all(ctx_no_strip.repo_path.clone())?;
    std::fs::create_dir_all(ctx_no_strip.repo_path.join("refs/heads"))?;
    std::fs::create_dir_all(ctx_no_strip.repo_path.join("refs/tags"))?;
    std::fs::create_dir_all(ctx_no_strip.repo_path.join("commits"))?;
    std::fs::create_dir_all(ctx_no_strip.repo_path.join("objects"))?;

    let index_path = ctx_no_strip.repo_path.join("index.bin");
    let index = dotman::storage::index::Index::new();
    index.save(&index_path)?;

    std::fs::write(
        ctx_no_strip.repo_path.join("HEAD"),
        "ref: refs/heads/main\n",
    )?;

    // Verify config setting
    assert!(
        !ctx_no_strip.config.security.strip_dangerous_permissions,
        "Config should have stripping disabled"
    );

    Ok(())
}
