use crate::DotmanContext;
use crate::output;
use crate::refs::resolver::RefResolver;
use crate::storage::snapshots::SnapshotManager;
use anyhow::{Context, Result};
use colored::Colorize;
use std::path::PathBuf;

/// Restore files from a specific commit
///
/// # Errors
///
/// Returns an error if:
/// - The repository is not initialized
/// - No files are specified
/// - The source reference cannot be resolved
/// - The specified commit does not exist
/// - Failed to restore files
pub fn execute(ctx: &DotmanContext, paths: &[String], source: Option<&str>) -> Result<()> {
    ctx.check_repo_initialized()?;

    if paths.is_empty() {
        return Err(anyhow::anyhow!("No files specified to restore"));
    }

    // Default to HEAD if no source is provided
    let source_ref = source.unwrap_or("HEAD");

    // Use the reference resolver to handle HEAD, HEAD~n, branches, and short hashes
    let resolver = RefResolver::new(ctx.repo_path.clone());
    let commit_id = resolver
        .resolve(source_ref)
        .with_context(|| format!("Failed to resolve reference: {source_ref}"))?;

    let snapshot_manager = SnapshotManager::with_permissions(
        ctx.repo_path.clone(),
        ctx.config.core.compression_level,
        ctx.config.tracking.preserve_permissions,
    );

    let snapshot = snapshot_manager
        .load_snapshot(&commit_id)
        .with_context(|| format!("Failed to load commit: {commit_id}"))?;

    let display_commit = if commit_id.len() >= 8 {
        &commit_id[..8]
    } else {
        &commit_id
    };

    output::info(&format!(
        "Restoring files from commit {}",
        display_commit.yellow()
    ));

    // Get home directory as base for relative paths
    let home = dirs::home_dir().context("Could not find home directory")?;

    let mut restored_count = 0;
    let mut not_found = Vec::new();

    for path_str in paths {
        let path = PathBuf::from(path_str);

        // Normalize the path - convert absolute to relative from home
        let relative_path = if path.is_absolute() {
            path.strip_prefix(&home).unwrap_or(&path).to_path_buf()
        } else {
            path.clone()
        };

        if let Some(snapshot_file) = snapshot.files.get(&relative_path) {
            // Determine the target path for restoration
            let target_path = if path.is_absolute() {
                path.clone()
            } else {
                home.join(&path)
            };

            // Create parent directories if needed
            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            // Restore the file content
            snapshot_manager.restore_file_content(&snapshot_file.content_hash, &target_path)?;

            // Restore file permissions using cross-platform module
            let permissions =
                crate::utils::permissions::FilePermissions::from_mode(snapshot_file.mode);
            permissions.apply_to_path(&target_path, ctx.config.tracking.preserve_permissions)?;

            println!("  {} {}", "✓".green(), target_path.display());
            restored_count += 1;
        } else {
            not_found.push(path_str.clone());
        }
    }

    // Report results
    if restored_count > 0 {
        output::success(&format!(
            "Restored {} file{} from commit {}",
            restored_count,
            if restored_count == 1 { "" } else { "s" },
            display_commit.yellow()
        ));
    }

    if !not_found.is_empty() {
        output::warning(&format!(
            "The following files were not found in commit {}: {}",
            display_commit.yellow(),
            not_found.join(", ")
        ));
    }

    if restored_count == 0 && !not_found.is_empty() {
        return Err(anyhow::anyhow!("No files were restored"));
    }

    Ok(())
}
