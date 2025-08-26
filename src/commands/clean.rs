use crate::storage::index::Index;
use crate::{DotmanContext, INDEX_FILE};
use anyhow::Result;
use colored::Colorize;
use std::collections::HashSet;
use std::path::PathBuf;
use walkdir::WalkDir;

pub fn execute(ctx: &DotmanContext, dry_run: bool, force: bool) -> Result<()> {
    ctx.ensure_repo_exists()?;

    // Safety check: require either -n or -f flag
    if !dry_run && !force {
        super::print_error("clean requires either -n (dry run) or -f (force) flag for safety");
        super::print_info("Use 'dot clean -n' to see what would be removed");
        super::print_info("Use 'dot clean -f' to actually remove untracked files");
        return Ok(());
    }

    // Load index to get tracked files
    let index_path = ctx.repo_path.join(INDEX_FILE);
    let index = Index::load(&index_path)?;

    // Find untracked files
    let untracked = find_untracked_files(ctx, &index)?;

    if untracked.is_empty() {
        super::print_info("Already clean - no untracked files found");
        return Ok(());
    }

    // Display what will be/was removed
    if dry_run {
        println!(
            "\n{}",
            "Would remove the following untracked files:"
                .yellow()
                .bold()
        );
    } else {
        println!("\n{}", "Removing untracked files:".red().bold());
    }

    let mut removed_count = 0;
    let mut failed_count = 0;

    for path in &untracked {
        if dry_run {
            println!("  {} {}", "would remove:".yellow(), path.display());
            removed_count += 1;
        } else {
            // Actually remove the file
            match std::fs::remove_file(path) {
                Ok(_) => {
                    println!("  {} {}", "removed:".red(), path.display());
                    removed_count += 1;
                }
                Err(e) => {
                    super::print_warning(&format!("Failed to remove {}: {}", path.display(), e));
                    failed_count += 1;
                }
            }
        }
    }

    // Print summary
    println!();
    if dry_run {
        super::print_info(&format!(
            "{} untracked file(s) would be removed",
            removed_count
        ));
        super::print_info("Run 'dot clean -f' to actually remove these files");
    } else {
        super::print_success(&format!("Removed {} untracked file(s)", removed_count));
        if failed_count > 0 {
            super::print_warning(&format!("Failed to remove {} file(s)", failed_count));
        }
    }

    Ok(())
}

fn find_untracked_files(ctx: &DotmanContext, index: &Index) -> Result<Vec<PathBuf>> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;

    // Create a set of tracked paths for quick lookup
    let tracked_paths: HashSet<PathBuf> = index.entries.keys().map(|p| home.join(p)).collect();

    let mut untracked = Vec::new();

    // Walk through home directory
    for entry in WalkDir::new(&home)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let path = e.path();
            // Skip hidden directories (except tracked ones)
            if path != home
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with('.'))
                    .unwrap_or(false)
            {
                return false;
            }
            // Skip the dotman repo itself
            if path == ctx.repo_path {
                return false;
            }
            true
        })
        .flatten()
    {
        let path = entry.path();
        if entry.file_type().is_file() && !tracked_paths.contains(path) {
            // Check against ignore patterns
            let relative_path = path.strip_prefix(&home).unwrap_or(path);
            if !crate::utils::should_ignore(relative_path, &ctx.config.tracking.ignore_patterns) {
                untracked.push(path.to_path_buf());
            }
        }
    }

    Ok(untracked)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::collections::HashMap;
    use tempfile::tempdir;

    #[test]
    fn test_find_untracked_files() -> Result<()> {
        let dir = tempdir()?;
        let repo_path = dir.path().join(".dotman");
        std::fs::create_dir_all(&repo_path)?;

        // Create a mock index with one tracked file
        let _index = Index {
            version: 1,
            entries: HashMap::new(),
        };

        // Note: In a real test, we'd need to handle the home directory properly
        // This is a simplified test that shows the structure

        let config = Config::default();
        let _ctx = DotmanContext {
            repo_path,
            config_path: dir.path().join("config"),
            config,
        };

        // This test is limited since it would scan the actual home directory
        // In a real test environment, we'd mock the home directory

        Ok(())
    }

    #[test]
    fn test_safety_check() -> Result<()> {
        let dir = tempdir()?;
        let repo_path = dir.path().join(".dotman");
        std::fs::create_dir_all(&repo_path)?;

        // Create empty index
        let index = Index {
            version: 1,
            entries: HashMap::new(),
        };
        let index_path = repo_path.join(INDEX_FILE);
        index.save(&index_path)?;

        let config = Config::default();
        let ctx = DotmanContext {
            repo_path,
            config_path: dir.path().join("config"),
            config,
        };

        // Test that neither dry_run nor force returns safely without error
        let result = execute(&ctx, false, false);
        assert!(result.is_ok());

        Ok(())
    }
}
