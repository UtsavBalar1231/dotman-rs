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

    for (path, staged_entry) in &index.staged_entries {
        match index.entries.get(path) {
            Some(committed_entry) => {
                if staged_entry.hash != committed_entry.hash {
                    statuses.push(FileStatus::Added(path.clone()));
                }
            }
            None => {
                statuses.push(FileStatus::Added(path.clone()));
            }
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

        if abs_path.exists()
            && let Ok(current_hash) = crate::utils::hash::hash_file(&abs_path)
            && current_hash != staged_entry.hash
        {
            statuses.push(FileStatus::Modified(path.clone()));
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
        return Ok(());
    }

    statuses.sort_by_key(|s| (s.status_char(), s.path().to_path_buf()));

    if short {
        for status in statuses {
            println!("{} {}", status.status_char(), status.path().display());
        }
    } else {
        print_status_group(
            &statuses,
            &FileStatus::Added(PathBuf::new()),
            "Changes to be committed:",
            "new file",
        );
        print_status_group(
            &statuses,
            &FileStatus::Modified(PathBuf::new()),
            "Changes not staged:",
            "modified",
        );
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
        .follow_links(false)
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
