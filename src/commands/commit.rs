use crate::storage::index::Index;
use crate::storage::snapshots::SnapshotManager;
use crate::storage::{Commit, FileEntry};
use crate::utils::{get_current_timestamp, get_current_user, hash::hash_bytes};
use crate::{DotmanContext, INDEX_FILE};
use anyhow::Result;
use colored::Colorize;

pub fn execute(ctx: &DotmanContext, message: &str, all: bool) -> Result<()> {
    ctx.ensure_repo_exists()?;

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
    }

    // Create commit ID from timestamp and message hash
    let timestamp = get_current_timestamp();
    let commit_id = format!(
        "{:016x}{}",
        timestamp,
        &hash_bytes(message.as_bytes())[..16]
    );

    // Get parent commit (if any)
    let parent = get_last_commit_id(ctx)?;

    // Create tree hash from all file hashes
    let mut tree_content = String::new();
    for (path, entry) in &index.entries {
        tree_content.push_str(&format!("{} {}\n", entry.hash, path.display()));
    }
    let tree_hash = hash_bytes(tree_content.as_bytes());

    // Create commit object
    let commit = Commit {
        id: commit_id.clone(),
        parent,
        message: message.to_string(),
        author: get_current_user(),
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

    // Show last 8 chars of commit ID for better uniqueness
    let display_id = if commit_id.len() >= 8 {
        &commit_id[commit_id.len() - 8..]
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

fn get_last_commit_id(ctx: &DotmanContext) -> Result<Option<String>> {
    let head_path = ctx.repo_path.join("HEAD");
    if head_path.exists() {
        let content = std::fs::read_to_string(&head_path)?;
        Ok(Some(content.trim().to_string()))
    } else {
        Ok(None)
    }
}

fn update_head(ctx: &DotmanContext, commit_id: &str) -> Result<()> {
    let head_path = ctx.repo_path.join("HEAD");
    std::fs::write(&head_path, commit_id)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
