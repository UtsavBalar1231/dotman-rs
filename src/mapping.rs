use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Manages the mapping between dotman commits and git commits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitMapping {
    /// Map from dotman commit ID to git commit ID for each remote
    dotman_to_git: HashMap<String, HashMap<String, String>>,
    /// Map from git commit ID to dotman commit ID for each remote
    git_to_dotman: HashMap<String, HashMap<String, String>>,
    /// Branch associations
    branch_mappings: HashMap<String, BranchMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchMapping {
    /// Current dotman commit for this branch
    dotman_head: String,
    /// Current git commit for this branch per remote
    git_heads: HashMap<String, String>,
}

impl CommitMapping {
    /// Create a new empty mapping
    pub fn new() -> Self {
        Self {
            dotman_to_git: HashMap::new(),
            git_to_dotman: HashMap::new(),
            branch_mappings: HashMap::new(),
        }
    }

    /// Load mapping from file
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read mapping file: {}", path.display()))?;

        toml::from_str(&content)
            .with_context(|| format!("Failed to parse mapping file: {}", path.display()))
    }

    /// Save mapping to file
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self).context("Failed to serialize mapping")?;

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("Failed to create mapping directory")?;
        }

        fs::write(path, content)
            .with_context(|| format!("Failed to write mapping file: {}", path.display()))
    }

    /// Add a mapping between dotman and git commits
    pub fn add_mapping(&mut self, remote: &str, dotman_commit: &str, git_commit: &str) {
        self.dotman_to_git
            .entry(remote.to_string())
            .or_default()
            .insert(dotman_commit.to_string(), git_commit.to_string());

        self.git_to_dotman
            .entry(remote.to_string())
            .or_default()
            .insert(git_commit.to_string(), dotman_commit.to_string());
    }

    /// Get git commit ID for a dotman commit
    pub fn get_git_commit(&self, remote: &str, dotman_commit: &str) -> Option<String> {
        self.dotman_to_git
            .get(remote)
            .and_then(|m| m.get(dotman_commit))
            .cloned()
    }

    /// Get dotman commit ID for a git commit
    pub fn get_dotman_commit(&self, remote: &str, git_commit: &str) -> Option<String> {
        self.git_to_dotman
            .get(remote)
            .and_then(|m| m.get(git_commit))
            .cloned()
    }

    /// Update branch mapping
    pub fn update_branch(&mut self, branch: &str, dotman_head: &str, remote: Option<(&str, &str)>) {
        let mapping = self
            .branch_mappings
            .entry(branch.to_string())
            .or_insert_with(|| BranchMapping {
                dotman_head: dotman_head.to_string(),
                git_heads: HashMap::new(),
            });

        mapping.dotman_head = dotman_head.to_string();

        if let Some((remote_name, git_head)) = remote {
            mapping
                .git_heads
                .insert(remote_name.to_string(), git_head.to_string());
        }
    }

    /// Get branch mapping
    pub fn get_branch(&self, branch: &str) -> Option<&BranchMapping> {
        self.branch_mappings.get(branch)
    }

    /// Remove all mappings for a remote
    pub fn remove_remote(&mut self, remote: &str) {
        self.dotman_to_git.remove(remote);
        self.git_to_dotman.remove(remote);

        for branch_mapping in self.branch_mappings.values_mut() {
            branch_mapping.git_heads.remove(remote);
        }
    }

    /// Check if a dotman commit has been pushed to a remote
    pub fn is_pushed(&self, remote: &str, dotman_commit: &str) -> bool {
        self.dotman_to_git
            .get(remote)
            .is_some_and(|m| m.contains_key(dotman_commit))
    }

    /// Get all mapped dotman commits for a remote
    pub fn get_mapped_commits(&self, remote: &str) -> Vec<String> {
        self.dotman_to_git
            .get(remote)
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }
}

impl Default for CommitMapping {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to manage mapping file path
pub struct MappingManager {
    mapping_path: PathBuf,
    mapping: CommitMapping,
}

impl MappingManager {
    /// Create a new mapping manager
    pub fn new(repo_path: &Path) -> Result<Self> {
        let mapping_path = repo_path.join("remote-mappings.json");
        let mapping = CommitMapping::load(&mapping_path)?;

        Ok(Self {
            mapping_path,
            mapping,
        })
    }

    /// Get a reference to the mapping
    pub fn mapping(&self) -> &CommitMapping {
        &self.mapping
    }

    /// Get a mutable reference to the mapping
    pub fn mapping_mut(&mut self) -> &mut CommitMapping {
        &mut self.mapping
    }

    /// Save the mapping to disk
    pub fn save(&self) -> Result<()> {
        self.mapping.save(&self.mapping_path)
    }

    /// Add a mapping and save
    pub fn add_and_save(
        &mut self,
        remote: &str,
        dotman_commit: &str,
        git_commit: &str,
    ) -> Result<()> {
        self.mapping.add_mapping(remote, dotman_commit, git_commit);
        self.save()
    }

    /// Update branch and save
    pub fn update_branch_and_save(
        &mut self,
        branch: &str,
        dotman_head: &str,
        remote: Option<(&str, &str)>,
    ) -> Result<()> {
        self.mapping.update_branch(branch, dotman_head, remote);
        self.save()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_commit_mapping() {
        let mut mapping = CommitMapping::new();

        mapping.add_mapping("origin", "dotman123", "git456");

        // Test retrieval
        assert_eq!(
            mapping.get_git_commit("origin", "dotman123"),
            Some("git456".to_string())
        );
        assert_eq!(
            mapping.get_dotman_commit("origin", "git456"),
            Some("dotman123".to_string())
        );

        // Test non-existent
        assert_eq!(mapping.get_git_commit("origin", "unknown"), None);
        assert_eq!(mapping.get_git_commit("other", "dotman123"), None);
    }

    #[test]
    fn test_branch_mapping() {
        let mut mapping = CommitMapping::new();

        // Update branch
        mapping.update_branch("main", "dotman789", Some(("origin", "git012")));

        // Test retrieval
        let branch = mapping.get_branch("main").unwrap();
        assert_eq!(branch.dotman_head, "dotman789");
        assert_eq!(branch.git_heads.get("origin"), Some(&"git012".to_string()));
    }

    #[test]
    fn test_save_load() -> Result<()> {
        let temp = tempdir()?;
        let mapping_file = temp.path().join("mapping.json");

        // Create and save mapping
        let mut mapping = CommitMapping::new();
        mapping.add_mapping("origin", "d1", "g1");
        mapping.add_mapping("origin", "d2", "g2");
        mapping.add_mapping("upstream", "d1", "g3");
        mapping.update_branch("main", "d2", Some(("origin", "g2")));

        mapping.save(&mapping_file)?;

        let loaded = CommitMapping::load(&mapping_file)?;
        assert_eq!(
            loaded.get_git_commit("origin", "d1"),
            Some("g1".to_string())
        );
        assert_eq!(
            loaded.get_git_commit("origin", "d2"),
            Some("g2".to_string())
        );
        assert_eq!(
            loaded.get_git_commit("upstream", "d1"),
            Some("g3".to_string())
        );

        let branch = loaded.get_branch("main").unwrap();
        assert_eq!(branch.dotman_head, "d2");

        Ok(())
    }

    #[test]
    fn test_remove_remote() {
        let mut mapping = CommitMapping::new();

        // Add mappings for multiple remotes
        mapping.add_mapping("origin", "d1", "g1");
        mapping.add_mapping("upstream", "d1", "g2");
        mapping.update_branch("main", "d1", Some(("origin", "g1")));
        mapping.update_branch("main", "d1", Some(("upstream", "g2")));

        // Remove origin
        mapping.remove_remote("origin");

        // Verify origin is gone but upstream remains
        assert_eq!(mapping.get_git_commit("origin", "d1"), None);
        assert_eq!(
            mapping.get_git_commit("upstream", "d1"),
            Some("g2".to_string())
        );

        let branch = mapping.get_branch("main").unwrap();
        assert_eq!(branch.git_heads.get("origin"), None);
        assert_eq!(branch.git_heads.get("upstream"), Some(&"g2".to_string()));
    }

    #[test]
    fn test_is_pushed() {
        let mut mapping = CommitMapping::new();

        mapping.add_mapping("origin", "d1", "g1");

        assert!(mapping.is_pushed("origin", "d1"));
        assert!(!mapping.is_pushed("origin", "d2"));
        assert!(!mapping.is_pushed("upstream", "d1"));
    }
}
