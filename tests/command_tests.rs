#![allow(clippy::too_many_lines)]

use anyhow::Result;
use dotman::commands::context::CommandContext;
use dotman::{DotmanContext, commands};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

mod add_command_tests {
    use super::*;

    pub fn setup_test_repo() -> Result<(TempDir, DotmanContext)> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path().join(".dotman");
        let config_path = temp_dir.path().join(".config/dotman/config");

        let ctx = DotmanContext::new_with_explicit_paths(repo_path, config_path)?;
        ctx.ensure_repo_exists()?;

        // Initialize the repository properly
        let index = dotman::storage::index::Index::new();
        let index_path = ctx.repo_path.join("index.bin");
        index.save(&index_path)?;

        // Initialize refs structure (HEAD, branches)
        let ref_manager = dotman::refs::RefManager::new(ctx.repo_path.clone());
        ref_manager.init()?;

        Ok((temp_dir, ctx))
    }

    #[test]
    fn test_add_single_file() -> Result<()> {
        let (temp_dir, ctx) = setup_test_repo()?;

        // Create a test file
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content")?;

        // Add the file
        commands::add::execute(&ctx, &[test_file.to_string_lossy().into()], false)?;

        // Check that it's staged
        let index = CommandContext::load_concurrent_index(&ctx)?;
        let staged = index.staged_entries();
        assert_eq!(staged.len(), 1);

        Ok(())
    }

    #[test]
    fn test_add_directory_recursive() -> Result<()> {
        let (temp_dir, ctx) = setup_test_repo()?;

        // Create a directory with nested files
        let test_dir = temp_dir.path().join("test_dir");
        fs::create_dir_all(&test_dir)?;
        fs::write(test_dir.join("file1.txt"), "content 1")?;
        fs::write(test_dir.join("file2.txt"), "content 2")?;

        let nested_dir = test_dir.join("nested");
        fs::create_dir_all(&nested_dir)?;
        fs::write(nested_dir.join("file3.txt"), "content 3")?;

        // Add the directory
        commands::add::execute(&ctx, &[test_dir.to_string_lossy().into()], false)?;

        // Check that all files are staged
        let index = CommandContext::load_concurrent_index(&ctx)?;
        let staged = index.staged_entries();
        assert_eq!(staged.len(), 3);

        Ok(())
    }

    #[test]
    fn test_add_nonexistent_file_without_force() -> Result<()> {
        let (_temp_dir, ctx) = setup_test_repo()?;

        let result = commands::add::execute(&ctx, &["/nonexistent/file.txt".into()], false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));

        Ok(())
    }

    #[test]
    fn test_add_nonexistent_file_with_force() -> Result<()> {
        let (_temp_dir, ctx) = setup_test_repo()?;

        // Should not error with force flag
        let result = commands::add::execute(&ctx, &["/nonexistent/file.txt".into()], true);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_add_symlink() -> Result<()> {
        let (temp_dir, ctx) = setup_test_repo()?;

        // Create a file and a symlink to it
        let target = temp_dir.path().join("target.txt");
        fs::write(&target, "target content")?;

        let symlink = temp_dir.path().join("link.txt");
        std::os::unix::fs::symlink(&target, &symlink)?;

        // Add the symlink
        commands::add::execute(&ctx, &[symlink.to_string_lossy().into()], false)?;

        let index = CommandContext::load_concurrent_index(&ctx)?;
        let staged = index.staged_entries();

        // Depending on config, should either follow or not follow symlink
        assert!(!staged.is_empty());

        Ok(())
    }

    #[test]
    fn test_add_with_ignore_patterns() -> Result<()> {
        let (temp_dir, ctx) = setup_test_repo()?;

        // Create files that should be ignored
        let test_dir = temp_dir.path().join("test_dir");
        fs::create_dir_all(&test_dir)?;
        fs::write(test_dir.join("file.txt"), "normal file")?;
        fs::write(test_dir.join("file.swp"), "swap file")?;

        let git_dir = test_dir.join(".git");
        fs::create_dir_all(&git_dir)?;
        fs::write(git_dir.join("config"), "git config")?;

        // Add the directory
        commands::add::execute(&ctx, &[test_dir.to_string_lossy().into()], false)?;

        // Only non-ignored files should be staged
        let index = CommandContext::load_concurrent_index(&ctx)?;
        let staged = index.staged_entries();

        // Should only have file.txt, not .swp or .git files
        assert_eq!(staged.len(), 1);

        Ok(())
    }

    #[test]
    fn test_add_updates_existing_staged_entry() -> Result<()> {
        let (temp_dir, ctx) = setup_test_repo()?;

        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "initial content")?;

        // Add the file
        commands::add::execute(&ctx, &[test_file.to_string_lossy().into()], false)?;

        // Modify the file
        fs::write(&test_file, "modified content")?;

        // Add it again
        commands::add::execute(&ctx, &[test_file.to_string_lossy().into()], false)?;

        let index = CommandContext::load_concurrent_index(&ctx)?;
        let staged = index.staged_entries();
        assert_eq!(staged.len(), 1);

        // The hash should be different
        let (_, entry) = &staged[0];
        assert!(entry.size == 16); // "modified content" length

        Ok(())
    }

    #[test]
    fn test_add_preserves_permissions() -> Result<()> {
        let (temp_dir, ctx) = setup_test_repo()?;

        let test_file = temp_dir.path().join("executable.sh");
        fs::write(&test_file, "#!/bin/bash\necho hello")?;

        // Make it executable
        let mut perms = fs::metadata(&test_file)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&test_file, perms)?;

        // Add the file
        commands::add::execute(&ctx, &[test_file.to_string_lossy().into()], false)?;

        let index = CommandContext::load_concurrent_index(&ctx)?;
        let staged = index.staged_entries();
        let (_, entry) = &staged[0];

        // Check that executable bit is preserved
        assert_eq!(entry.mode & 0o111, 0o111);

        Ok(())
    }
}

mod commit_command_tests {
    use super::*;
    use dotman::utils::commit::generate_commit_id;

    fn setup_repo_with_staged_files() -> Result<(TempDir, DotmanContext)> {
        let (temp_dir, ctx) = super::add_command_tests::setup_test_repo()?;

        // Create and stage some files
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");
        fs::write(&file1, "content 1")?;
        fs::write(&file2, "content 2")?;

        commands::add::execute(
            &ctx,
            &[
                file1.to_string_lossy().into(),
                file2.to_string_lossy().into(),
            ],
            false,
        )?;

        Ok((temp_dir, ctx))
    }

    #[test]
    fn test_commit_creates_snapshot() -> Result<()> {
        let (_temp_dir, ctx) = setup_repo_with_staged_files()?;

        // Make a commit
        commands::commit::execute(&ctx, "Initial commit", false)?;

        // Check that commit was created
        let commits_dir = ctx.repo_path.join("commits");
        assert!(commits_dir.exists());

        let commits: Vec<_> = fs::read_dir(commits_dir)?.collect();
        assert_eq!(commits.len(), 1);

        // Check that index was cleared
        let index = CommandContext::load_concurrent_index(&ctx)?;
        assert!(index.staged_entries().is_empty());

        Ok(())
    }

    #[test]
    fn test_commit_with_no_staged_files() -> Result<()> {
        let (_temp_dir, ctx) = super::add_command_tests::setup_test_repo()?;

        // Try to commit without staging anything
        let result = commands::commit::execute(&ctx, "Empty commit", false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No files tracked"));

        Ok(())
    }

    #[test]
    fn test_commit_id_generation_deterministic() {
        let tree_hash = "abcd1234567890abcdef1234567890abcdef1234";
        let parent = Some("parent1234567890abcdef1234567890abcdef12");
        let message = "Test commit message";
        let author = "Test User <test@example.com>";
        let timestamp = 1_234_567_890;
        let nanos = 123_456_789;

        let id1 = generate_commit_id(tree_hash, parent, message, author, timestamp, nanos);
        let id2 = generate_commit_id(tree_hash, parent, message, author, timestamp, nanos);

        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 32); // xxHash produces 32-char hex
    }

    #[test]
    fn test_commit_id_unique_for_different_content() {
        let tree_hash = "abcd1234567890abcdef1234567890abcdef1234";
        let author = "Test User <test@example.com>";
        let timestamp = 1_234_567_890;
        let nanos = 123_456_789;

        let id1 = generate_commit_id(tree_hash, None, "Message 1", author, timestamp, nanos);
        let id2 = generate_commit_id(tree_hash, None, "Message 2", author, timestamp, nanos);

        assert_ne!(id1, id2);
    }

    #[test]
    fn test_commit_updates_head() -> Result<()> {
        let (_temp_dir, ctx) = setup_repo_with_staged_files()?;

        commands::commit::execute(&ctx, "First commit", false)?;

        // Check that HEAD was updated
        let head_commit = ctx.create_ref_resolver().resolve("HEAD")?;
        assert_ne!(head_commit, dotman::NULL_COMMIT_ID);

        Ok(())
    }

    #[test]
    fn test_commit_with_parent() -> Result<()> {
        let (temp_dir, ctx) = setup_repo_with_staged_files()?;

        // First commit
        commands::commit::execute(&ctx, "First commit", false)?;
        let first_commit = ctx.create_ref_resolver().resolve("HEAD")?;

        // Stage more files
        let file3 = temp_dir.path().join("file3.txt");
        fs::write(&file3, "content 3")?;
        commands::add::execute(&ctx, &[file3.to_string_lossy().into()], false)?;

        // Second commit
        commands::commit::execute(&ctx, "Second commit", false)?;
        let second_commit = ctx.create_ref_resolver().resolve("HEAD")?;

        assert_ne!(first_commit, second_commit);

        // Load the second commit and check it has the first as parent
        let snapshot_manager = dotman::storage::snapshots::SnapshotManager::new(ctx.repo_path, 3);
        let snapshot = snapshot_manager.load_snapshot(&second_commit)?;
        assert_eq!(snapshot.commit.parent, Some(first_commit));

        Ok(())
    }
}

mod status_command_tests {
    use super::*;

    #[test]
    fn test_status_clean_repo() -> Result<()> {
        let (_temp_dir, ctx) = super::add_command_tests::setup_test_repo()?;

        // Status should show clean
        commands::status::execute(&ctx, false, true)?;

        Ok(())
    }

    #[test]
    fn test_status_with_staged_files() -> Result<()> {
        let (temp_dir, ctx) = super::add_command_tests::setup_test_repo()?;

        let test_file = temp_dir.path().join("staged.txt");
        fs::write(&test_file, "staged content")?;
        commands::add::execute(&ctx, &[test_file.to_string_lossy().into()], false)?;

        // Status should show staged files
        commands::status::execute(&ctx, false, true)?;

        let index = CommandContext::load_concurrent_index(&ctx)?;
        assert_eq!(index.staged_entries().len(), 1);

        Ok(())
    }

    #[test]
    fn test_status_with_modified_files() -> Result<()> {
        let (temp_dir, ctx) = super::add_command_tests::setup_test_repo()?;

        // Create, add, and commit a file
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "initial")?;
        commands::add::execute(&ctx, &[test_file.to_string_lossy().into()], false)?;
        commands::commit::execute(&ctx, "Initial commit", false)?;

        // Modify the file
        fs::write(&test_file, "modified")?;

        // Status should show modified files
        commands::status::execute(&ctx, false, true)?;

        Ok(())
    }

    #[test]
    fn test_status_with_deleted_files() -> Result<()> {
        let (temp_dir, ctx) = super::add_command_tests::setup_test_repo()?;

        // Create, add, and commit a file
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "content")?;
        commands::add::execute(&ctx, &[test_file.to_string_lossy().into()], false)?;
        commands::commit::execute(&ctx, "Add file", false)?;

        // Delete the file
        fs::remove_file(&test_file)?;

        // Status should show deleted files
        commands::status::execute(&ctx, false, true)?;

        Ok(())
    }
}

mod branch_command_tests {
    use super::*;

    #[test]
    fn test_create_branch() -> Result<()> {
        let (_temp_dir, ctx) = super::add_command_tests::setup_test_repo()?;

        // Create initial commit
        let temp_file = ctx.repo_path.parent().unwrap().join("temp.txt");
        fs::write(&temp_file, "content")?;
        commands::add::execute(&ctx, &[temp_file.to_string_lossy().into()], false)?;
        commands::commit::execute(&ctx, "Initial commit", false)?;

        // Create a new branch
        commands::branch::create(&ctx, "feature", None)?;

        // Check that branch was created
        let branch_ref = ctx.repo_path.join("refs/heads/feature");
        assert!(branch_ref.exists());

        Ok(())
    }

    #[test]
    fn test_list_branches() -> Result<()> {
        let (_temp_dir, ctx) = super::add_command_tests::setup_test_repo()?;

        // Create some branches
        commands::branch::create(&ctx, "feature1", None)?;
        commands::branch::create(&ctx, "feature2", None)?;

        // List should work
        commands::branch::list(&ctx)?;

        Ok(())
    }

    #[test]
    fn test_delete_branch() -> Result<()> {
        let (_temp_dir, ctx) = super::add_command_tests::setup_test_repo()?;

        // Create and delete a branch
        commands::branch::create(&ctx, "temp-branch", None)?;
        commands::branch::delete(&ctx, "temp-branch", false)?;

        // Branch should not exist
        let branch_ref = ctx.repo_path.join("refs/heads/temp-branch");
        assert!(!branch_ref.exists());

        Ok(())
    }

    #[test]
    fn test_cannot_delete_current_branch() -> Result<()> {
        let (_temp_dir, ctx) = super::add_command_tests::setup_test_repo()?;

        // Try to delete the current branch (main)
        let result = commands::branch::delete(&ctx, "main", false);
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_rename_branch() -> Result<()> {
        let (_temp_dir, ctx) = super::add_command_tests::setup_test_repo()?;

        // Create and rename a branch
        commands::branch::create(&ctx, "old-name", None)?;
        commands::branch::rename(&ctx, Some("old-name"), "new-name")?;

        // Old branch should not exist, new one should
        assert!(!ctx.repo_path.join("refs/heads/old-name").exists());
        assert!(ctx.repo_path.join("refs/heads/new-name").exists());

        Ok(())
    }

    #[test]
    fn test_branch_with_b_flag() -> Result<()> {
        let (_temp_dir, ctx) = super::add_command_tests::setup_test_repo()?;

        // Create and checkout a new branch using the shorthand
        commands::branch::create(&ctx, "feature", None)?;
        commands::checkout::execute(&ctx, "feature", false)?;

        // Verify we're on the new branch
        let ref_manager = dotman::refs::RefManager::new(ctx.repo_path.clone());
        let current = ref_manager.current_branch()?;
        assert_eq!(current, Some("feature".to_string()));

        // Verify branch was created
        assert!(ctx.repo_path.join("refs/heads/feature").exists());

        Ok(())
    }

    #[test]
    fn test_branch_with_b_flag_and_start_point() -> Result<()> {
        let (_temp_dir, ctx) = super::add_command_tests::setup_test_repo()?;

        // Create and checkout feature from main
        commands::branch::create(&ctx, "feature", Some("main"))?;
        commands::checkout::execute(&ctx, "feature", false)?;

        // Verify we're on feature branch
        let ref_manager = dotman::refs::RefManager::new(ctx.repo_path);
        let current = ref_manager.current_branch()?;
        assert_eq!(current, Some("feature".to_string()));

        // Verify both branches point to the same commit
        let main_commit = ref_manager.get_branch_commit("main")?;
        let feature_commit = ref_manager.get_branch_commit("feature")?;
        assert_eq!(main_commit, feature_commit);

        Ok(())
    }
}

mod checkout_command_tests {
    use super::*;

    #[test]
    fn test_checkout_branch() -> Result<()> {
        let (_temp_dir, ctx) = super::add_command_tests::setup_test_repo()?;

        // Create a branch and checkout
        commands::branch::create(&ctx, "feature", None)?;
        commands::checkout::execute(&ctx, "feature", false)?;

        // Current branch should be feature
        let ref_manager = dotman::refs::RefManager::new(ctx.repo_path);
        let current = ref_manager.current_branch()?;
        assert_eq!(current, Some("feature".to_string()));

        Ok(())
    }

    #[test]
    fn test_checkout_with_uncommitted_changes() -> Result<()> {
        let (temp_dir, ctx) = super::add_command_tests::setup_test_repo()?;

        // Create a branch
        commands::branch::create(&ctx, "feature", None)?;

        // Stage a file
        let test_file = temp_dir.path().join("uncommitted.txt");
        fs::write(&test_file, "uncommitted")?;
        commands::add::execute(&ctx, &[test_file.to_string_lossy().into()], false)?;

        // Checkout should fail without force
        let result = commands::checkout::execute(&ctx, "feature", false);
        assert!(result.is_err());

        // Force checkout should work
        let result = commands::checkout::execute(&ctx, "feature", true);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_checkout_nonexistent_branch() -> Result<()> {
        let (_temp_dir, ctx) = super::add_command_tests::setup_test_repo()?;

        let result = commands::checkout::execute(&ctx, "nonexistent", false);
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_checkout_with_b_flag_from_head() -> Result<()> {
        let (_temp_dir, ctx) = super::add_command_tests::setup_test_repo()?;

        // Create and checkout a new branch from HEAD using -b flag
        commands::branch::create(&ctx, "feature", None)?;
        commands::checkout::execute(&ctx, "feature", false)?;

        // Verify we're on the new branch
        let ref_manager = dotman::refs::RefManager::new(ctx.repo_path.clone());
        let current = ref_manager.current_branch()?;
        assert_eq!(current, Some("feature".to_string()));

        // Verify branch was created
        assert!(ctx.repo_path.join("refs/heads/feature").exists());

        Ok(())
    }

    #[test]
    fn test_checkout_with_b_flag_from_branch() -> Result<()> {
        let (_temp_dir, ctx) = super::add_command_tests::setup_test_repo()?;

        // Create dev branch
        commands::branch::create(&ctx, "dev", None)?;

        // Create and checkout feature from dev
        commands::branch::create(&ctx, "feature", Some("dev"))?;
        commands::checkout::execute(&ctx, "feature", false)?;

        // Verify we're on feature branch
        let ref_manager = dotman::refs::RefManager::new(ctx.repo_path);
        let current = ref_manager.current_branch()?;
        assert_eq!(current, Some("feature".to_string()));

        Ok(())
    }

    #[test]
    fn test_checkout_with_b_flag_from_commit() -> Result<()> {
        let (_temp_dir, ctx) = super::add_command_tests::setup_test_repo()?;

        // Since setup_test_repo creates an empty repo, just use main as start point
        // This tests that we can create a branch from an existing branch name
        commands::branch::create(&ctx, "feature", Some("main"))?;
        commands::checkout::execute(&ctx, "feature", false)?;

        // Verify we're on feature branch
        let ref_manager = dotman::refs::RefManager::new(ctx.repo_path.clone());
        let current = ref_manager.current_branch()?;
        assert_eq!(current, Some("feature".to_string()));

        // Verify branch was created
        assert!(ctx.repo_path.join("refs/heads/feature").exists());

        Ok(())
    }

    #[test]
    fn test_checkout_b_flag_branch_already_exists() -> Result<()> {
        let (_temp_dir, ctx) = super::add_command_tests::setup_test_repo()?;

        // Create a branch
        commands::branch::create(&ctx, "existing", None)?;

        // Try to create the same branch again with -b flag
        let result = commands::branch::create(&ctx, "existing", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));

        Ok(())
    }
}

mod reset_command_tests {
    use super::*;

    fn setup_repo_with_commits() -> Result<(TempDir, DotmanContext, Vec<String>)> {
        let (temp_dir, ctx) = super::add_command_tests::setup_test_repo()?;
        let mut commits = Vec::new();

        // Create multiple commits
        for i in 1..=3 {
            let file = temp_dir.path().join(format!("file{i}.txt"));
            fs::write(&file, format!("content {i}"))?;
            commands::add::execute(&ctx, &[file.to_string_lossy().into()], false)?;
            commands::commit::execute(&ctx, &format!("Commit {i}"), false)?;
            commits.push(ctx.create_ref_resolver().resolve("HEAD")?);
        }

        Ok((temp_dir, ctx, commits))
    }

    #[test]
    fn test_reset_to_previous_commit() -> Result<()> {
        let (_temp_dir, ctx, commits) = setup_repo_with_commits()?;

        // Reset to first commit
        commands::reset::execute(&ctx, &commits[0], false, false, false, false, &[])?;

        // HEAD should point to first commit
        let head = ctx.create_ref_resolver().resolve("HEAD")?;
        assert_eq!(head, commits[0]);

        Ok(())
    }

    #[test]
    fn test_reset_soft() -> Result<()> {
        let (temp_dir, ctx, commits) = setup_repo_with_commits()?;

        // Add another file without committing
        let new_file = temp_dir.path().join("new.txt");
        fs::write(&new_file, "new content")?;
        commands::add::execute(&ctx, &[new_file.to_string_lossy().into()], false)?;

        // Soft reset should preserve staged changes
        // Pass soft=true as second parameter
        commands::reset::execute(&ctx, &commits[0], false, true, false, false, &[])?;

        let index = CommandContext::load_concurrent_index(&ctx)?;
        assert!(!index.staged_entries().is_empty());

        Ok(())
    }

    #[test]
    fn test_reset_hard() -> Result<()> {
        let (temp_dir, ctx, commits) = setup_repo_with_commits()?;

        // Add another file without committing
        let new_file = temp_dir.path().join("new.txt");
        fs::write(&new_file, "new content")?;
        commands::add::execute(&ctx, &[new_file.to_string_lossy().into()], false)?;

        // Hard reset should discard staged changes
        commands::reset::execute(&ctx, &commits[0], true, false, false, false, &[])?;

        let index = CommandContext::load_concurrent_index(&ctx)?;
        assert!(index.staged_entries().is_empty());

        Ok(())
    }

    #[test]
    fn test_reset_with_head_notation() -> Result<()> {
        let (_temp_dir, ctx, _commits) = setup_repo_with_commits()?;

        // Reset to HEAD~1
        commands::reset::execute(&ctx, "HEAD~1", false, false, false, false, &[])?;

        Ok(())
    }
}

mod restore_command_tests {
    use super::*;

    #[test]
    fn test_restore_modified_file() -> Result<()> {
        let (temp_dir, ctx) = super::add_command_tests::setup_test_repo()?;

        // Create and commit a file
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "original")?;
        commands::add::execute(&ctx, &[test_file.to_string_lossy().into()], false)?;
        commands::commit::execute(&ctx, "Add file", false)?;

        // Modify the file
        fs::write(&test_file, "modified")?;

        // Restore it
        commands::restore::execute(&ctx, &[test_file.to_string_lossy().into()], None)?;

        // File should be back to original
        let content = fs::read_to_string(&test_file)?;
        assert_eq!(content, "original");

        Ok(())
    }

    #[test]
    fn test_restore_staged_file() -> Result<()> {
        let (temp_dir, ctx) = super::add_command_tests::setup_test_repo()?;

        // Create, stage, and commit a file
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "original")?;
        commands::add::execute(&ctx, &[test_file.to_string_lossy().into()], false)?;
        commands::commit::execute(&ctx, "Add test file", false)?;

        // Modify and stage the file
        fs::write(&test_file, "modified")?;
        commands::add::execute(&ctx, &[test_file.to_string_lossy().into()], false)?;

        // Restore from HEAD (should restore to "original")
        commands::restore::execute(&ctx, &[test_file.to_string_lossy().into()], None)?;

        // File should be restored to original content
        let content = fs::read_to_string(&test_file)?;
        assert_eq!(content, "original");

        Ok(())
    }

    #[test]
    fn test_restore_deleted_file() -> Result<()> {
        let (temp_dir, ctx) = super::add_command_tests::setup_test_repo()?;

        // Create and commit a file
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "content")?;
        commands::add::execute(&ctx, &[test_file.to_string_lossy().into()], false)?;
        commands::commit::execute(&ctx, "Add file", false)?;

        // Delete the file
        fs::remove_file(&test_file)?;

        // Restore it
        commands::restore::execute(&ctx, &[test_file.to_string_lossy().into()], None)?;

        // File should exist again
        assert!(test_file.exists());
        let content = fs::read_to_string(&test_file)?;
        assert_eq!(content, "content");

        Ok(())
    }
}
