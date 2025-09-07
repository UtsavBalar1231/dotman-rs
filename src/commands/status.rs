use crate::refs::RefManager;
use crate::storage::FileStatus;
use crate::storage::index::Index;
use crate::{DotmanContext, INDEX_FILE};
use anyhow::Result;
use colored::Colorize;
use std::collections::HashSet;
use std::path::PathBuf;

pub fn execute(ctx: &DotmanContext, short: bool, show_untracked: bool) -> Result<()> {
    ctx.check_repo_initialized()?;

    // Display current branch information
    let ref_manager = RefManager::new(ctx.repo_path.clone());
    if let Some(branch) = ref_manager.current_branch()? {
        println!("On branch {}", branch.bold());
    } else {
        // Detached HEAD state
        if let Some(commit) = ref_manager.get_head_commit()? {
            println!(
                "HEAD detached at {}",
                &commit[..8.min(commit.len())].yellow()
            );
        }
    }

    let index_path = ctx.repo_path.join(INDEX_FILE);
    let index = Index::load(&index_path)?;

    // Check if repository has any commits (placeholder is 40 zeros)
    let placeholder_commit = "0".repeat(40);
    let has_commits = ref_manager
        .get_head_commit()?
        .is_some_and(|c| c != placeholder_commit);

    // Check if index is empty (no tracked files and no staged files)
    if index.entries.is_empty() && index.staged_entries.is_empty() {
        if !has_commits {
            println!("\nNo commits yet");
        }
        println!("\nnothing to add (use \"dot add\" to track files)");
        return Ok(());
    }

    // Collect all file statuses
    let mut statuses = Vec::new();
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;

    // Check staged vs committed (changes to be committed)
    for (path, staged_entry) in &index.staged_entries {
        match index.entries.get(path) {
            Some(committed_entry) => {
                if staged_entry.hash != committed_entry.hash {
                    statuses.push(FileStatus::Added(path.clone())); // Staged modification
                }
            }
            None => {
                statuses.push(FileStatus::Added(path.clone())); // New file staged
            }
        }
    }

    // Check for deleted files (in committed but not in staged)
    for path in index.entries.keys() {
        if !index.staged_entries.contains_key(path) {
            statuses.push(FileStatus::Deleted(path.clone())); // Staged deletion
        }
    }

    // Check working directory vs staged (changes not staged)
    for (path, staged_entry) in &index.staged_entries {
        let abs_path = if path.is_relative() {
            home.join(path)
        } else {
            path.clone()
        };

        if abs_path.exists() {
            // Check if file has been modified since staging
            if let Ok(current_hash) = crate::utils::hash::hash_file(&abs_path)
                && current_hash != staged_entry.hash
            {
                statuses.push(FileStatus::Modified(path.clone()));
            }
        }
    }

    // If --untracked flag is set, scan for untracked files
    if show_untracked {
        let untracked = find_untracked_files(ctx, &index)?;
        for file in untracked {
            statuses.push(FileStatus::Untracked(file));
        }
    }

    if statuses.is_empty() {
        // Only show clean working directory message if we have tracked files
        // (we already handled the empty index case above)
        println!("\nnothing to commit, working tree clean");
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

pub fn find_untracked_files(ctx: &DotmanContext, index: &Index) -> Result<Vec<PathBuf>> {
    use walkdir::WalkDir;

    let mut untracked = Vec::new();
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;

    // Get set of tracked and staged paths for quick lookup
    let mut tracked_paths: HashSet<PathBuf> = HashSet::new();

    // Include both committed and staged files as tracked
    for path in index.entries.keys().chain(index.staged_entries.keys()) {
        if path.is_relative() {
            tracked_paths.insert(home.join(path));
        } else {
            tracked_paths.insert(path.clone());
        }
    }

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
            no_pager: true,
        };

        // Note: This test is limited since it would scan the actual home directory
        // In a real test environment, we'd mock the home directory

        Ok(())
    }
}
