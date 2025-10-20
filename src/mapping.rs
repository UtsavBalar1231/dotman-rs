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

/// Represents the mapping between dotman and git commits for a specific branch.
///
/// This struct tracks the current state of a branch in both dotman and git repositories,
/// allowing synchronization between the two systems. Each branch can have multiple git
/// remote heads associated with it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchMapping {
    /// Current dotman commit for this branch
    dotman_head: String,
    /// Current git commit for this branch per remote
    git_heads: HashMap<String, String>,
}

impl CommitMapping {
    /// Create a new empty mapping
    #[must_use]
    pub fn new() -> Self {
        Self {
            dotman_to_git: HashMap::new(),
            git_to_dotman: HashMap::new(),
            branch_mappings: HashMap::new(),
        }
    }

    /// Load mapping from file
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - File exists but cannot be read
    /// - File contains invalid TOML
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
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to serialize mapping to TOML
    /// - Failed to create parent directory
    /// - Failed to write file
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
    #[must_use]
    pub fn get_git_commit(&self, remote: &str, dotman_commit: &str) -> Option<String> {
        self.dotman_to_git
            .get(remote)
            .and_then(|m| m.get(dotman_commit))
            .cloned()
    }

    /// Get dotman commit ID for a git commit
    #[must_use]
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
    #[must_use]
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
    #[must_use]
    pub fn is_pushed(&self, remote: &str, dotman_commit: &str) -> bool {
        self.dotman_to_git
            .get(remote)
            .is_some_and(|m| m.contains_key(dotman_commit))
    }

    /// Get all mapped dotman commits for a remote
    #[must_use]
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

/// Helper to manage mapping file path and persistence operations.
///
/// This struct provides a convenient interface for loading, modifying, and saving
/// commit mappings to disk. It handles the file path management automatically.
pub struct MappingManager {
    /// Path to the mapping file on disk
    mapping_path: PathBuf,
    /// In-memory commit mapping data
    mapping: CommitMapping,
}

impl MappingManager {
    /// Create a new mapping manager
    ///
    /// # Errors
    ///
    /// Returns an error if the mapping file exists but cannot be loaded
    pub fn new(repo_path: &Path) -> Result<Self> {
        let mapping_path = repo_path.join("remote-mappings.json");
        let mapping = CommitMapping::load(&mapping_path)?;

        Ok(Self {
            mapping_path,
            mapping,
        })
    }

    /// Get a reference to the mapping
    #[must_use]
    pub const fn mapping(&self) -> &CommitMapping {
        &self.mapping
    }

    /// Get a mutable reference to the mapping
    pub const fn mapping_mut(&mut self) -> &mut CommitMapping {
        &mut self.mapping
    }

    /// Save the mapping to disk
    ///
    /// # Errors
    ///
    /// Returns an error if the mapping cannot be saved
    pub fn save(&self) -> Result<()> {
        self.mapping.save(&self.mapping_path)
    }

    /// Add a mapping and save
    ///
    /// # Errors
    ///
    /// Returns an error if the mapping cannot be saved to disk
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
    ///
    /// # Errors
    ///
    /// Returns an error if the mapping cannot be saved to disk
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
