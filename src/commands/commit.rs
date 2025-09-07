use crate::refs::resolver::RefResolver;
use crate::storage::index::Index;
use crate::storage::snapshots::SnapshotManager;
use crate::storage::{Commit, FileEntry};
use crate::utils::{
    commit::generate_commit_id, get_current_timestamp, get_current_user_with_config,
    hash::hash_bytes,
};
use crate::{DotmanContext, INDEX_FILE};
use anyhow::{Context, Result};
use colored::Colorize;

pub fn execute(ctx: &DotmanContext, message: &str, all: bool) -> Result<()> {
    ctx.check_repo_initialized()?;

    let index_path = ctx.repo_path.join(INDEX_FILE);
    let mut index = Index::load(&index_path)?;

    if index.staged_entries.is_empty() && index.entries.is_empty() {
        super::print_warning("No files tracked. Use 'dot add' to track files first.");
        return Ok(());
    }

    // If --all flag is set, stage all modified tracked files first
    if all {
        super::print_info("Staging all tracked files...");
        stage_all_tracked_files(ctx, &mut index)?;
    }

    // Check if there are any staged changes
    if !index.has_staged_changes() {
        super::print_warning(
            "No changes staged for commit. Use 'dot add' to stage changes or 'dot commit --all' to commit all changes.",
        );
        return Ok(());
    }

    // Get timestamp and author for commit
    let timestamp = get_current_timestamp();
    let author = get_current_user_with_config(&ctx.config);

    // Get parent commit (if any)
    let parent = get_last_commit_id(ctx)?;

    // Create tree hash from all staged file hashes
    let mut tree_content = String::new();
    for (path, entry) in &index.staged_entries {
        tree_content.push_str(&format!("{} {}\n", entry.hash, path.display()));
    }
    let tree_hash = hash_bytes(tree_content.as_bytes());

    // Generate content-addressed commit ID
    let commit_id = generate_commit_id(&tree_hash, parent.as_deref(), message, &author, timestamp);

    // Create commit object
    let commit = Commit {
        id: commit_id.clone(),
        parent,
        message: message.to_string(),
        author,
        timestamp,
        tree_hash,
    };

    // Create snapshot
    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    let files: Vec<FileEntry> = index.staged_entries.values().cloned().collect();
    snapshot_manager.create_snapshot(commit.clone(), &files)?;

    // After successful commit, move staged entries to committed entries
    index.commit_staged();
    index.save(&index_path)?;

    // Update HEAD
    update_head(ctx, &commit_id)?;

    // Show first 8 chars of commit ID (Git-style)
    let display_id = if commit_id.len() >= 8 {
        &commit_id[..8]
    } else {
        &commit_id
    };
    super::print_success(&format!(
        "Created commit {} with {} files",
        display_id.yellow(),
        files.len()
    ));
    println!("  {}: {}", "Author".bold(), commit.author);
    println!("  {}: {}", "Message".bold(), commit.message);

    Ok(())
}

pub fn execute_amend(ctx: &DotmanContext, message: Option<&str>, all: bool) -> Result<()> {
    ctx.check_repo_initialized()?;

    // Get the last commit
    let resolver = RefResolver::new(ctx.repo_path.clone());
    let last_commit_id = resolver.resolve("HEAD").context("No commits to amend")?;

    // Load the last commit
    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);
    let last_snapshot = snapshot_manager
        .load_snapshot(&last_commit_id)
        .with_context(|| format!("Failed to load commit: {}", last_commit_id))?;

    // Load current index
    let index_path = ctx.repo_path.join(INDEX_FILE);
    let mut index = Index::load(&index_path)?;

    // If --all flag is set, stage all tracked files first
    if all {
        super::print_info("Staging all tracked files...");
        stage_all_tracked_files(ctx, &mut index)?;
    }

    if index.staged_entries.is_empty() {
        super::print_warning("No files staged. Use 'dot add' to stage files first.");
        return Ok(());
    }

    // Use provided message or keep the original
    let commit_message = message.unwrap_or(&last_snapshot.commit.message);

    // Create tree hash from all staged file hashes
    let mut tree_content = String::new();
    for (path, entry) in &index.staged_entries {
        tree_content.push_str(&format!("{} {}\n", entry.hash, path.display()));
    }
    let tree_hash = hash_bytes(tree_content.as_bytes());

    // Get commit details
    let timestamp = get_current_timestamp();
    let author = get_current_user_with_config(&ctx.config);

    // Generate new content-addressed commit ID for the amended commit
    let commit_id = generate_commit_id(
        &tree_hash,
        last_snapshot.commit.parent.as_deref(),
        commit_message,
        &author,
        timestamp,
    );

    // Create amended commit object with same parent as the original
    let commit = Commit {
        id: commit_id.clone(),
        parent: last_snapshot.commit.parent.clone(),
        message: commit_message.to_string(),
        author,
        timestamp,
        tree_hash,
    };

    // Delete the old snapshot
    snapshot_manager.delete_snapshot(&last_commit_id)?;

    // Create new snapshot with amended content
    let files: Vec<FileEntry> = index.staged_entries.values().cloned().collect();
    snapshot_manager.create_snapshot(commit.clone(), &files)?;

    // After successful commit, move staged entries to committed entries
    index.commit_staged();
    index.save(&index_path)?;

    // Update HEAD to point to the new commit ID since it's content-addressed
    update_head(ctx, &commit_id)?;

    let display_id = if commit_id.len() >= 8 {
        &commit_id[..8]
    } else {
        &commit_id
    };

    super::print_success(&format!(
        "Amended commit {} with {} files",
        display_id.yellow(),
        files.len()
    ));
    println!("  {}: {}", "Author".bold(), commit.author);
    println!("  {}: {}", "Message".bold(), commit.message);

    Ok(())
}

fn stage_all_tracked_files(_ctx: &DotmanContext, index: &mut Index) -> Result<()> {
    // Get home directory for making paths relative
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;

    let mut staged = 0;

    // Stage all currently tracked files with their current state
    let entries: Vec<_> = index.entries.keys().cloned().collect();
    for path in entries {
        // Need to convert relative path back to absolute for checking existence
        let abs_path = home.join(&path);
        if abs_path.exists()
            && let Ok(entry) = crate::commands::add::create_file_entry(&abs_path, &home)
        {
            index.stage_entry(entry);
            staged += 1;
        }
    }

    if staged > 0 {
        super::print_info(&format!("Staged {} tracked file(s)", staged));
    }

    Ok(())
}

fn get_last_commit_id(ctx: &DotmanContext) -> Result<Option<String>> {
    use crate::refs::RefManager;

    let ref_manager = RefManager::new(ctx.repo_path.clone());
    ref_manager.get_head_commit()
}

fn update_head(ctx: &DotmanContext, commit_id: &str) -> Result<()> {
    use crate::reflog::ReflogManager;
    use crate::refs::RefManager;

    let ref_manager = RefManager::new(ctx.repo_path.clone());
    let reflog_manager = ReflogManager::new(ctx.repo_path.clone());

    // Check if we're on a branch or detached HEAD
    if let Some(current_branch) = ref_manager.current_branch()? {
        // Get current HEAD value before updating
        let old_value = reflog_manager
            .get_current_head()
            .unwrap_or_else(|_| "0".repeat(40));

        // Update the current branch to point to the new commit
        ref_manager.update_branch(&current_branch, commit_id)?;

        // Log the reflog entry - for branch commits, we log the commit hash as new value
        reflog_manager.log_head_update(
            &old_value,
            commit_id,
            "commit",
            &format!("commit: {}", &commit_id[..8.min(commit_id.len())]),
        )?;
    } else {
        // Detached HEAD - update HEAD directly with reflog
        ref_manager.set_head_to_commit_with_reflog(
            commit_id,
            "commit",
            &format!("commit: {}", &commit_id[..8.min(commit_id.len())]),
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn test_get_last_commit_id() -> Result<()> {
        let dir = tempdir()?;
        let repo_path = dir.path().join(".dotman");
        std::fs::create_dir_all(&repo_path)?;

        let ctx = DotmanContext {
            repo_path: repo_path.clone(),
            config_path: dir.path().join("config"),
            config: Default::default(),
            no_pager: true,
        };

        // No HEAD file yet
        assert_eq!(get_last_commit_id(&ctx)?, None);

        // Create HEAD file
        std::fs::write(repo_path.join("HEAD"), "abc123")?;
        assert_eq!(get_last_commit_id(&ctx)?, Some("abc123".to_string()));

        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_execute_amend() -> Result<()> {
        use crate::config::Config;
        use crate::refs::RefManager;
        use std::fs;

        let dir = tempdir()?;
        let repo_path = dir.path().join(".dotman");
        fs::create_dir_all(&repo_path)?;
        fs::create_dir_all(repo_path.join("commits"))?;
        fs::create_dir_all(repo_path.join("objects"))?;
        fs::create_dir_all(repo_path.join("refs/heads"))?;

        // Initialize refs
        let ref_manager = RefManager::new(repo_path.clone());
        ref_manager.init()?;

        // Create context
        let config = Config::default();
        let config_path = dir.path().join("config.toml");
        config.save(&config_path)?;

        let ctx = DotmanContext {
            repo_path: repo_path.clone(),
            config_path,
            config,
            no_pager: true,
        };

        // Create an index with a file
        let mut index = Index::new();
        let test_file = PathBuf::from(".bashrc");
        // Stage the entry for commit
        index.stage_entry(FileEntry {
            path: test_file.clone(),
            hash: "test_hash_1".to_string(),
            size: 100,
            modified: 1234567890,
            mode: 0o644,
        });
        let index_path = repo_path.join(INDEX_FILE);
        index.save(&index_path)?;

        // Set HOME for the test
        unsafe {
            std::env::set_var("HOME", dir.path());
        }

        // Create the actual test file on disk
        fs::write(dir.path().join(".bashrc"), "test content")?;

        // Create first commit
        execute(&ctx, "Initial commit", false)?;

        // Load the index (it was modified by execute), update and stage the file
        let mut index = Index::load(&index_path)?;
        index.stage_entry(FileEntry {
            path: test_file,
            hash: "test_hash_2".to_string(),
            size: 200,
            modified: 1234567891,
            mode: 0o644,
        });
        index.save(&index_path)?;

        // Update the actual file on disk as well
        fs::write(dir.path().join(".bashrc"), "updated test content")?;

        // Amend the commit
        let result = execute_amend(&ctx, Some("Amended commit"), false);
        assert!(result.is_ok());

        Ok(())
    }
}
