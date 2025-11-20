#![allow(clippy::too_many_lines)]
#![allow(clippy::indexing_slicing)]

use anyhow::Result;
use dotman::DotmanContext;
use dotman::config::{Config, RemoteConfig, RemoteType};
use dotman::mapping::{CommitMapping, MappingManager};
use dotman::mirror::GitMirror;
use dotman::mirror::errors::GitError;
use dotman::refs::RefManager;
use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

/// Setup a test repository with basic structure
fn setup_test_repo() -> Result<(TempDir, DotmanContext)> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join(".dotman");
    let config_path = temp_dir.path().join(".config/dotman/config");

    let ctx = DotmanContext::new_with_explicit_paths(repo_path, config_path)?;
    ctx.ensure_repo_exists()?;

    let index = dotman::storage::index::Index::new();
    let index_path = ctx.repo_path.join("index.bin");
    index.save(&index_path)?;

    let ref_manager = RefManager::new(ctx.repo_path.clone());
    ref_manager.init()?;

    Ok((temp_dir, ctx))
}

/// Setup a test git repository (bare repo for testing)
fn setup_test_git_remote(temp_dir: &TempDir) -> Result<PathBuf> {
    let remote_path = temp_dir.path().join("remote.git");
    fs::create_dir_all(&remote_path)?;

    let output = std::process::Command::new("git")
        .args(["init", "--bare"])
        .current_dir(&remote_path)
        .output()?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Failed to init bare git repo: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(remote_path)
}

mod mirror_tests {
    use super::*;

    #[test]
    #[serial]
    fn test_mirror_init_race_condition() -> Result<()> {
        let (temp_dir, ctx) = setup_test_repo()?;
        let remote_path = setup_test_git_remote(&temp_dir)?;
        let remote_url = format!("file://{}", remote_path.display());

        let repo_path = ctx.repo_path.clone();
        let config = ctx.config;

        let success_count = Arc::new(Mutex::new(0));
        let error_count = Arc::new(Mutex::new(0));

        let mut handles = vec![];

        for i in 0..5 {
            let repo_path = repo_path.clone();
            let remote_url = remote_url.clone();
            let config = config.clone();
            let success_count = Arc::clone(&success_count);
            let error_count = Arc::clone(&error_count);

            let handle = thread::spawn(move || {
                thread::sleep(Duration::from_millis(i * 10));

                let mirror = GitMirror::new(&repo_path, "origin", &remote_url, config);
                if matches!(mirror.init_mirror(), Ok(())) {
                    let mut count = success_count.lock().unwrap();
                    *count += 1;
                } else {
                    let mut count = error_count.lock().unwrap();
                    *count += 1;
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let success = *success_count.lock().unwrap();
        let errors = *error_count.lock().unwrap();

        // All threads should succeed due to lock-based coordination
        // First thread creates, others verify existing mirror
        assert_eq!(success, 5, "All mirror init attempts should succeed");
        assert_eq!(errors, 0, "No init attempts should fail");

        let mirror_path = repo_path.join("mirrors/origin");
        assert!(mirror_path.exists(), "Mirror directory should exist");

        Ok(())
    }

    #[test]
    #[serial]
    fn test_mirror_cleanup_retry() -> Result<()> {
        let (temp_dir, ctx) = setup_test_repo()?;
        let remote_path = setup_test_git_remote(&temp_dir)?;
        let remote_url = format!("file://{}", remote_path.display());

        let mirror = GitMirror::new(&ctx.repo_path, "origin", &remote_url, ctx.config.clone());
        mirror.init_mirror()?;

        // Add some test files to the mirror
        let mirror_path = mirror.get_mirror_path();
        let test_file = mirror_path.join("test.txt");
        fs::write(&test_file, "test content")?;

        let test_dir = mirror_path.join("subdir");
        fs::create_dir_all(&test_dir)?;
        fs::write(test_dir.join("file.txt"), "nested content")?;

        // Clear working directory (with retry logic)
        mirror.clear_working_directory()?;

        // Verify cleanup succeeded
        assert!(!test_file.exists(), "Test file should be removed");
        assert!(!test_dir.exists(), "Test directory should be removed");

        let git_dir = mirror_path.join(".git");
        assert!(git_dir.exists(), ".git directory should remain");

        Ok(())
    }

    #[test]
    #[serial]
    fn test_mirror_verify_existing() -> Result<()> {
        let (temp_dir, ctx) = setup_test_repo()?;
        let remote_path = setup_test_git_remote(&temp_dir)?;
        let remote_url = format!("file://{}", remote_path.display());

        let mirror = GitMirror::new(&ctx.repo_path, "origin", &remote_url, ctx.config.clone());
        mirror.init_mirror()?;

        // Initialize again should succeed (verification)
        mirror.init_mirror()?;

        let mirror_path = mirror.get_mirror_path();
        assert!(mirror_path.exists());
        assert!(mirror_path.join(".git").exists());

        Ok(())
    }
}

mod git_error_tests {
    use super::*;

    #[test]
    fn test_git_error_network_detection() {
        let test_cases = vec![
            "fatal: Could not resolve host: github.com",
            "fatal: Connection timed out",
            "error: network is unreachable",
            "fatal: Failed to connect to github.com",
            "error: Connection refused",
        ];

        for stderr in test_cases {
            let error = GitError::from_stderr("git fetch", stderr);
            assert!(
                matches!(error, GitError::Network(_)),
                "Should detect network error for: {stderr}"
            );
            assert!(error.should_retry(), "Network errors should be retryable");
            assert_eq!(error.error_type(), "Network Error");
        }
    }

    #[test]
    fn test_git_error_authentication_detection() {
        let test_cases = vec![
            "fatal: Authentication failed for 'https://github.com/user/repo.git'",
            "error: Permission denied (publickey)",
            "fatal: Access denied",
            "error: Invalid credentials",
            "fatal: Could not read Username",
        ];

        for stderr in test_cases {
            let error = GitError::from_stderr("git push", stderr);
            assert!(
                matches!(error, GitError::Authentication(_)),
                "Should detect auth error for: {stderr}"
            );
            assert!(!error.should_retry(), "Auth errors should not be retryable");
            assert_eq!(error.error_type(), "Authentication Error");
        }
    }

    #[test]
    fn test_git_error_conflict_detection() {
        let test_cases = vec![
            "error: failed to push some refs\nhint: Updates were rejected because the tip is behind",
            "error: non-fast-forward update",
            "CONFLICT (content): Merge conflict in file.txt",
            "error: Merge conflict in file.txt",
        ];

        for stderr in test_cases {
            let error = GitError::from_stderr("git push", stderr);
            assert!(
                matches!(error, GitError::Conflict(_)),
                "Should detect conflict error for: {stderr}"
            );
            assert_eq!(error.error_type(), "Conflict");
        }
    }

    #[test]
    fn test_git_error_not_found_detection() {
        let test_cases = vec![
            "fatal: Remote branch 'nonexistent' not found",
            "error: remote branch does not exist",
            "fatal: Couldn't find remote ref refs/heads/branch",
            "error: remote not found",
        ];

        for stderr in test_cases {
            let error = GitError::from_stderr("git fetch", stderr);
            assert!(
                matches!(error, GitError::NotFound(_)),
                "Should detect not-found error for: {stderr}"
            );
            assert_eq!(error.error_type(), "Not Found");
        }
    }

    #[test]
    fn test_git_error_permission_detection() {
        // Note: Avoid "permission denied" string as it matches Authentication first
        // Use patterns unique to Permission errors
        let test_cases = vec![
            "fatal: Unable to create temporary file",
            "error: cannot open .git/index.lock for writing",
            "error: Read-only file system",
        ];

        for stderr in test_cases {
            let error = GitError::from_stderr("git add", stderr);
            assert!(
                matches!(error, GitError::Permission(_)),
                "Should detect permission error for: {stderr}"
            );
            assert_eq!(error.error_type(), "Permission Denied");
        }
    }

    #[test]
    fn test_git_error_invalid_ref_detection() {
        let test_cases = vec![
            "fatal: invalid reference format: refs/heads/bad name",
            "error: malformed object name HEAD~999",
            "fatal: bad revision 'nonexistent'",
            "fatal: ambiguous argument 'HEAD~': unknown revision",
        ];

        for stderr in test_cases {
            let error = GitError::from_stderr("git show", stderr);
            assert!(
                matches!(error, GitError::InvalidRef(_)),
                "Should detect invalid-ref error for: {stderr}"
            );
            assert_eq!(error.error_type(), "Invalid Reference");
        }
    }

    #[test]
    fn test_git_error_unknown_fallback() {
        let stderr = "fatal: Something completely unexpected happened";
        let error = GitError::from_stderr("git unknown", stderr);
        assert!(
            matches!(error, GitError::Unknown(_)),
            "Should fallback to unknown error"
        );
        assert_eq!(error.error_type(), "Unknown Error");
        assert!(!error.should_retry());
    }

    #[test]
    fn test_git_error_user_messages() {
        let error = GitError::Network("test error".to_string());
        let msg = error.user_message();
        assert!(msg.contains("Suggestions:"));
        assert!(msg.contains("internet connection"));

        let error = GitError::Authentication("test error".to_string());
        let msg = error.user_message();
        assert!(msg.contains("SSH key"));
        assert!(msg.contains("credentials"));

        let error = GitError::Conflict("test error".to_string());
        let msg = error.user_message();
        assert!(msg.contains("--force"));
        assert!(msg.contains("pull"));
    }
}

mod remote_ref_tests {
    use super::*;

    #[test]
    #[serial]
    fn test_remote_ref_tracking() -> Result<()> {
        let (temp_dir, ctx) = setup_test_repo()?;
        let ref_manager = RefManager::new(ctx.repo_path.clone());

        // Create a test commit
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test")?;

        // Create a local branch
        ref_manager.create_branch("main", None)?;

        // Create remote refs directory
        let remote_refs_dir = ctx.repo_path.join("refs/remotes/origin");
        fs::create_dir_all(&remote_refs_dir)?;

        // Create remote tracking branch
        fs::write(remote_refs_dir.join("main"), "abc123def456")?;

        // Verify remote ref exists
        assert!(
            ref_manager.remote_ref_exists("origin", "main"),
            "Remote ref should exist"
        );

        // List remote refs
        let remote_refs = fs::read_dir(&remote_refs_dir)?
            .filter_map(std::result::Result::ok)
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect::<Vec<_>>();

        assert_eq!(remote_refs.len(), 1);
        assert!(remote_refs.contains(&"main".to_string()));

        // Delete remote refs
        ref_manager.delete_remote_refs("origin")?;

        assert!(
            !ref_manager.remote_ref_exists("origin", "main"),
            "Remote ref should be deleted"
        );
        assert!(
            !remote_refs_dir.exists(),
            "Remote refs dir should be deleted"
        );

        Ok(())
    }

    #[test]
    #[serial]
    fn test_multiple_remote_refs() -> Result<()> {
        let (_temp_dir, ctx) = setup_test_repo()?;
        let ref_manager = RefManager::new(ctx.repo_path.clone());

        // Create multiple remote tracking branches
        for remote in &["origin", "upstream"] {
            let remote_refs_dir = ctx.repo_path.join(format!("refs/remotes/{remote}"));
            fs::create_dir_all(&remote_refs_dir)?;

            for branch in &["main", "develop", "feature"] {
                fs::write(remote_refs_dir.join(branch), "commit123")?;
            }
        }

        // Verify all exist
        for remote in &["origin", "upstream"] {
            for branch in &["main", "develop", "feature"] {
                assert!(
                    ref_manager.remote_ref_exists(remote, branch),
                    "Remote ref {remote}/{branch} should exist"
                );
            }
        }

        // Delete one remote's refs
        ref_manager.delete_remote_refs("origin")?;

        // Verify origin refs deleted, upstream remains
        assert!(!ref_manager.remote_ref_exists("origin", "main"));
        assert!(ref_manager.remote_ref_exists("upstream", "main"));
        assert!(ref_manager.remote_ref_exists("upstream", "develop"));
        assert!(ref_manager.remote_ref_exists("upstream", "feature"));

        Ok(())
    }
}

mod mapping_tests {
    use super::*;

    #[test]
    #[serial]
    fn test_config_mapping_validation() -> Result<()> {
        let (_temp_dir, _ctx) = setup_test_repo()?;

        let mut mapping = CommitMapping::new();

        // Add mappings for remotes
        mapping.add_mapping("origin", "dotman123", "git456");
        mapping.add_mapping("upstream", "dotman789", "gitabc");
        mapping.add_mapping("nonexistent", "dotmanxyz", "git999");

        // Create config with only some remotes
        let mut config = Config::default();
        config.set_remote(
            "origin".to_string(),
            RemoteConfig {
                remote_type: RemoteType::Git,
                url: Some("https://example.com/repo.git".to_string()),
            },
        );
        config.set_remote(
            "upstream".to_string(),
            RemoteConfig {
                remote_type: RemoteType::Git,
                url: Some("https://example.com/upstream.git".to_string()),
            },
        );

        // Validate mapping against config
        let warnings = mapping.validate(&config)?;

        assert_eq!(warnings.len(), 1, "Should have one warning");
        assert!(
            warnings[0].contains("nonexistent"),
            "Warning should mention nonexistent remote"
        );
        assert!(
            warnings[0].contains("unknown remote"),
            "Warning should indicate unknown remote"
        );

        Ok(())
    }

    #[test]
    #[serial]
    fn test_mapping_branch_validation() -> Result<()> {
        let (_temp_dir, _ctx) = setup_test_repo()?;

        let mut mapping = CommitMapping::new();

        // Add branch mappings
        mapping.update_branch("main", "commit1", Some(("origin", "gitcommit1")));
        mapping.update_branch("feature", "commit2", Some(("nonexistent", "gitcommit2")));

        // Create config with only origin
        let mut config = Config::default();
        config.set_remote(
            "origin".to_string(),
            RemoteConfig {
                remote_type: RemoteType::Git,
                url: Some("https://example.com/repo.git".to_string()),
            },
        );

        // Validate mapping
        let warnings = mapping.validate(&config)?;

        assert_eq!(
            warnings.len(),
            1,
            "Should have one warning for branch mapping"
        );
        assert!(warnings[0].contains("feature"));
        assert!(warnings[0].contains("nonexistent"));

        Ok(())
    }

    #[test]
    #[serial]
    fn test_mapping_manager_locking() -> Result<()> {
        let (_temp_dir, ctx) = setup_test_repo()?;

        // Create first manager (acquires lock)
        let mut manager1 = MappingManager::new(&ctx.repo_path)?;
        manager1.add_and_save("origin", "commit1", "git1")?;

        // Try to create second manager (should wait for lock)
        let repo_path = ctx.repo_path;
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(100));
            MappingManager::new(&repo_path)
        });

        // Release first lock by dropping
        drop(manager1);

        // Second manager should now acquire lock
        let manager2 = handle.join().unwrap()?;
        assert!(
            manager2
                .mapping()
                .get_git_commit("origin", "commit1")
                .is_some()
        );

        Ok(())
    }

    #[test]
    #[serial]
    fn test_mapping_remove_remote() -> Result<()> {
        let mut mapping = CommitMapping::new();

        mapping.add_mapping("origin", "commit1", "git1");
        mapping.add_mapping("origin", "commit2", "git2");
        mapping.add_mapping("upstream", "commit3", "git3");
        mapping.update_branch("main", "commit1", Some(("origin", "git1")));
        mapping.update_branch("develop", "commit3", Some(("upstream", "git3")));

        // Remove origin
        mapping.remove_remote("origin");

        // Verify origin mappings removed
        assert!(mapping.get_git_commit("origin", "commit1").is_none());
        assert!(mapping.get_git_commit("origin", "commit2").is_none());

        // Verify upstream mappings remain
        assert!(mapping.get_git_commit("upstream", "commit3").is_some());

        // Verify branch mappings updated (indirectly through get_branch check)
        // Since git_heads is private, we verify through the branch existence
        assert!(
            mapping.get_branch("main").is_some(),
            "Main branch mapping should still exist"
        );
        assert!(
            mapping.get_branch("develop").is_some(),
            "Develop branch mapping should still exist"
        );

        Ok(())
    }

    #[test]
    #[serial]
    fn test_mapping_persistence() -> Result<()> {
        let (_temp_dir, ctx) = setup_test_repo()?;
        let _mapping_path = ctx.repo_path.join("remote-mappings.toml");

        {
            let mut manager = MappingManager::new(&ctx.repo_path)?;
            manager.add_and_save("origin", "dotman1", "git1")?;
            manager.add_and_save("origin", "dotman2", "git2")?;
            manager.update_branch_and_save("main", "dotman1", Some(("origin", "git1")))?;
        }

        // Load in new manager
        let manager = MappingManager::new(&ctx.repo_path)?;
        assert_eq!(
            manager.mapping().get_git_commit("origin", "dotman1"),
            Some("git1".to_string())
        );
        assert_eq!(
            manager.mapping().get_dotman_commit("origin", "git2"),
            Some("dotman2".to_string())
        );

        // Verify branch mapping exists (fields are private, so we can only check existence)
        assert!(
            manager.mapping().get_branch("main").is_some(),
            "Branch mapping should exist"
        );

        Ok(())
    }

    #[test]
    #[serial]
    fn test_mapping_corruption_recovery() -> Result<()> {
        let (_temp_dir, ctx) = setup_test_repo()?;
        let mapping_path = ctx.repo_path.join("remote-mappings.toml");
        let backup_path = mapping_path.with_extension("bak");

        // Create valid mapping - first save doesn't create backup
        {
            let mut manager = MappingManager::new(&ctx.repo_path)?;
            manager.add_and_save("origin", "commit1", "git1")?;
        }

        // Second save creates a backup from the first save
        {
            let mut manager = MappingManager::new(&ctx.repo_path)?;
            manager.add_and_save("origin", "commit2", "git2")?;
        }

        // Verify backup now exists
        assert!(
            backup_path.exists(),
            "Backup should exist after second save"
        );

        // Corrupt main file
        fs::write(&mapping_path, "invalid toml content ][")?;

        // Load should recover from backup
        let manager = MappingManager::new(&ctx.repo_path)?;

        // Should recover from backup (which has commit1 but not commit2)
        assert!(
            manager
                .mapping()
                .get_git_commit("origin", "commit1")
                .is_some(),
            "Should recover mapping from backup"
        );

        Ok(())
    }
}

#[test]
fn test_mapping_bidirectional_lookup() {
    let mut mapping = CommitMapping::new();

    mapping.add_mapping("origin", "dotman_abc", "git_xyz");

    assert_eq!(
        mapping.get_git_commit("origin", "dotman_abc"),
        Some("git_xyz".to_string())
    );
    assert_eq!(
        mapping.get_dotman_commit("origin", "git_xyz"),
        Some("dotman_abc".to_string())
    );

    assert!(mapping.is_pushed("origin", "dotman_abc"));
    assert!(!mapping.is_pushed("origin", "dotman_nonexistent"));
}

#[test]
fn test_mapping_multiple_remotes() {
    let mut mapping = CommitMapping::new();

    mapping.add_mapping("origin", "commit1", "git1");
    mapping.add_mapping("upstream", "commit1", "git2");

    assert_eq!(
        mapping.get_git_commit("origin", "commit1"),
        Some("git1".to_string())
    );
    assert_eq!(
        mapping.get_git_commit("upstream", "commit1"),
        Some("git2".to_string())
    );

    let origin_commits = mapping.get_mapped_commits("origin");
    let upstream_commits = mapping.get_mapped_commits("upstream");

    assert_eq!(origin_commits.len(), 1);
    assert_eq!(upstream_commits.len(), 1);
}
