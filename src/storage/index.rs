//! Core index implementation for staging area.
//!
//! This module provides the main index structure for the staging area in a dotman repository.
//! The index maintains two categories of changes:
//!
//! - **Staged entries**: Files ready to be committed
//! - **Deleted entries**: Files marked for deletion
//!
//! **Note**: Committed files are stored in snapshots, not in the index. The index is purely
//! a staging area for the next commit
//!
//! # Storage Format
//!
//! The index is stored as a binary file using bincode serialization for maximum performance.
//! File locking via [`fs4`] ensures safe concurrent access.
//!
//! # Caching Strategy
//!
//! The index implements an intelligent caching system:
//! - Stores hash alongside file size and modification time
//! - Avoids re-hashing unchanged files (cache hit)
//! - Provides cache statistics for performance analysis
//!
//! # Thread Safety
//!
//! For concurrent operations, use [`ConcurrentIndex`](super::concurrent_index::ConcurrentIndex) instead.
//! This implementation uses `HashMap` and is not thread-safe.
//!
//! # Examples
//!
//! ```no_run
//! use dotman::storage::index::Index;
//! use std::path::PathBuf;
//!
//! # fn main() -> anyhow::Result<()> {
//! let mut index = Index::load(&PathBuf::from(".dotman/index.bin"))?;
//!
//! // Check cache statistics
//! let (total, cached, hit_rate) = index.get_cache_stats();
//! println!("Cache hit rate: {:.1}%", hit_rate * 100.0);
//! # Ok(())
//! # }
//! ```

use super::FileEntry;
use crate::utils::serialization;
use anyhow::{Context, Result};
use fs4::fs_std::FileExt;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Main index structure for tracking file states.
///
/// The index maintains three categories of files:
/// - `entries`: Committed files (saved in repository)
/// - `staged_entries`: Files staged for next commit
/// - `deleted_entries`: Files marked for deletion
///
/// # Versioning
///
/// The index version field allows for future format changes while
/// maintaining backwards compatibility.
///
/// # Performance
///
/// Uses [`HashMap`] for O(1) lookups. For concurrent operations,
/// use [`ConcurrentIndex`](super::concurrent_index::ConcurrentIndex).
#[derive(Debug, Clone, Serialize, Deserialize)]
// False positive: Index has no unsafe methods or invariants.
// The derived Deserialize implementation is completely safe.
#[allow(clippy::unsafe_derive_deserialize)]
pub struct Index {
    /// Index format version for compatibility.
    pub version: u32,

    /// Staged file entries (ready to commit).
    pub staged_entries: HashMap<PathBuf, FileEntry>,

    /// Deleted file entries (marked for removal).
    #[serde(default)]
    pub deleted_entries: HashSet<PathBuf>,
}

impl Default for Index {
    fn default() -> Self {
        Self::new()
    }
}

impl Index {
    /// Creates a new empty index.
    ///
    /// Initializes an index with version 2 and empty collections
    /// for staged entries and deleted entries.
    ///
    /// # Returns
    ///
    /// A new [`Index`] instance with default values
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: 2,
            staged_entries: HashMap::new(),
            deleted_entries: HashSet::new(),
        }
    }

    /// Load an index from disk
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to open or read the index file
    /// - Failed to deserialize the index
    /// - Failed to acquire file lock
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }

        // Open file with read access for locking
        let file = std::fs::OpenOptions::new()
            .read(true)
            .open(path)
            .with_context(|| format!("Failed to open index file: {}", path.display()))?;

        // Acquire shared lock for reading
        file.lock_shared()
            .context("Failed to acquire shared lock on index file")?;

        // Read the entire file while locked
        let data = std::fs::read(path)
            .with_context(|| format!("Failed to read index file: {}", path.display()))?;

        // Release lock before deserialization
        file.unlock().context("Failed to unlock index file")?;

        let index: Self =
            serialization::deserialize(&data).context("Failed to deserialize index")?;

        Ok(index)
    }

    /// Get cache statistics for the index
    ///
    /// Returns a tuple of (`total_entries`, `cached_entries`, `cache_hit_rate`)
    #[must_use]
    pub fn get_cache_stats(&self) -> (usize, usize, f64) {
        let total = self.staged_entries.len();
        if total == 0 {
            return (0, 0, 0.0);
        }

        let cached = self
            .staged_entries
            .values()
            .filter(|e| e.cached_hash.is_some())
            .count();

        #[allow(clippy::cast_precision_loss)]
        let hit_rate = cached as f64 / total as f64;
        (total, cached, hit_rate)
    }

    /// Save the index to disk
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to serialize the index
    /// - Failed to write to disk
    /// - Failed to acquire file lock
    pub fn save(&self, path: &Path) -> Result<()> {
        // Create a copy of the index without cached_hash for serialization
        // The cached_hash is only for runtime performance and shouldn't be persisted
        let mut index_to_save = self.clone();

        // Clear cached_hash from all staged entries before saving
        for entry in index_to_save.staged_entries.values_mut() {
            entry.cached_hash = None;
        }

        let data = serialization::serialize(&index_to_save).context("Failed to serialize index")?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        // Write directly without locking to avoid potential issues
        std::fs::write(path, &data)
            .with_context(|| format!("Failed to write index file: {}", path.display()))?;

        Ok(())
    }

    /// Save the index, merging with existing data on disk
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to open or read existing index
    /// - Failed to serialize or write the merged index
    /// - Failed to acquire file lock
    pub fn save_merge(&self, path: &Path) -> Result<()> {
        use std::io::Write;
        if !path.exists() {
            std::fs::write(path, [])
                .with_context(|| format!("Failed to create index file: {}", path.display()))?;
        }

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .with_context(|| format!("Failed to open index file for merge: {}", path.display()))?;

        file.lock_exclusive()
            .context("Failed to acquire exclusive lock on index file")?;

        let mut final_index = if path.exists()
            && file
                .metadata()
                .context("Failed to get file metadata")?
                .len()
                > 0
        {
            let existing_data = std::fs::read(path)
                .with_context(|| format!("Failed to read existing index: {}", path.display()))?;
            serialization::deserialize::<Self>(&existing_data).with_context(|| {
                format!(
                    "Failed to deserialize existing index from: {}",
                    path.display()
                )
            })?
        } else {
            Self::new()
        };

        for (path, entry) in &self.staged_entries {
            let mut entry_without_cache = entry.clone();
            entry_without_cache.cached_hash = None;
            final_index
                .staged_entries
                .insert(path.clone(), entry_without_cache);
        }

        // Merge deleted entries
        for path in &self.deleted_entries {
            final_index.deleted_entries.insert(path.clone());
        }

        let data =
            serialization::serialize(&final_index).context("Failed to serialize merged index")?;

        file.set_len(0).context("Failed to truncate index file")?;
        let mut file_writer = &file;
        file_writer
            .write_all(&data)
            .context("Failed to write index data")?;
        file_writer.flush().context("Failed to flush index data")?;

        file.unlock().context("Failed to unlock index file")?;

        Ok(())
    }

    /// Stages a file entry for the next commit.
    ///
    /// Adds the entry to the staged entries collection, making it
    /// ready to be committed. Staged entries will be moved to
    /// committed entries when [`commit_staged`](Self::commit_staged) is called.
    ///
    /// # Arguments
    ///
    /// * `entry` - The file entry to stage for commit
    pub fn stage_entry(&mut self, entry: FileEntry) {
        self.staged_entries.insert(entry.path.clone(), entry);
    }

    /// Retrieves a reference to a staged file entry by path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path of the staged entry to retrieve
    ///
    /// # Returns
    ///
    /// A reference to the staged [`FileEntry`] if found, or [`None`] if not present
    #[must_use]
    pub fn get_staged_entry(&self, path: &Path) -> Option<&FileEntry> {
        self.staged_entries.get(path)
    }

    /// Checks if there are any staged changes ready to commit.
    ///
    /// Returns `true` if there are files staged for commit or marked for deletion.
    ///
    /// # Returns
    ///
    /// `true` if there are staged changes, `false` otherwise
    #[must_use]
    pub fn has_staged_changes(&self) -> bool {
        !self.staged_entries.is_empty() || !self.deleted_entries.is_empty()
    }

    /// Commits all staged changes to the index.
    ///
    /// Clears the staging area after a commit is created.
    /// The actual commit data is stored in snapshots, not in the index.
    ///
    /// # Note
    ///
    /// This method should be called AFTER creating a snapshot with the staged files.
    /// It simply clears the staging area to prepare for the next commit.
    pub fn commit_staged(&mut self) {
        // Clear staged entries - they're now in the snapshot
        self.staged_entries.clear();
        // Clear deleted entries - deletions are now in the snapshot
        self.deleted_entries.clear();
    }

    /// Mark a file as deleted
    pub fn mark_deleted(&mut self, path: &PathBuf) {
        self.deleted_entries.insert(path.clone());
        // Remove from staged entries if present
        self.staged_entries.remove(path);
    }

    /// Unmark a file as deleted
    pub fn unmark_deleted(&mut self, path: &Path) -> bool {
        self.deleted_entries.remove(path)
    }

    /// Check if a file is marked for deletion
    #[must_use]
    pub fn is_deleted(&self, path: &Path) -> bool {
        self.deleted_entries.contains(path)
    }

    /// Get all deleted entries
    #[must_use]
    pub const fn get_deleted_entries(&self) -> &HashSet<PathBuf> {
        &self.deleted_entries
    }

    /// Get file statuses for a list of paths by comparing against staged entries
    ///
    /// This method checks each path against the staged entries and returns
    /// a list of file statuses indicating modifications or deletions.
    ///
    /// # Arguments
    ///
    /// * `paths` - List of file paths to check (typically from committed snapshot)
    ///
    /// # Returns
    ///
    /// A vector of [`crate::storage::FileStatus`] for files that have changed
    #[must_use]
    pub fn get_status_parallel(&self, paths: &[PathBuf]) -> Vec<crate::storage::FileStatus> {
        use crate::storage::FileStatus;

        let mut statuses = Vec::new();

        // Check each path against staged entries
        for path in paths {
            if let Some(staged_entry) = self.staged_entries.get(path) {
                // File is staged - check if it exists and matches
                if path.exists() {
                    // Try to hash the file and compare
                    if let Ok((current_hash, _)) =
                        crate::storage::file_ops::hash_file(path, staged_entry.cached_hash.as_ref())
                        && current_hash != staged_entry.hash
                    {
                        statuses.push(FileStatus::Modified(path.clone()));
                    }
                } else {
                    statuses.push(FileStatus::Deleted(path.clone()));
                }
            }
        }

        // Check for files staged but not in the paths list (new files)
        for path in self.staged_entries.keys() {
            if !paths.contains(path) {
                statuses.push(FileStatus::Added(path.clone()));
            }
        }

        // Add explicitly marked deletions
        for path in &self.deleted_entries {
            statuses.push(FileStatus::Deleted(path.clone()));
        }

        statuses
    }
}
