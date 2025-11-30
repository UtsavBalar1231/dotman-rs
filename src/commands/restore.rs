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
pub fn execute(
    ctx: &DotmanContext,
    paths: &[String],
    source: Option<&str>,
    dry_run: bool,
) -> Result<()> {
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

    // Get home directory as base for relative paths
    let home = dirs::home_dir().context("Could not find home directory")?;

    if dry_run {
        preview_restore(&snapshot, paths, &home, display_commit);
        return Ok(());
    }

    output::info(&format!(
        "Restoring files from commit {}",
        display_commit.yellow()
    ));

    let mut restored_count = 0;
    let mut not_found = Vec::new();

    let mut progress = output::start_progress("Restoring files", paths.len());
    for (i, path_str) in paths.iter().enumerate() {
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
            permissions.apply_to_path(
                &target_path,
                ctx.config.tracking.preserve_permissions,
                false,
            )?;

            println!("  {} {}", "✓".green(), target_path.display());
            restored_count += 1;
        } else {
            not_found.push(path_str.clone());
        }
        progress.update(i + 1);
    }
    progress.finish();

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

/// Preview what files would be restored
fn preview_restore(
    snapshot: &crate::storage::snapshots::Snapshot,
    paths: &[String],
    home: &std::path::Path,
    display_commit: &str,
) {
    println!("\n{}", "Dry run - would restore:".yellow().bold());
    println!(
        "  {} Source: commit {}",
        "→".dimmed(),
        display_commit.yellow()
    );

    let mut would_restore = Vec::new();
    let mut not_found = Vec::new();

    for path_str in paths {
        let path = PathBuf::from(path_str);
        let relative_path = if path.is_absolute() {
            path.strip_prefix(home).unwrap_or(&path).to_path_buf()
        } else {
            path.clone()
        };

        if snapshot.files.contains_key(&relative_path) {
            let target_path = if path.is_absolute() {
                path.clone()
            } else {
                home.join(&path)
            };
            would_restore.push(target_path);
        } else {
            not_found.push(path_str.clone());
        }
    }

    if !would_restore.is_empty() {
        println!(
            "  {} {} file(s) would be restored:",
            "→".dimmed(),
            would_restore.len()
        );
        for path in &would_restore {
            println!("    {} {}", "✓".green(), path.display());
        }
    }

    if !not_found.is_empty() {
        println!(
            "  {} {} file(s) not found in commit:",
            "⚠".yellow(),
            not_found.len()
        );
        for path in &not_found {
            println!("    {} {}", "✗".red(), path);
        }
    }

    println!("\n{}", "Run without --dry-run to execute".dimmed());
}
