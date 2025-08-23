use crate::storage::FileStatus;
use crate::storage::index::{ConcurrentIndex, Index};
use crate::storage::snapshots::SnapshotManager;
use crate::{DotmanContext, INDEX_FILE};
use anyhow::Result;
use colored::Colorize;
use std::collections::HashSet;
use std::path::PathBuf;

pub fn execute(ctx: &DotmanContext, short: bool, show_untracked: bool) -> Result<()> {
    ctx.ensure_repo_exists()?;

    let index_path = ctx.repo_path.join(INDEX_FILE);
    let index = Index::load(&index_path)?;
    let concurrent_index = ConcurrentIndex::from_index(index.clone());

    // Get all current files in tracked directories
    let current_files = get_current_files(ctx)?;

    // Get status in parallel (this detects modified/deleted)
    let mut statuses = concurrent_index.get_status_parallel(&current_files);

    // If --untracked flag is set, scan for untracked files
    if show_untracked {
        let untracked = find_untracked_files(ctx, &index)?;
        for file in untracked {
            statuses.push(FileStatus::Untracked(file));
        }
    }

    // Check for added files (in index but not in last commit)
    let head_path = ctx.repo_path.join("HEAD");
    if head_path.exists() {
        // We have commits, compare index against last commit
        let last_commit_id = std::fs::read_to_string(&head_path)?.trim().to_string();
        let snapshot_manager =
            SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

        if let Ok(snapshot) = snapshot_manager.load_snapshot(&last_commit_id) {
            let committed_files: HashSet<_> = snapshot.files.keys().cloned().collect();

            // Files in index but not in last commit are "Added"
            for path in index.entries.keys() {
                if !committed_files.contains(path) {
                    // Remove from untracked if it's there
                    statuses.retain(|s| !matches!(s, FileStatus::Untracked(p) if p == path));
                    // Add as Added status
                    statuses.push(FileStatus::Added(path.clone()));
                }
            }
        }
    } else {
        // No commits yet, all indexed files are "Added"
        for path in index.entries.keys() {
            // Remove from untracked if it's there
            statuses.retain(|s| !matches!(s, FileStatus::Untracked(p) if p == path));
            // Add as Added status
            statuses.push(FileStatus::Added(path.clone()));
        }
    }

    if statuses.is_empty() {
        super::print_info("No changes detected");
        println!("Working directory clean");
        return Ok(());
    }

    // Sort statuses for consistent output
    statuses.sort_by_key(|s| (s.status_char(), s.path().to_path_buf()));

    if short {
        // Short format: just status char and path
        for status in statuses {
            println!("{} {}", status.status_char(), status.path().display());
        }
    } else {
        // Long format: grouped by status type
        print_status_group(
            &statuses,
            FileStatus::Added(PathBuf::new()),
            "Changes to be committed:",
            "new file",
        );
        print_status_group(
            &statuses,
            FileStatus::Modified(PathBuf::new()),
            "Changes not staged:",
            "modified",
        );
        print_status_group(
            &statuses,
            FileStatus::Deleted(PathBuf::new()),
            "Deleted files:",
            "deleted",
        );
        print_status_group(
            &statuses,
            FileStatus::Untracked(PathBuf::new()),
            "Untracked files:",
            "untracked",
        );
    }

    Ok(())
}

pub fn get_current_files(ctx: &DotmanContext) -> Result<Vec<PathBuf>> {
    let index_path = ctx.repo_path.join(INDEX_FILE);
    let index = Index::load(&index_path)?;

    let mut files = Vec::new();

    // Get home directory to convert relative paths to absolute
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;

    // Only return tracked files to check for modifications/deletions
    // We don't scan for untracked files - only explicitly added files are tracked
    for path in index.entries.keys() {
        // Convert relative path to absolute for file operations
        let abs_path = if path.is_relative() {
            home.join(path)
        } else {
            path.clone()
        };
        files.push(abs_path);
    }

    Ok(files)
}

fn find_untracked_files(ctx: &DotmanContext, index: &Index) -> Result<Vec<PathBuf>> {
    use walkdir::WalkDir;

    let mut untracked = Vec::new();
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;

    // Get set of tracked paths for quick lookup
    let tracked_paths: HashSet<PathBuf> = index
        .entries
        .keys()
        .map(|p| {
            if p.is_relative() {
                home.join(p)
            } else {
                p.clone()
            }
        })
        .collect();

    // Walk home directory but skip .dotman and other ignored directories
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

fn print_status_group(statuses: &[FileStatus], status_type: FileStatus, header: &str, label: &str) {
    let filtered: Vec<&FileStatus> = statuses
        .iter()
        .filter(|s| std::mem::discriminant(*s) == std::mem::discriminant(&status_type))
        .collect();

    if !filtered.is_empty() {
        println!("\n{}:", header.bold());
        for status in filtered {
            let color_label = match status {
                FileStatus::Added(_) => label.green(),
                FileStatus::Modified(_) => label.yellow(),
                FileStatus::Deleted(_) => label.red(),
                FileStatus::Untracked(_) => label.white(),
            };
            println!("  {}: {}", color_label, status.path().display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use tempfile::tempdir;

    #[test]
    fn test_get_current_files() -> Result<()> {
        let dir = tempdir()?;
        let repo_path = dir.path().join(".dotman");
        std::fs::create_dir_all(&repo_path)?;

        // Create test files
        std::fs::write(dir.path().join("file1.txt"), "content1")?;
        std::fs::write(dir.path().join("file2.txt"), "content2")?;
        std::fs::create_dir(dir.path().join(".git"))?;
        std::fs::write(dir.path().join(".git/config"), "git config")?;

        let mut config = Config::default();
        config.tracking.ignore_patterns = vec![".git".to_string()];

        let _ctx = DotmanContext {
            repo_path,
            config_path: dir.path().join("config"),
            config,
        };

        // Note: This test is limited since it would scan the actual home directory
        // In a real test environment, we'd mock the home directory

        Ok(())
    }
}
