use crate::DotmanContext;
use crate::storage::snapshots::SnapshotManager;
use anyhow::{Context, Result};
use colored::Colorize;

pub fn execute(ctx: &DotmanContext, target: &str, force: bool) -> Result<()> {
    ctx.ensure_repo_exists()?;

    // Check for uncommitted changes if not forcing
    if !force {
        let status_output = check_working_directory_clean(ctx)?;
        if !status_output {
            anyhow::bail!(
                "You have uncommitted changes. Use --force to override or commit your changes first."
            );
        }
    }

    // Resolve HEAD if needed
    let commit_id = if target == "HEAD" {
        let head_path = ctx.repo_path.join("HEAD");
        if head_path.exists() {
            std::fs::read_to_string(&head_path)?.trim().to_string()
        } else {
            anyhow::bail!("No commits yet (HEAD not found)");
        }
    } else {
        target.to_string()
    };

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    // Load the target snapshot
    let snapshot = snapshot_manager
        .load_snapshot(&commit_id)
        .with_context(|| format!("Failed to load commit: {}", commit_id))?;

    let display_target = if commit_id.len() >= 8 {
        &commit_id[commit_id.len() - 8..]
    } else {
        &commit_id
    };
    super::print_info(&format!("Checking out commit {}", display_target.yellow()));

    // Get home directory as target
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;

    // Restore files
    snapshot_manager.restore_snapshot(target, &home)?;

    // Update HEAD
    update_head(ctx, &commit_id)?;

    let display_id = if commit_id.len() >= 8 {
        &commit_id[commit_id.len() - 8..]
    } else {
        &commit_id
    };

    super::print_success(&format!(
        "Checked out commit {} ({} files restored)",
        display_id.yellow(),
        snapshot.files.len()
    ));

    println!("  {}: {}", "Author".bold(), snapshot.commit.author);
    println!("  {}: {}", "Message".bold(), snapshot.commit.message);

    Ok(())
}

fn check_working_directory_clean(ctx: &DotmanContext) -> Result<bool> {
    use crate::INDEX_FILE;
    use crate::commands::status::get_current_files;
    use crate::storage::index::{ConcurrentIndex, Index};

    let index_path = ctx.repo_path.join(INDEX_FILE);
    let index = Index::load(&index_path)?;
    let concurrent_index = ConcurrentIndex::from_index(index);

    let current_files = get_current_files(ctx)?;
    let statuses = concurrent_index.get_status_parallel(&current_files);

    Ok(statuses.is_empty())
}

fn update_head(ctx: &DotmanContext, commit_id: &str) -> Result<()> {
    let head_path = ctx.repo_path.join("HEAD");
    std::fs::write(&head_path, commit_id)?;
    Ok(())
}

// Helper to get current files - removed duplicate, use the one from status module
