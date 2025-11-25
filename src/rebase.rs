//! Rebase state management for persistent rebase operations
//!
//! This module provides functionality for managing rebase state across multiple
//! rebase steps, similar to Git's rebase mechanism. The state is persisted to disk
//! to allow for interruption and continuation when conflicts occur.

use crate::utils::serialization::{deserialize, serialize};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Persistent state for an ongoing rebase operation
///
/// This structure tracks all information needed to continue or abort a rebase
/// operation that may span multiple steps (e.g., when conflicts occur).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebaseState {
    /// The commit we're rebasing onto (the new base)
    pub onto: String,
    /// The original HEAD commit before rebase started
    pub original_head: String,
    /// The original branch name, if HEAD was on a branch (None if detached)
    pub original_branch: Option<String>,
    /// List of commit IDs to replay in order
    pub commits_to_replay: Vec<String>,
    /// Current index in `commits_to_replay` (0-based)
    pub current_index: usize,
    /// Files that have conflicts in the current replay step
    pub conflict_files: Vec<PathBuf>,
}

impl RebaseState {
    /// Create a new rebase state
    ///
    /// # Arguments
    ///
    /// * `onto` - The commit ID we're rebasing onto
    /// * `original_head` - The HEAD commit ID before rebase started
    /// * `original_branch` - The branch name if HEAD was on a branch
    /// * `commits_to_replay` - List of commit IDs to replay
    #[must_use]
    pub const fn new(
        onto: String,
        original_head: String,
        original_branch: Option<String>,
        commits_to_replay: Vec<String>,
    ) -> Self {
        Self {
            onto,
            original_head,
            original_branch,
            commits_to_replay,
            current_index: 0,
            conflict_files: Vec::new(),
        }
    }

    /// Save the rebase state to disk
    ///
    /// # Arguments
    ///
    /// * `repo_path` - Path to the repository root
    ///
    /// # Errors
    ///
    /// Returns an error if the state file cannot be written
    pub fn save(&self, repo_path: &Path) -> Result<()> {
        let state_path = repo_path.join("REBASE_STATE");
        let serialized = serialize(self).context("Failed to serialize rebase state")?;
        fs::write(&state_path, serialized)
            .with_context(|| format!("Failed to write REBASE_STATE: {}", state_path.display()))?;
        Ok(())
    }

    /// Load the rebase state from disk
    ///
    /// # Arguments
    ///
    /// * `repo_path` - Path to the repository root
    ///
    /// # Returns
    ///
    /// Returns `Some(RebaseState)` if a rebase is in progress, `None` otherwise
    ///
    /// # Errors
    ///
    /// Returns an error if the state file exists but cannot be read or deserialized
    pub fn load(repo_path: &Path) -> Result<Option<Self>> {
        let state_path = repo_path.join("REBASE_STATE");
        if !state_path.exists() {
            return Ok(None);
        }

        let bytes = fs::read(&state_path)
            .with_context(|| format!("Failed to read REBASE_STATE: {}", state_path.display()))?;
        let state: Self = deserialize(&bytes).context("Failed to deserialize rebase state")?;
        Ok(Some(state))
    }

    /// Clear the rebase state from disk
    ///
    /// # Arguments
    ///
    /// * `repo_path` - Path to the repository root
    ///
    /// # Errors
    ///
    /// Returns an error if the state file cannot be deleted
    pub fn clear(repo_path: &Path) -> Result<()> {
        let state_path = repo_path.join("REBASE_STATE");
        if state_path.exists() {
            fs::remove_file(&state_path).with_context(|| {
                format!("Failed to remove REBASE_STATE: {}", state_path.display())
            })?;
        }
        Ok(())
    }

    /// Check if a rebase is currently in progress
    ///
    /// # Arguments
    ///
    /// * `repo_path` - Path to the repository root
    ///
    /// # Returns
    ///
    /// `true` if `REBASE_STATE` file exists, `false` otherwise
    #[must_use]
    pub fn is_in_progress(repo_path: &Path) -> bool {
        repo_path.join("REBASE_STATE").exists()
    }

    /// Get the current commit ID being replayed
    ///
    /// # Returns
    ///
    /// The commit ID at `current_index`, or `None` if rebase is complete
    #[must_use]
    pub fn current_commit(&self) -> Option<&str> {
        self.commits_to_replay
            .get(self.current_index)
            .map(String::as_str)
    }

    /// Advance to the next commit in the replay sequence
    pub fn advance(&mut self) {
        self.current_index += 1;
        self.conflict_files.clear();
    }

    /// Check if the rebase is complete
    ///
    /// # Returns
    ///
    /// `true` if all commits have been replayed, `false` otherwise
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.current_index >= self.commits_to_replay.len()
    }

    /// Get the total number of commits to replay
    #[must_use]
    pub const fn total_commits(&self) -> usize {
        self.commits_to_replay.len()
    }

    /// Get the number of commits remaining (including current)
    #[must_use]
    pub fn remaining_commits(&self) -> usize {
        self.commits_to_replay
            .len()
            .saturating_sub(self.current_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_rebase_state_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let state = RebaseState::new(
            "onto123".to_string(),
            "head456".to_string(),
            Some("main".to_string()),
            vec!["commit1".to_string(), "commit2".to_string()],
        );

        // Save and load
        state.save(repo_path).unwrap();
        let loaded = RebaseState::load(repo_path).unwrap().unwrap();

        assert_eq!(loaded.onto, "onto123");
        assert_eq!(loaded.original_head, "head456");
        assert_eq!(loaded.original_branch, Some("main".to_string()));
        assert_eq!(loaded.commits_to_replay.len(), 2);
        assert_eq!(loaded.current_index, 0);
    }

    #[test]
    fn test_rebase_state_progress() {
        let mut state = RebaseState::new(
            "onto".to_string(),
            "head".to_string(),
            None,
            vec!["c1".to_string(), "c2".to_string(), "c3".to_string()],
        );

        assert_eq!(state.current_commit(), Some("c1"));
        assert!(!state.is_complete());
        assert_eq!(state.remaining_commits(), 3);

        state.advance();
        assert_eq!(state.current_commit(), Some("c2"));
        assert_eq!(state.remaining_commits(), 2);

        state.advance();
        assert_eq!(state.current_commit(), Some("c3"));
        assert_eq!(state.remaining_commits(), 1);

        state.advance();
        assert_eq!(state.current_commit(), None);
        assert!(state.is_complete());
        assert_eq!(state.remaining_commits(), 0);
    }

    #[test]
    fn test_is_in_progress() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        assert!(!RebaseState::is_in_progress(repo_path));

        let state = RebaseState::new(
            "onto".to_string(),
            "head".to_string(),
            None,
            vec!["c1".to_string()],
        );
        state.save(repo_path).unwrap();

        assert!(RebaseState::is_in_progress(repo_path));

        RebaseState::clear(repo_path).unwrap();
        assert!(!RebaseState::is_in_progress(repo_path));
    }
}
