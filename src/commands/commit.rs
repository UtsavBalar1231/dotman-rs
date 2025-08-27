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
    let index = Index::load(&index_path)?;

    if index.entries.is_empty() {
        super::print_warning("No files tracked. Use 'dot add' to track files first.");
        return Ok(());
    }

    // If --all flag is set, update all tracked files first
    if all {
        super::print_info("Updating all tracked files...");
        update_all_tracked_files(ctx)?;
    } else {
        // Without --all, check if there are any staged changes
        if !has_staged_changes(ctx)? {
            super::print_warning(
                "No changes staged for commit. Use 'dot add' to stage changes or 'dot commit --all' to commit all changes.",
            );
            return Ok(());
        }
    }

    // Get timestamp and author for commit
    let timestamp = get_current_timestamp();
    let author = get_current_user_with_config(&ctx.config);

    // Get parent commit (if any)
    let parent = get_last_commit_id(ctx)?;

    // Create tree hash from all file hashes
    let mut tree_content = String::new();
    for (path, entry) in &index.entries {
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

    let files: Vec<FileEntry> = index.entries.values().cloned().collect();
    snapshot_manager.create_snapshot(commit.clone(), &files)?;

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

    // If --all flag is set, update all tracked files first
    if all {
        super::print_info("Updating all tracked files...");
        update_all_tracked_files(ctx)?;
    }

    // Load current index
    let index_path = ctx.repo_path.join(INDEX_FILE);
    let index = Index::load(&index_path)?;

    if index.entries.is_empty() {
        super::print_warning("No files tracked. Use 'dot add' to track files first.");
        return Ok(());
    }

    // Use provided message or keep the original
    let commit_message = message.unwrap_or(&last_snapshot.commit.message);

    // Create tree hash from all file hashes
    let mut tree_content = String::new();
    for (path, entry) in &index.entries {
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
    let files: Vec<FileEntry> = index.entries.values().cloned().collect();
    snapshot_manager.create_snapshot(commit.clone(), &files)?;

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

fn update_all_tracked_files(ctx: &DotmanContext) -> Result<()> {
    let index_path = ctx.repo_path.join(INDEX_FILE);
    let mut index = Index::load(&index_path)?;

    // Get home directory for making paths relative
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;

    let mut updated = 0;
    let entries: Vec<_> = index.entries.keys().cloned().collect();

    for path in entries {
        // Need to convert relative path back to absolute for checking existence
        let abs_path = home.join(&path);
        if abs_path.exists()
            && let Ok(entry) = crate::commands::add::create_file_entry(&abs_path, &home)
        {
            index.add_entry(entry);
            updated += 1;
        }
    }

    if updated > 0 {
        index.save(&index_path)?;
        super::print_info(&format!("Updated {} tracked file(s)", updated));
    }

    Ok(())
}

fn has_staged_changes(ctx: &DotmanContext) -> Result<bool> {
    let index_path = ctx.repo_path.join(INDEX_FILE);
    let index = Index::load(&index_path)?;

    if index.entries.is_empty() {
        return Ok(false);
    }

    // If there's no HEAD file (first commit), we have staged changes if index has entries
    let head_path = ctx.repo_path.join("HEAD");
    if !head_path.exists() {
        return Ok(true);
    }

    // For now, always allow commits if index has entries
    // A more sophisticated check would compare index against HEAD commit
    Ok(true)
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
        };

        // Create an index with a file
        let mut index = Index::new();
        let test_file = PathBuf::from(".bashrc");
        index.add_entry(FileEntry {
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

        // Update the file in index
        index.add_entry(FileEntry {
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
