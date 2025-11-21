use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Component, Path};

/// Role of a directory in the tracked file hierarchy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectoryRole {
    /// Directory directly contains tracked files - collect untracked files here
    Leaf,
    /// Ancestor directory - traverse through but don't collect
    Transit,
    /// Not in tracked hierarchy - skip entirely
    Untracked,
}

/// Prefix tree (trie) representing the tracked directory hierarchy
pub struct DirTrie {
    /// Child nodes keyed by directory name component
    children: HashMap<OsString, Self>,
    /// Role of this directory in the hierarchy
    role: DirectoryRole,
}

impl DirTrie {
    /// Create a new empty trie with Transit role (for root)
    #[must_use]
    pub fn new() -> Self {
        Self {
            children: HashMap::new(),
            role: DirectoryRole::Transit,
        }
    }

    /// Insert a tracked file into the trie, marking its parent as Leaf
    /// and all ancestors as Transit
    pub fn insert_tracked_file(&mut self, file_path: &Path, home: &Path) {
        let Some(parent) = file_path.parent() else {
            return;
        };

        let Ok(rel_path) = parent.strip_prefix(home) else {
            return;
        };

        let mut current = self;

        for component in rel_path.components() {
            let Component::Normal(name) = component else {
                continue;
            };

            current = current
                .children
                .entry(name.to_os_string())
                .or_insert_with(|| Self {
                    children: HashMap::new(),
                    role: DirectoryRole::Transit,
                });
        }

        // Mark the final node as Leaf (directly contains tracked file)
        current.role = DirectoryRole::Leaf;
    }

    /// Look up a directory's role in the tracked hierarchy
    #[must_use]
    pub fn get_role(&self, dir_path: &Path, home: &Path) -> DirectoryRole {
        // Handle home directory specially - always Transit
        if dir_path == home {
            return DirectoryRole::Transit;
        }

        // Convert to relative path
        let Ok(rel_path) = dir_path.strip_prefix(home) else {
            return DirectoryRole::Untracked;
        };

        // Walk down the trie following the path
        let mut current = self;

        for component in rel_path.components() {
            let Component::Normal(name) = component else {
                return DirectoryRole::Untracked;
            };

            match current.children.get(name) {
                Some(child) => current = child,
                None => return DirectoryRole::Untracked,
            }
        }

        current.role
    }

    /// Returns true for Transit and Leaf directories (directories in tracked hierarchy)
    #[inline]
    #[must_use]
    pub fn should_traverse(&self, dir_path: &Path, home: &Path) -> bool {
        matches!(
            self.get_role(dir_path, home),
            DirectoryRole::Transit | DirectoryRole::Leaf
        )
    }

    /// Returns true for Leaf directories (directories directly containing tracked files)
    #[inline]
    #[must_use]
    pub fn should_collect(&self, dir_path: &Path, home: &Path) -> bool {
        self.get_role(dir_path, home) == DirectoryRole::Leaf
    }
}

impl Default for DirTrie {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_empty_trie() {
        let trie = DirTrie::new();
        let home = PathBuf::from("/home/user");
        let test_dir = home.join(".config");

        assert_eq!(trie.get_role(&test_dir, &home), DirectoryRole::Untracked);
    }

    #[test]
    fn test_leaf_directory() {
        let mut trie = DirTrie::new();
        let home = PathBuf::from("/home/user");
        let file = home.join(".config/nvim/init.lua");

        trie.insert_tracked_file(&file, &home);

        let nvim_dir = home.join(".config/nvim");
        assert_eq!(trie.get_role(&nvim_dir, &home), DirectoryRole::Leaf);
    }

    #[test]
    fn test_transit_directory() {
        let mut trie = DirTrie::new();
        let home = PathBuf::from("/home/user");
        let file = home.join(".config/nvim/init.lua");

        trie.insert_tracked_file(&file, &home);

        let config_dir = home.join(".config");
        assert_eq!(trie.get_role(&config_dir, &home), DirectoryRole::Transit);
    }

    #[test]
    fn test_home_always_transit() {
        let trie = DirTrie::new();
        let home = PathBuf::from("/home/user");

        assert_eq!(trie.get_role(&home, &home), DirectoryRole::Transit);
    }

    #[test]
    fn test_untracked_directory() {
        let mut trie = DirTrie::new();
        let home = PathBuf::from("/home/user");
        let file = home.join(".config/nvim/init.lua");

        trie.insert_tracked_file(&file, &home);

        let other_dir = home.join(".local/share");
        assert_eq!(trie.get_role(&other_dir, &home), DirectoryRole::Untracked);
    }

    #[test]
    fn test_deep_nesting() {
        let mut trie = DirTrie::new();
        let home = PathBuf::from("/home/user");
        let file = home.join(".local/share/nvim/site/pack/plugins/start/telescope/init.lua");

        trie.insert_tracked_file(&file, &home);

        let telescope_dir = home.join(".local/share/nvim/site/pack/plugins/start/telescope");
        assert_eq!(trie.get_role(&telescope_dir, &home), DirectoryRole::Leaf);

        let start_dir = home.join(".local/share/nvim/site/pack/plugins/start");
        assert_eq!(trie.get_role(&start_dir, &home), DirectoryRole::Transit);
    }

    #[test]
    fn test_should_traverse() {
        let mut trie = DirTrie::new();
        let home = PathBuf::from("/home/user");
        let file = home.join(".config/nvim/init.lua");

        trie.insert_tracked_file(&file, &home);

        assert!(trie.should_traverse(&home.join(".config"), &home));
        assert!(trie.should_traverse(&home.join(".config/nvim"), &home));
        assert!(!trie.should_traverse(&home.join(".local"), &home));
    }

    #[test]
    fn test_should_collect() {
        let mut trie = DirTrie::new();
        let home = PathBuf::from("/home/user");
        let file = home.join(".config/nvim/init.lua");

        trie.insert_tracked_file(&file, &home);

        assert!(!trie.should_collect(&home.join(".config"), &home));
        assert!(trie.should_collect(&home.join(".config/nvim"), &home));
        assert!(!trie.should_collect(&home.join(".local"), &home));
    }
}
