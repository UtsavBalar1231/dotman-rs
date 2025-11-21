use crate::scanner::dir_trie::DirTrie;
use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Find untracked files in leaf directories (directories that directly contain tracked files)
///
/// This function performs a single-pass filesystem traversal, using a trie to determine:
/// - Which directories to traverse (Transit and Leaf directories)
/// - Which directories to collect untracked files from (only Leaf directories)
///
/// # Arguments
/// * `home` - Home directory path
/// * `repo_path` - Dotman repository path (will be excluded from traversal)
/// * `trie` - Directory trie built from tracked files
/// * `tracked_files` - Set of tracked file paths (for exclusion)
///
/// # Returns
/// Vector of untracked file paths found in leaf directories
///
/// # Errors
///
/// Returns an error if directory traversal fails
pub fn find_untracked_files<S: ::std::hash::BuildHasher>(
    home: &Path,
    repo_path: &Path,
    trie: &DirTrie,
    tracked_files: &HashSet<PathBuf, S>,
) -> Result<Vec<PathBuf>> {
    let mut untracked = Vec::new();

    WalkDir::new(home)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let path = e.path();

            // Never enter dotman repo
            if path == repo_path {
                return false;
            }

            // For directories: use trie to decide whether to traverse
            if e.file_type().is_dir() {
                return trie.should_traverse(path, home);
            }

            // For files: always return true here, we'll filter in the main loop
            true
        })
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .for_each(|entry| {
            let path = entry.path();

            // Skip tracked files
            if tracked_files.contains(path) {
                return;
            }

            // Only collect from leaf directories
            if let Some(parent) = path.parent()
                && trie.should_collect(parent, home)
            {
                untracked.push(path.to_path_buf());
            }
        });

    Ok(untracked)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::dir_trie::DirTrie;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_find_untracked_in_leaf_directory() {
        let temp = TempDir::new().unwrap();
        let home = temp.path();
        let repo = home.join(".dotman");
        fs::create_dir(&repo).unwrap();

        // Create directory structure
        let config_dir = home.join(".config");
        let nvim_dir = config_dir.join("nvim");
        fs::create_dir_all(&nvim_dir).unwrap();

        // Create files
        let tracked_file = nvim_dir.join("init.lua");
        let untracked_file = nvim_dir.join("untracked.lua");
        fs::write(&tracked_file, "tracked").unwrap();
        fs::write(&untracked_file, "untracked").unwrap();

        // Build trie
        let mut trie = DirTrie::new();
        trie.insert_tracked_file(&tracked_file, home);

        let mut tracked_files = HashSet::new();
        tracked_files.insert(tracked_file.clone());

        // Find untracked
        let untracked = find_untracked_files(home, &repo, &trie, &tracked_files).unwrap();

        assert_eq!(untracked.len(), 1);
        assert!(untracked.contains(&untracked_file));
    }

    #[test]
    fn test_skip_untracked_in_parent_directory() {
        let temp = TempDir::new().unwrap();
        let home = temp.path();
        let repo = home.join(".dotman");
        fs::create_dir(&repo).unwrap();

        // Create directory structure
        let config_dir = home.join(".config");
        let nvim_dir = config_dir.join("nvim");
        fs::create_dir_all(&nvim_dir).unwrap();

        // Create files
        let tracked_file = nvim_dir.join("init.lua");
        let untracked_in_parent = config_dir.join("untracked.txt");
        let untracked_in_leaf = nvim_dir.join("untracked.lua");
        fs::write(&tracked_file, "tracked").unwrap();
        fs::write(&untracked_in_parent, "parent").unwrap();
        fs::write(&untracked_in_leaf, "leaf").unwrap();

        // Build trie
        let mut trie = DirTrie::new();
        trie.insert_tracked_file(&tracked_file, home);

        let mut tracked_files = HashSet::new();
        tracked_files.insert(tracked_file.clone());

        // Find untracked
        let untracked = find_untracked_files(home, &repo, &trie, &tracked_files).unwrap();

        // Should only find untracked file in leaf directory, not parent
        assert_eq!(untracked.len(), 1);
        assert!(untracked.contains(&untracked_in_leaf));
        assert!(!untracked.contains(&untracked_in_parent));
    }

    #[test]
    fn test_exclude_dotman_repo() {
        let temp = TempDir::new().unwrap();
        let home = temp.path();
        let repo = home.join(".dotman");
        fs::create_dir_all(&repo).unwrap();

        // Create file in repo
        let file_in_repo = repo.join("config");
        fs::write(&file_in_repo, "config").unwrap();

        // Build empty trie
        let trie = DirTrie::new();
        let tracked_files = HashSet::new();

        // Find untracked
        let untracked = find_untracked_files(home, &repo, &trie, &tracked_files).unwrap();

        // Should not find any files in .dotman
        assert!(!untracked.contains(&file_in_repo));
    }
}
