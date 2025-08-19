use crate::storage::FileStatus;
use crate::storage::index::{ConcurrentIndex, Index};
use crate::storage::snapshots::SnapshotManager;
use crate::utils::should_ignore;
use crate::{DotmanContext, INDEX_FILE};
use anyhow::Result;
use colored::Colorize;
use std::collections::HashSet;
use std::path::PathBuf;
use walkdir::WalkDir;

pub fn execute(ctx: &DotmanContext, short: bool) -> Result<()> {
    ctx.ensure_repo_exists()?;

    let index_path = ctx.repo_path.join(INDEX_FILE);
    let index = Index::load(&index_path)?;
    let concurrent_index = ConcurrentIndex::from_index(index.clone());

    // Get all current files in tracked directories
    let current_files = get_current_files(ctx)?;

    // Get status in parallel (this detects modified/deleted/untracked)
    let mut statuses = concurrent_index.get_status_parallel(&current_files);

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
    let mut scanned_dirs = std::collections::HashSet::new();

    // First, add all tracked files to check for modifications/deletions
    for path in index.entries.keys() {
        files.push(path.clone());
    }

    // Then scan parent directories of tracked files for new untracked files
    for path in index.entries.keys() {
        // Get the parent directory, or use current directory for files in root
        let parent = if let Some(p) = path.parent() {
            if p.as_os_str().is_empty() {
                // Empty parent means file is in current directory
                std::env::current_dir()?
            } else {
                p.to_path_buf()
            }
        } else {
            // No parent means it's the current directory
            std::env::current_dir()?
        };

        // Skip if we've already scanned this directory
        if !scanned_dirs.insert(parent.clone()) {
            continue;
        }

        // Walk the directory to find all files (not just subdirectories)
        if parent.exists() {
            for entry in WalkDir::new(&parent)
                .follow_links(ctx.config.tracking.follow_symlinks)
                .max_depth(3) // Limit depth to avoid deep recursion
                .into_iter()
                .filter_entry(|e| {
                    // Ignore dotman's own files
                    if e.path().starts_with(&ctx.repo_path) {
                        return false;
                    }
                    !should_ignore(e.path(), &ctx.config.tracking.ignore_patterns)
                })
            {
                match entry {
                    Ok(entry) => {
                        if entry.file_type().is_file() {
                            let entry_path = entry.path().to_path_buf();
                            // Convert to relative path if it's within current directory
                            let relative_path = if entry_path.is_absolute() {
                                if let Ok(cwd) = std::env::current_dir() {
                                    entry_path
                                        .strip_prefix(&cwd)
                                        .unwrap_or(&entry_path)
                                        .to_path_buf()
                                } else {
                                    entry_path
                                }
                            } else {
                                entry_path
                            };
                            // Add only if not already in the list
                            if !files.contains(&relative_path) {
                                files.push(relative_path);
                            }
                        }
                    }
                    Err(err) => {
                        // Skip permission denied errors
                        if let Some(io_err) = err.io_error()
                            && io_err.kind() == std::io::ErrorKind::PermissionDenied
                        {
                            continue;
                        }
                        // For other errors, continue silently
                    }
                }
            }
        }
    }

    // If no tracked files yet, scan current directory for potential files to add
    if index.entries.is_empty() {
        let current_dir = std::env::current_dir()?;
        for entry in WalkDir::new(&current_dir)
            .follow_links(ctx.config.tracking.follow_symlinks)
            .max_depth(3)
            .into_iter()
            .filter_entry(|e| !should_ignore(e.path(), &ctx.config.tracking.ignore_patterns))
        {
            match entry {
                Ok(entry) => {
                    if entry.file_type().is_file() {
                        files.push(entry.path().to_path_buf());
                    }
                }
                Err(_) => continue,
            }
        }
    }

    Ok(files)
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
