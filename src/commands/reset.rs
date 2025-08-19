use crate::storage::index::Index;
use crate::storage::snapshots::SnapshotManager;
use crate::{DotmanContext, INDEX_FILE};
use anyhow::{Context, Result};
use colored::Colorize;

pub fn execute(ctx: &DotmanContext, commit: &str, hard: bool, soft: bool) -> Result<()> {
    ctx.ensure_repo_exists()?;

    if hard && soft {
        anyhow::bail!("Cannot use both --hard and --soft flags");
    }

    let commit_id = if commit == "HEAD" {
        get_head(ctx)?.ok_or_else(|| anyhow::anyhow!("No commits yet"))?
    } else {
        commit.to_string()
    };

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    // Load the target snapshot
    let snapshot = snapshot_manager
        .load_snapshot(&commit_id)
        .with_context(|| format!("Failed to load commit: {}", commit_id))?;

    if hard {
        // Hard reset: update index and working directory
        super::print_info(&format!(
            "Hard reset to commit {}",
            commit_id[..8.min(commit_id.len())].yellow()
        ));

        // Restore files to working directory
        let home =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        snapshot_manager.restore_snapshot(&commit_id, &home)?;

        // Update index to match commit
        let mut index = Index::new();
        for (path, file) in &snapshot.files {
            index.add_entry(crate::storage::FileEntry {
                path: path.clone(),
                hash: file.hash.clone(),
                size: 0, // Will be updated on next status
                modified: snapshot.commit.timestamp,
                mode: file.mode,
            });
        }

        let index_path = ctx.repo_path.join(INDEX_FILE);
        index.save(&index_path)?;

        super::print_success(&format!(
            "Hard reset complete. Working directory and index updated to match commit {}",
            commit_id[..8.min(commit_id.len())].yellow()
        ));
    } else if soft {
        // Soft reset: only move HEAD, keep index and working directory
        super::print_info(&format!(
            "Soft reset to commit {}",
            commit_id[..8.min(commit_id.len())].yellow()
        ));

        super::print_success(&format!(
            "Soft reset complete. HEAD now points to commit {}",
            commit_id[..8.min(commit_id.len())].yellow()
        ));
    } else {
        // Mixed reset (default): update index but not working directory
        super::print_info(&format!(
            "Mixed reset to commit {}",
            commit_id[..8.min(commit_id.len())].yellow()
        ));

        // Update index to match commit
        let mut index = Index::new();
        for (path, file) in &snapshot.files {
            index.add_entry(crate::storage::FileEntry {
                path: path.clone(),
                hash: file.hash.clone(),
                size: 0,
                modified: snapshot.commit.timestamp,
                mode: file.mode,
            });
        }

        let index_path = ctx.repo_path.join(INDEX_FILE);
        index.save(&index_path)?;

        super::print_success(&format!(
            "Mixed reset complete. Index updated to match commit {}",
            commit_id[..8.min(commit_id.len())].yellow()
        ));
    }

    // Update HEAD to point to the new commit
    update_head(ctx, &commit_id)?;

    Ok(())
}

fn get_head(ctx: &DotmanContext) -> Result<Option<String>> {
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
