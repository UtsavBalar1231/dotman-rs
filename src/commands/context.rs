use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::DotmanContext;
use crate::refs::resolver::RefResolver;
use crate::storage::concurrent_index::ConcurrentIndex;
use crate::storage::index::Index;
use crate::storage::snapshots::SnapshotManager;

/// Trait providing common operations for command modules
pub trait CommandContext {
    /// Ensures the repository is initialized before executing a command
    ///
    /// # Errors
    ///
    /// Returns an error if the repository is not initialized
    fn ensure_initialized(&self) -> Result<()>;

    /// Loads the repository index
    ///
    /// # Errors
    ///
    /// Returns an error if the index file cannot be loaded
    fn load_index(&self) -> Result<Index>;

    /// Loads the repository index as a concurrent index for thread-safe operations
    ///
    /// # Errors
    ///
    /// Returns an error if the index file cannot be loaded
    fn load_concurrent_index(&self) -> Result<ConcurrentIndex>;

    /// Gets the home directory path
    ///
    /// # Errors
    ///
    /// Returns an error if the home directory cannot be determined
    fn get_home_dir(&self) -> Result<PathBuf>;

    /// Creates a `SnapshotManager` with the current configuration
    fn create_snapshot_manager(&self) -> SnapshotManager;

    /// Creates a `RefResolver` for reference resolution
    fn create_ref_resolver(&self) -> RefResolver;

    /// Returns a display-friendly version of a commit ID (first 8 chars)
    fn display_commit_id<'a>(&self, commit_id: &'a str) -> &'a str;
}

impl CommandContext for DotmanContext {
    fn ensure_initialized(&self) -> Result<()> {
        self.check_repo_initialized()
    }

    fn load_index(&self) -> Result<Index> {
        let index_path = self.repo_path.join("index.bin");
        Index::load(&index_path)
    }

    fn load_concurrent_index(&self) -> Result<ConcurrentIndex> {
        let index_path = self.repo_path.join("index.bin");
        ConcurrentIndex::load(&index_path)
    }

    fn get_home_dir(&self) -> Result<PathBuf> {
        dirs::home_dir().context("Could not find home directory")
    }

    fn create_snapshot_manager(&self) -> SnapshotManager {
        SnapshotManager::with_permissions(
            self.repo_path.clone(),
            self.config.core.compression_level,
            self.config.tracking.preserve_permissions,
        )
    }

    fn create_ref_resolver(&self) -> RefResolver {
        RefResolver::new(self.repo_path.clone())
    }

    fn display_commit_id<'a>(&self, commit_id: &'a str) -> &'a str {
        if commit_id.len() >= 8 {
            &commit_id[..8]
        } else {
            commit_id
        }
    }
}
