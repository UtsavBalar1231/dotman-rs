#![allow(clippy::too_many_lines)]
#![allow(clippy::indexing_slicing)] // Safe in test environment

use anyhow::Result;
use dotman::commands::context::CommandContext;
use dotman::refs::{RefManager, resolver::RefResolver};
use dotman::{DotmanContext, NULL_COMMIT_ID, commands};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

mod ref_resolver_tests {
    use super::*;

    fn setup_test_repo_with_commits() -> Result<(TempDir, DotmanContext, Vec<String>)> {
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

        let mut commit_ids = Vec::new();

        // Create multiple commits
        for i in 1..=3 {
            let file = temp_dir.path().join(format!("file{i}.txt"));
            fs::write(&file, format!("content {i}"))?;
            commands::add::execute(&ctx, &[file.to_string_lossy().into()], false, false)?;
            commands::commit::execute(&ctx, &format!("Commit {i}"), false)?;
            let resolver = ctx.create_ref_resolver();
            commit_ids.push(resolver.resolve("HEAD")?);
        }

        Ok((temp_dir, ctx, commit_ids))
    }

    #[test]
    fn test_resolve_head() -> Result<()> {
        let (_temp, ctx, commits) = setup_test_repo_with_commits()?;

        let resolver = RefResolver::new(ctx.repo_path);
        let head_commit = resolver.resolve("HEAD")?;

        assert_eq!(head_commit, commits[2]); // Latest commit

        Ok(())
    }

    #[test]
    fn test_resolve_head_parent() -> Result<()> {
        let (_temp, ctx, commits) = setup_test_repo_with_commits()?;

        let resolver = RefResolver::new(ctx.repo_path);

        // HEAD^ should be the parent
        let parent = resolver.resolve("HEAD^")?;
        assert_eq!(parent, commits[1]);

        // HEAD^^ should be grandparent
        let grandparent = resolver.resolve("HEAD^^")?;
        assert_eq!(grandparent, commits[0]);

        Ok(())
    }

    #[test]
    fn test_resolve_head_tilde() -> Result<()> {
        let (_temp, ctx, commits) = setup_test_repo_with_commits()?;

        let resolver = RefResolver::new(ctx.repo_path);

        // HEAD~1 should be the parent
        let parent = resolver.resolve("HEAD~1")?;
        assert_eq!(parent, commits[1]);

        // HEAD~2 should be grandparent
        let grandparent = resolver.resolve("HEAD~2")?;
        assert_eq!(grandparent, commits[0]);

        // HEAD~0 should be HEAD itself
        let head = resolver.resolve("HEAD~0")?;
        assert_eq!(head, commits[2]);

        Ok(())
    }

    #[test]
    fn test_resolve_branch_name() -> Result<()> {
        let (_temp, ctx, _commits) = setup_test_repo_with_commits()?;

        // Create a new branch
        commands::branch::create(&ctx, "feature", None)?;

        let resolver = RefResolver::new(ctx.repo_path);
        let resolved_main = resolver.resolve("main")?;
        let resolved_feature = resolver.resolve("feature")?;

        // Both should point to the same commit (created from HEAD)
        assert_eq!(resolved_main, resolved_feature);

        Ok(())
    }

    #[test]
    fn test_resolve_tag() -> Result<()> {
        let (_temp, ctx, commits) = setup_test_repo_with_commits()?;

        // Create a tag
        commands::tag::create(&ctx, "v1.0", None)?;

        let resolver = RefResolver::new(ctx.repo_path);
        let commit_id = resolver.resolve("v1.0")?;

        assert_eq!(commit_id, commits[2]); // Tag points to HEAD

        Ok(())
    }

    #[test]
    fn test_resolve_full_commit_id() -> Result<()> {
        let (_temp, ctx, commits) = setup_test_repo_with_commits()?;

        let resolver = RefResolver::new(ctx.repo_path);

        // Should resolve full commit ID
        let commit_id = resolver.resolve(&commits[0])?;
        assert_eq!(commit_id, commits[0]);

        Ok(())
    }

    #[test]
    fn test_resolve_short_commit_id() -> Result<()> {
        let (_temp, ctx, commits) = setup_test_repo_with_commits()?;

        let resolver = RefResolver::new(ctx.repo_path);

        // Should resolve short commit ID (first 8 chars)
        let short_id = &commits[1][..8];
        let commit_id = resolver.resolve(short_id)?;
        assert_eq!(commit_id, commits[1]);

        Ok(())
    }

    #[test]
    fn test_resolve_ref_format() -> Result<()> {
        let (_temp, ctx, _commits) = setup_test_repo_with_commits()?;

        let resolver = RefResolver::new(ctx.repo_path);

        // Test ref: refs/heads/main format
        let commit_id = resolver.resolve("ref: refs/heads/main")?;
        let head = resolver.resolve("HEAD")?;
        assert_eq!(commit_id, head);

        Ok(())
    }

    #[test]
    fn test_resolve_invalid_reference() -> Result<()> {
        let (_temp, ctx, _commits) = setup_test_repo_with_commits()?;

        let resolver = RefResolver::new(ctx.repo_path);

        // Should error on invalid references
        assert!(resolver.resolve("nonexistent").is_err());
        assert!(resolver.resolve("HEAD~100").is_err());
        assert!(resolver.resolve("HEAD^100").is_err());

        Ok(())
    }

    #[test]
    fn test_resolve_ambiguous_short_hash() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path().join(".dotman");
        let config_path = temp_dir.path().join(".config/dotman/config");

        let ctx = DotmanContext::new_explicit(repo_path.clone(), config_path)?;
        ctx.ensure_repo_exists()?;

        // Create commits with similar prefixes (this is artificial)
        let commits_dir = repo_path.join("commits");
        fs::write(
            commits_dir.join("aaaa1111222233334444555566667777.zst"),
            "dummy1",
        )?;
        fs::write(
            commits_dir.join("aaaa2222333344445555666677778888.zst"),
            "dummy2",
        )?;

        let resolver = RefResolver::new(repo_path);

        // Should error on ambiguous short hash
        let result = resolver.resolve("aaaa");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Ambiguous"));

        Ok(())
    }

    #[test]
    fn test_resolve_head_on_empty_repo() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path().join(".dotman");
        let config_path = temp_dir.path().join(".config/dotman/config");

        let ctx = DotmanContext::new_explicit(repo_path.clone(), config_path)?;
        ctx.ensure_repo_exists()?;

        // Initialize the repository properly
        let index = dotman::storage::index::Index::new();
        let index_path = repo_path.join("index.bin");
        index.save(&index_path)?;

        // Initialize refs structure (HEAD, branches)
        let ref_manager = dotman::refs::RefManager::new(repo_path.clone());
        ref_manager.init()?;

        let resolver = RefResolver::new(repo_path);

        // HEAD on empty repo should return NULL_COMMIT_ID
        let commit_id = resolver.resolve("HEAD")?;
        assert_eq!(commit_id, NULL_COMMIT_ID);

        Ok(())
    }
}

mod ref_manager_tests {
    use super::*;
    use dotman::storage::{Commit, snapshots::Snapshot};
    use dotman::utils::serialization;
    use std::collections::HashMap;
    use zstd::stream::encode_all;

    fn setup_test_repo() -> Result<(TempDir, PathBuf)> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path().join(".dotman");
        fs::create_dir_all(&repo_path)?;
        fs::create_dir_all(repo_path.join("refs/heads"))?;
        fs::create_dir_all(repo_path.join("refs/tags"))?;
        fs::create_dir_all(repo_path.join("refs/remotes"))?;
        fs::write(repo_path.join("HEAD"), "ref: refs/heads/main")?;
        fs::write(repo_path.join("refs/heads/main"), NULL_COMMIT_ID)?;

        Ok((temp_dir, repo_path))
    }

    #[test]
    fn test_get_current_branch() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;
        let ref_manager = RefManager::new(repo_path);

        let branch = ref_manager.current_branch()?;
        assert_eq!(branch, Some("main".to_string()));

        Ok(())
    }

    #[test]
    fn test_get_current_branch_detached() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;

        // Set HEAD to a direct commit
        fs::write(repo_path.join("HEAD"), "1234567890abcdef")?;

        let ref_manager = RefManager::new(repo_path);
        let result = ref_manager.current_branch()?;

        // When detached, current_branch should return None
        assert_eq!(result, None);

        Ok(())
    }

    #[test]
    fn test_create_branch() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;
        let ref_manager = RefManager::new(repo_path.clone());

        // Use a valid 32-hex-char commit ID (dotman uses xxHash3)
        let commit_id = "0123456789abcdef0123456789abcdef";
        ref_manager.create_branch("feature", Some(commit_id))?;

        let branch_file = repo_path.join("refs/heads/feature");
        assert!(branch_file.exists());

        let content = fs::read_to_string(branch_file)?;
        assert_eq!(content, commit_id);

        Ok(())
    }

    #[test]
    fn test_delete_branch() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;
        let ref_manager = RefManager::new(repo_path.clone());

        let commit_id = "0123456789abcdef0123456789abcdef";
        ref_manager.create_branch("temp", Some(commit_id))?;
        assert!(repo_path.join("refs/heads/temp").exists());

        ref_manager.delete_branch("temp")?;
        assert!(!repo_path.join("refs/heads/temp").exists());

        Ok(())
    }

    #[test]
    fn test_list_branches() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;
        let ref_manager = RefManager::new(repo_path);

        let commit1 = "1111111111111111111111111111111a";
        let commit2 = "2222222222222222222222222222222b";
        ref_manager.create_branch("feature1", Some(commit1))?;
        ref_manager.create_branch("feature2", Some(commit2))?;

        let branches = ref_manager.list_branches()?;
        assert!(branches.contains(&"main".to_string()));
        assert!(branches.contains(&"feature1".to_string()));
        assert!(branches.contains(&"feature2".to_string()));
        assert_eq!(branches.len(), 3);

        Ok(())
    }

    #[test]
    fn test_update_head() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;
        let ref_manager = RefManager::new(repo_path.clone());

        // Update to a branch
        ref_manager.set_head_to_branch("feature", None, None)?;
        let head_content = fs::read_to_string(repo_path.join("HEAD"))?;
        assert_eq!(head_content, "ref: refs/heads/feature");

        // Update to a commit
        ref_manager.set_head_to_commit("abcdef123456", None, None)?;
        let head_content = fs::read_to_string(repo_path.join("HEAD"))?;
        assert_eq!(head_content, "abcdef123456");

        Ok(())
    }

    #[test]
    fn test_get_branch_commit() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;
        let ref_manager = RefManager::new(repo_path);

        let commit_id = "fedcba9876543210fedcba9876543210";
        ref_manager.create_branch("feature", Some(commit_id))?;
        let commit = ref_manager.get_branch_commit("feature")?;
        assert_eq!(commit, commit_id);

        // Non-existent branch
        let result = ref_manager.get_branch_commit("nonexistent");
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_update_branch() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;
        let ref_manager = RefManager::new(repo_path.clone());

        // Use valid hex commit IDs and create valid snapshot files
        let old_commit = "abc123def456";
        let new_commit = "def789abc012";

        // Create minimal valid snapshots

        // Create minimal valid snapshot for old_commit
        let old_snapshot = Snapshot {
            commit: Commit {
                id: old_commit.to_string(),
                parents: vec![],
                message: "test".to_string(),
                author: "test".to_string(),
                timestamp: 0,
                tree_hash: "test".to_string(),
            },
            files: HashMap::new(),
        };

        // Create minimal valid snapshot for new_commit
        let new_snapshot = Snapshot {
            commit: Commit {
                id: new_commit.to_string(),
                parents: vec![old_commit.to_string()],
                message: "test".to_string(),
                author: "test".to_string(),
                timestamp: 1,
                tree_hash: "test2".to_string(),
            },
            files: HashMap::new(),
        };

        // Save snapshots
        let commits_dir = repo_path.join("commits");
        fs::create_dir_all(&commits_dir)?;

        let old_serialized = serialization::serialize(&old_snapshot)?;
        let old_compressed = encode_all(&old_serialized[..], 3)?;
        fs::write(
            commits_dir.join(format!("{old_commit}.zst")),
            old_compressed,
        )?;

        let new_serialized = serialization::serialize(&new_snapshot)?;
        let new_compressed = encode_all(&new_serialized[..], 3)?;
        fs::write(
            commits_dir.join(format!("{new_commit}.zst")),
            new_compressed,
        )?;

        ref_manager.create_branch("feature", Some(old_commit))?;
        ref_manager.update_branch("feature", new_commit)?;

        let content = fs::read_to_string(repo_path.join("refs/heads/feature"))?;
        assert_eq!(content, new_commit);

        Ok(())
    }

    #[test]
    fn test_create_tag() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;
        let ref_manager = RefManager::new(repo_path.clone());

        ref_manager.create_tag("v1.0", Some("commit123"))?;

        let tag_file = repo_path.join("refs/tags/v1.0");
        assert!(tag_file.exists());

        let content = fs::read_to_string(tag_file)?;
        assert_eq!(content, "commit123");

        Ok(())
    }

    #[test]
    fn test_delete_tag() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;
        let ref_manager = RefManager::new(repo_path.clone());

        ref_manager.create_tag("v1.0", Some("commit123"))?;
        assert!(repo_path.join("refs/tags/v1.0").exists());

        ref_manager.delete_tag("v1.0")?;
        assert!(!repo_path.join("refs/tags/v1.0").exists());

        Ok(())
    }

    #[test]
    fn test_list_tags() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;
        let ref_manager = RefManager::new(repo_path);

        ref_manager.create_tag("v1.0", Some("commit1"))?;
        ref_manager.create_tag("v2.0", Some("commit2"))?;
        ref_manager.create_tag("v1.1", Some("commit3"))?;

        let tags = ref_manager.list_tags()?;
        assert_eq!(tags.len(), 3);
        assert!(tags.contains(&"v1.0".to_string()));
        assert!(tags.contains(&"v2.0".to_string()));
        assert!(tags.contains(&"v1.1".to_string()));

        Ok(())
    }

    #[test]
    fn test_get_tag_commit() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;
        let ref_manager = RefManager::new(repo_path);

        ref_manager.create_tag("v1.0", Some("commit123"))?;
        let commit = ref_manager.get_tag_commit("v1.0")?;
        assert_eq!(commit, "commit123");

        // Non-existent tag
        let result = ref_manager.get_tag_commit("nonexistent");
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_remote_refs() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;
        let _ref_manager = RefManager::new(repo_path.clone());

        // Create remote tracking branches
        let origin_dir = repo_path.join("refs/remotes/origin");
        fs::create_dir_all(&origin_dir)?;
        fs::write(origin_dir.join("main"), "remote_commit")?;

        // Check remote branch exists by reading directory directly
        // (list_remote_branches is not implemented in RefManager)
        let remote_branches: Vec<String> = fs::read_dir(&origin_dir)?
            .filter_map(std::result::Result::ok)
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect();
        assert_eq!(remote_branches.len(), 1);
        assert!(remote_branches.contains(&"main".to_string()));

        Ok(())
    }

    #[test]
    fn test_branch_exists() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;
        let ref_manager = RefManager::new(repo_path);

        assert!(ref_manager.branch_exists("main"));
        assert!(!ref_manager.branch_exists("nonexistent"));

        let commit_id = "abcdef0123456789abcdef0123456789";
        ref_manager.create_branch("feature", Some(commit_id))?;
        assert!(ref_manager.branch_exists("feature"));

        Ok(())
    }

    #[test]
    fn test_tag_exists() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;
        let ref_manager = RefManager::new(repo_path);

        assert!(!ref_manager.tag_exists("v1.0"));

        ref_manager.create_tag("v1.0", Some("commit123"))?;
        assert!(ref_manager.tag_exists("v1.0"));

        Ok(())
    }
}

mod integration_tests {
    use super::*;

    #[test]
    fn test_ref_resolution_with_branches_and_tags() -> Result<()> {
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

        let ctx = DotmanContext::new_explicit(repo_path.clone(), config_path)?;
        ctx.ensure_repo_exists()?;

        // Initialize the repository properly
        let index = dotman::storage::index::Index::new();
        let index_path = ctx.repo_path.join("index.bin");
        index.save(&index_path)?;

        // Initialize refs structure (HEAD, branches)
        let ref_manager = dotman::refs::RefManager::new(ctx.repo_path.clone());
        ref_manager.init()?;

        // Create commits
        for i in 1..=3 {
            let file = temp_dir.path().join(format!("file{i}.txt"));
            fs::write(&file, format!("content {i}"))?;
            commands::add::execute(&ctx, &[file.to_string_lossy().into()], false, false)?;
            commands::commit::execute(&ctx, &format!("Commit {i}"), false)?;
        }

        let resolver = ctx.create_ref_resolver();
        let head_commit = resolver.resolve("HEAD")?;

        // Create branch and tag at HEAD
        commands::branch::create(&ctx, "develop", None)?;
        commands::tag::create(&ctx, "v1.0", None)?;

        // Create another commit
        let file = temp_dir.path().join("file4.txt");
        fs::write(&file, "content 4")?;
        commands::add::execute(&ctx, &[file.to_string_lossy().into()], false, false)?;
        commands::commit::execute(&ctx, "Commit 4", false)?;

        let resolver = RefResolver::new(repo_path);

        // Branch and tag should still point to old commit
        let branch_commit = resolver.resolve("develop")?;
        let tag_commit = resolver.resolve("v1.0")?;
        assert_eq!(branch_commit, head_commit);
        assert_eq!(tag_commit, head_commit);

        // HEAD should be newer
        let new_head = resolver.resolve("HEAD")?;
        assert_ne!(new_head, head_commit);

        Ok(())
    }

    #[test]
    fn test_complex_ref_operations() -> Result<()> {
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

        let ctx = DotmanContext::new_explicit(repo_path.clone(), config_path)?;
        ctx.ensure_repo_exists()?;

        // Initialize the repository properly
        let index = dotman::storage::index::Index::new();
        let index_path = ctx.repo_path.join("index.bin");
        index.save(&index_path)?;

        // Initialize refs structure (HEAD, branches)
        let ref_manager = dotman::refs::RefManager::new(ctx.repo_path.clone());
        ref_manager.init()?;

        // Create initial structure
        let file = temp_dir.path().join("initial.txt");
        fs::write(&file, "initial")?;
        commands::add::execute(&ctx, &[file.to_string_lossy().into()], false, false)?;
        commands::commit::execute(&ctx, "Initial commit", false)?;

        // Create feature branch
        commands::branch::create(&ctx, "feature", None)?;
        commands::checkout::execute(&ctx, "feature", true, false)?; // Use force since we know the state is clean

        // Commit on feature branch
        let feature_file = temp_dir.path().join("feature.txt");
        fs::write(&feature_file, "feature work")?;
        commands::add::execute(&ctx, &[feature_file.to_string_lossy().into()], false, false)?;
        commands::commit::execute(&ctx, "Feature commit", false)?;

        let resolver = ctx.create_ref_resolver();
        let feature_commit = resolver.resolve("HEAD")?;

        // Switch back to main
        commands::checkout::execute(&ctx, "main", true, false)?; // Use force since we know the state is clean

        // Commit on main
        let main_file = temp_dir.path().join("main.txt");
        fs::write(&main_file, "main work")?;
        commands::add::execute(&ctx, &[main_file.to_string_lossy().into()], false, false)?;
        commands::commit::execute(&ctx, "Main commit", false)?;

        let resolver = ctx.create_ref_resolver();
        let main_commit = resolver.resolve("HEAD")?;

        // Verify branches diverged
        assert_ne!(feature_commit, main_commit);

        let resolver = RefResolver::new(repo_path);
        assert_eq!(resolver.resolve("feature")?, feature_commit);
        assert_eq!(resolver.resolve("main")?, main_commit);

        Ok(())
    }
}
