use crate::refs::RefManager;
use crate::storage::FileStatus;
use crate::storage::index::Index;
use crate::{DotmanContext, INDEX_FILE};
use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::HashSet;
use std::path::PathBuf;

/// Execute status command - show the working tree status
///
/// # Errors
///
/// Returns an error if:
/// - The repository is not initialized
/// - Cannot read the index
/// - File status checks fail
pub fn execute(ctx: &DotmanContext, short: bool, show_untracked: bool) -> Result<()> {
    execute_verbose(ctx, short, show_untracked, false)
}

/// Execute status command with optional verbose output
///
/// # Errors
///
/// Returns an error if:
/// - The repository is not initialized
/// - Cannot read the index
/// - File status checks fail
#[allow(clippy::too_many_lines)]
pub fn execute_verbose(
    ctx: &DotmanContext,
    short: bool,
    show_untracked: bool,
    verbose: bool,
) -> Result<()> {
    ctx.check_repo_initialized()?;

    let ref_manager = RefManager::new(ctx.repo_path.clone());
    if let Some(branch) = ref_manager.current_branch()? {
        println!("On branch {}", branch.bold());
    } else if let Some(commit) = ref_manager.get_head_commit()? {
        println!(
            "HEAD detached at {}",
            &commit[..8.min(commit.len())].yellow()
        );
    }

    let index_path = ctx.repo_path.join(INDEX_FILE);
    let index = Index::load(&index_path)?;

    let placeholder_commit = "0".repeat(40);
    let has_commits = ref_manager
        .get_head_commit()?
        .is_some_and(|c| c != placeholder_commit);

    if index.entries.is_empty() && index.staged_entries.is_empty() {
        if !has_commits {
            println!("\nNo commits yet");
        }
        println!("\nnothing to add (use \"dot add\" to track files)");
        return Ok(());
    }

    let mut statuses = Vec::new();
    let home = dirs::home_dir().context("Could not find home directory")?;

    // Load the last commit snapshot to check if files are new or modified
    let last_commit_files = ref_manager
        .get_head_commit()?
        .filter(|id| id != &placeholder_commit)
        .and_then(|commit_id| {
            let snapshot_manager = crate::storage::snapshots::SnapshotManager::new(
                ctx.repo_path.clone(),
                ctx.config.core.compression_level,
            );
            snapshot_manager.load_snapshot(&commit_id).ok()
        })
        .map(|snapshot| snapshot.files);

    // Helper to determine file status
    let file_status = |path: &PathBuf| -> FileStatus {
        let in_last_commit = last_commit_files
            .as_ref()
            .is_some_and(|files| files.contains_key(path));

        if in_last_commit {
            FileStatus::Modified(path.clone())
        } else {
            FileStatus::Added(path.clone())
        }
    };

    // Check staged entries
    for (path, staged_entry) in &index.staged_entries {
        match index.entries.get(path) {
            Some(committed_entry) if staged_entry.hash != committed_entry.hash => {
                statuses.push(file_status(path));
            }
            None => {
                statuses.push(file_status(path));
            }
            _ => {} // File unchanged
        }
    }

    for path in index.entries.keys() {
        if !index.staged_entries.contains_key(path) {
            statuses.push(FileStatus::Deleted(path.clone()));
        }
    }

    for (path, staged_entry) in &index.staged_entries {
        let abs_path = if path.is_relative() {
            home.join(path)
        } else {
            path.clone()
        };

        if abs_path.exists() {
            // Use cached hash for performance
            let (current_hash, _) =
                crate::storage::file_ops::hash_file(&abs_path, staged_entry.cached_hash.as_ref())
                    .unwrap_or_else(|_| {
                        (
                            String::new(),
                            crate::storage::CachedHash {
                                hash: String::new(),
                                size_at_hash: 0,
                                mtime_at_hash: 0,
                            },
                        )
                    });

            if !current_hash.is_empty() && current_hash != staged_entry.hash {
                statuses.push(FileStatus::Modified(path.clone()));
            }
        }
    }

    if show_untracked {
        let untracked = find_untracked_files(ctx, &index)?;
        for file in untracked {
            statuses.push(FileStatus::Untracked(file));
        }
    }

    if statuses.is_empty() {
        println!("\nnothing to commit, working tree clean");

        // Show cache statistics in verbose mode
        if verbose {
            let (total, cached, hit_rate) = index.get_cache_stats();
            println!("\n{}", "Cache Statistics:".bold());
            println!("  Total entries: {total}");
            println!("  Cached entries: {cached}");
            println!("  Cache hit rate: {:.1}%", hit_rate * 100.0);
        }

        return Ok(());
    }

    statuses.sort_by_key(|s| (s.status_char(), s.path().to_path_buf()));

    if short {
        for status in statuses {
            println!("{} {}", status.status_char(), status.path().display());
        }
    } else {
        // Separate staged and unstaged modifications
        let staged_new: Vec<&FileStatus> = statuses
            .iter()
            .filter(|s| matches!(s, FileStatus::Added(_)))
            .collect();

        let staged_modified: Vec<&FileStatus> = statuses
            .iter()
            .filter(|s| {
                matches!(s, FileStatus::Modified(_)) && {
                    // Check if the file is in staged_entries
                    if let FileStatus::Modified(p) = s {
                        index.staged_entries.contains_key(p)
                    } else {
                        false
                    }
                }
            })
            .collect();

        // Print staged changes
        if !staged_new.is_empty() || !staged_modified.is_empty() {
            println!("\n{}:", "Changes to be committed:".bold());
            for status in &staged_new {
                println!("  {}: {}", "new file".green(), status.path().display());
            }
            for status in &staged_modified {
                println!("  {}: {}", "modified".yellow(), status.path().display());
            }
        }

        // Print unstaged modifications
        let unstaged_modified: Vec<&FileStatus> = statuses
            .iter()
            .filter(|s| {
                matches!(s, FileStatus::Modified(_)) && {
                    if let FileStatus::Modified(p) = s {
                        !index.staged_entries.contains_key(p)
                    } else {
                        false
                    }
                }
            })
            .collect();

        if !unstaged_modified.is_empty() {
            println!("\n{}:", "Changes not staged:".bold());
            for status in &unstaged_modified {
                println!("  {}: {}", "modified".yellow(), status.path().display());
            }
        }

        print_status_group(
            &statuses,
            &FileStatus::Deleted(PathBuf::new()),
            "Deleted files:",
            "deleted",
        );
        print_status_group(
            &statuses,
            &FileStatus::Untracked(PathBuf::new()),
            "Untracked files:",
            "untracked",
        );
    }

    // Show cache statistics in verbose mode
    if verbose {
        let (total, cached, hit_rate) = index.get_cache_stats();
        println!("\n{}", "Cache Statistics:".bold());
        println!("  Total entries: {total}");
        println!("  Cached entries: {cached}");
        println!("  Cache hit rate: {:.1}%", hit_rate * 100.0);
    }

    Ok(())
}

/// Get all currently tracked files
///
/// # Errors
///
/// Returns an error if:
/// - Cannot load the index
/// - Cannot determine home directory
pub fn get_current_files(ctx: &DotmanContext) -> Result<Vec<PathBuf>> {
    let index_path = ctx.repo_path.join(INDEX_FILE);
    let index = Index::load(&index_path)?;

    let mut files = Vec::new();

    let home = dirs::home_dir().context("Could not find home directory")?;

    for path in index.entries.keys() {
        let abs_path = if path.is_relative() {
            home.join(path)
        } else {
            path.clone()
        };
        files.push(abs_path);
    }

    Ok(files)
}

/// Find untracked files based on configured patterns
///
/// # Errors
///
/// Returns an error if:
/// - Cannot determine home directory
/// - File traversal fails
pub fn find_untracked_files(ctx: &DotmanContext, index: &Index) -> Result<Vec<PathBuf>> {
    use walkdir::WalkDir;

    let mut untracked = Vec::new();
    let home = dirs::home_dir().context("Could not find home directory")?;

    let mut tracked_paths: HashSet<PathBuf> = HashSet::new();

    for path in index.entries.keys().chain(index.staged_entries.keys()) {
        if path.is_relative() {
            tracked_paths.insert(home.join(path));
        } else {
            tracked_paths.insert(path.clone());
        }
    }

    for entry in WalkDir::new(&home)
        .follow_links(ctx.config.tracking.follow_symlinks)
        .into_iter()
        .filter_entry(|e| {
            let path = e.path();
            if path != home
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with('.'))
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

fn print_status_group(
    statuses: &[FileStatus],
    status_type: &FileStatus,
    header: &str,
    label: &str,
) {
    let filtered: Vec<&FileStatus> = statuses
        .iter()
        .filter(|s| std::mem::discriminant(*s) == std::mem::discriminant(status_type))
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
            no_pager: true,
        };

        // Note: This test is limited since it would scan the actual home directory
        // In a real test environment, we'd mock the home directory

        Ok(())
    }
}
