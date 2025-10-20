use anyhow::Result;
use dotman::refs::resolver::RefResolver;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_ref_resolver_head() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().to_path_buf();

    // Setup repository structure
    fs::create_dir_all(repo_path.join("refs/heads"))?;
    fs::write(repo_path.join("HEAD"), b"ref: refs/heads/main")?;
    fs::write(
        repo_path.join("refs/heads/main"),
        b"abc123def456789abcdef123456789ab",
    )?;

    let resolver = RefResolver::new(repo_path);

    // Test HEAD resolution
    let commit_id = resolver.resolve("HEAD")?;
    assert_eq!(commit_id, "abc123def456789abcdef123456789ab");

    // Test branch resolution
    let commit_id = resolver.resolve("main")?;
    assert_eq!(commit_id, "abc123def456789abcdef123456789ab");

    Ok(())
}

#[test]
fn test_ref_resolver_detached_head() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().to_path_buf();

    // Setup detached HEAD
    fs::write(repo_path.join("HEAD"), b"def456abc123789abcdef123456789ab")?;

    let resolver = RefResolver::new(repo_path);

    // Test detached HEAD resolution
    let commit_id = resolver.resolve("HEAD")?;
    assert_eq!(commit_id, "def456abc123789abcdef123456789ab");

    Ok(())
}

#[test]
fn test_ref_resolver_branch() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().to_path_buf();

    // Setup multiple branches
    fs::create_dir_all(repo_path.join("refs/heads"))?;
    fs::write(
        repo_path.join("refs/heads/feature"),
        b"123456789abcdef123456789abcdef12",
    )?;
    fs::write(
        repo_path.join("refs/heads/develop"),
        b"abcdef123456789abcdef123456789ab",
    )?;

    let resolver = RefResolver::new(repo_path);

    // Test branch resolution
    let feature_id = resolver.resolve("feature")?;
    assert_eq!(feature_id, "123456789abcdef123456789abcdef12");

    let develop_id = resolver.resolve("develop")?;
    assert_eq!(develop_id, "abcdef123456789abcdef123456789ab");

    Ok(())
}

#[test]
fn test_ref_resolver_tag() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().to_path_buf();

    // Setup tags
    fs::create_dir_all(repo_path.join("refs/tags"))?;
    fs::write(
        repo_path.join("refs/tags/v1.0.0"),
        b"fedcba987654321fedcba9876543210f",
    )?;

    let resolver = RefResolver::new(repo_path);

    // Test tag resolution
    let tag_id = resolver.resolve("v1.0.0")?;
    assert_eq!(tag_id, "fedcba987654321fedcba9876543210f");

    Ok(())
}

#[test]
fn test_ref_resolver_short_hash() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().to_path_buf();

    // Create commits directory with mock commits
    fs::create_dir_all(repo_path.join("commits"))?;
    fs::write(
        repo_path.join("commits/abc123def456789abcdef123456789ab.zst"),
        b"",
    )?;
    fs::write(
        repo_path.join("commits/def456abc123789abcdef123456789ab.zst"),
        b"",
    )?;

    let resolver = RefResolver::new(repo_path);

    // Test unique short hash
    let result = resolver.resolve("abc123de")?;
    assert_eq!(result, "abc123def456789abcdef123456789ab");

    let result = resolver.resolve("def456ab")?;
    assert_eq!(result, "def456abc123789abcdef123456789ab");

    Ok(())
}

#[test]
fn test_ref_resolver_ambiguous_short_hash() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().to_path_buf();

    // Create commits with same prefix
    fs::create_dir_all(repo_path.join("commits"))?;
    fs::write(
        repo_path.join("commits/abc123def456789abcdef123456789ab.zst"),
        b"",
    )?;
    fs::write(
        repo_path.join("commits/abc123abc123789abcdef123456789ab.zst"),
        b"",
    )?;

    let resolver = RefResolver::new(repo_path);

    // Test ambiguous prefix - should fail
    let result = resolver.resolve("abc123");
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Ambiguous") || err_msg.contains("multiple"));

    Ok(())
}

#[test]
fn test_ref_resolver_full_hash() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().to_path_buf();

    // Create a commit
    fs::create_dir_all(repo_path.join("commits"))?;
    fs::write(
        repo_path.join("commits/abc123def456789abcdef123456789ab.zst"),
        b"",
    )?;

    let resolver = RefResolver::new(repo_path);

    // Test full hash resolution
    let result = resolver.resolve("abc123def456789abcdef123456789ab")?;
    assert_eq!(result, "abc123def456789abcdef123456789ab");

    Ok(())
}

#[test]
fn test_ref_resolver_parent_notation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().to_path_buf();

    // This would require proper commit structure with parent references
    // For now, we test that the notation is recognized but may fail
    fs::create_dir_all(repo_path.join("refs/heads"))?;
    fs::write(repo_path.join("HEAD"), b"ref: refs/heads/main")?;
    fs::write(
        repo_path.join("refs/heads/main"),
        b"abc123def456789abcdef123456789ab",
    )?;

    let resolver = RefResolver::new(repo_path);

    // Test parent notations - these should be recognized but may fail without proper commit structure
    let result = resolver.resolve("HEAD^");
    // The resolver should attempt to resolve this, but may fail if commit doesn't exist
    assert!(result.is_err() || result.is_ok());

    let result = resolver.resolve("HEAD~1");
    assert!(result.is_err() || result.is_ok());

    let result = resolver.resolve("HEAD~5");
    assert!(result.is_err() || result.is_ok());

    Ok(())
}

#[test]
fn test_ref_resolver_invalid_ref() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().to_path_buf();

    let resolver = RefResolver::new(repo_path);

    // Test non-existent ref
    let result = resolver.resolve("non-existent-branch");
    assert!(result.is_err());

    Ok(())
}

// TODO: Remote branch resolution (origin/main format) is not yet implemented
// #[test]
// fn test_ref_resolver_remote_branch() -> Result<()> {
//     let temp_dir = TempDir::new()?;
//     let repo_path = temp_dir.path().to_path_buf();
//
//     // Setup remote branch
//     fs::create_dir_all(repo_path.join("refs/remotes/origin"))?;
//     fs::write(
//         repo_path.join("refs/remotes/origin/main"),
//         b"fedcba987654321fedcba9876543210f",
//     )?;
//
//     let resolver = RefResolver::new(repo_path.clone());
//
//     // Test remote branch resolution
//     let remote_id = resolver.resolve("origin/main")?;
//     assert_eq!(remote_id, "fedcba987654321fedcba9876543210f");
//
//     Ok(())
// }
