//! Core index implementation for file tracking.
//!
//! This module provides the main index structure for tracking files in a dotman repository.
//! The index maintains three categories of files:
//!
//! - **Committed entries**: Files already saved in commits
//! - **Staged entries**: Files ready to be committed
//! - **Deleted entries**: Files marked for deletion
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

use super::{FileEntry, FileStatus};
use crate::utils::serialization;
use anyhow::{Context, Result};
use fs4::fs_std::FileExt;
use rayon::prelude::*;
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
#[allow(clippy::unsafe_derive_deserialize)]
pub struct Index {
    /// Index format version for compatibility.
    pub version: u32,

    /// Committed file entries (saved in repository).
    pub entries: HashMap<PathBuf, FileEntry>,

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
    /// Initializes an index with version 1 and empty collections
    /// for entries, staged entries, and deleted entries.
    ///
    /// # Returns
    ///
    /// A new [`Index`] instance with default values
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: 1,
            entries: HashMap::new(),
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
        let total = self.entries.len() + self.staged_entries.len();
        if total == 0 {
            return (0, 0, 0.0);
        }

        let cached = self
            .entries
            .values()
            .filter(|e| e.cached_hash.is_some())
            .count()
            + self
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

        // Clear cached_hash from all entries before saving
        for entry in index_to_save.entries.values_mut() {
            entry.cached_hash = None;
        }
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

        for (path, entry) in &self.entries {
            let mut entry_without_cache = entry.clone();
            entry_without_cache.cached_hash = None;
            final_index
                .entries
                .insert(path.clone(), entry_without_cache);
        }

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

    /// Adds a file entry to the committed entries collection.
    ///
    /// Inserts the entry into the index's committed entries map,
    /// using the entry's path as the key.
    ///
    /// # Arguments
    ///
    /// * `entry` - The file entry to add to committed entries
    pub fn add_entry(&mut self, entry: FileEntry) {
        self.entries.insert(entry.path.clone(), entry);
    }

    /// Add multiple entries in parallel for better performance
    pub fn add_entries_parallel(&mut self, entries: Vec<FileEntry>) {
        let new_entries: HashMap<PathBuf, FileEntry> = entries
            .into_par_iter()
            .map(|entry| (entry.path.clone(), entry))
            .collect();

        self.entries.extend(new_entries);
    }

    /// Removes a file entry from the committed entries collection.
    ///
    /// # Arguments
    ///
    /// * `path` - The path of the entry to remove
    ///
    /// # Returns
    ///
    /// The removed [`FileEntry`] if it existed, or [`None`] if not found
    pub fn remove_entry(&mut self, path: &Path) -> Option<FileEntry> {
        self.entries.remove(path)
    }

    /// Retrieves a reference to a committed file entry by path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path of the entry to retrieve
    ///
    /// # Returns
    ///
    /// A reference to the [`FileEntry`] if found, or [`None`] if not present
    #[must_use]
    pub fn get_entry(&self, path: &Path) -> Option<&FileEntry> {
        self.entries.get(path)
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
    /// Returns `true` if any of the following conditions are met:
    /// - There are files marked for deletion
    /// - There are staged entries with different content than committed entries
    /// - There are new staged entries not present in committed entries
    /// - There are committed entries not present in staged entries (unless marked for deletion)
    ///
    /// # Returns
    ///
    /// `true` if there are staged changes, `false` otherwise
    #[must_use]
    pub fn has_staged_changes(&self) -> bool {
        // Check for deletions
        if !self.deleted_entries.is_empty() {
            return true;
        }

        for (path, staged_entry) in &self.staged_entries {
            match self.entries.get(path) {
                Some(committed_entry) => {
                    if staged_entry.hash != committed_entry.hash {
                        return true;
                    }
                }
                None => return true,
            }
        }

        for path in self.entries.keys() {
            if !self.staged_entries.contains_key(path) && !self.deleted_entries.contains(path) {
                return true;
            }
        }

        false
    }

    /// Commits all staged changes to the index.
    ///
    /// This operation performs three actions:
    /// 1. Moves all staged entries to the committed entries collection
    /// 2. Removes all files marked for deletion from committed entries
    /// 3. Clears both the staged entries and deleted entries collections
    ///
    /// After this operation, the index will have no staged changes.
    pub fn commit_staged(&mut self) {
        // Merge staged_entries into entries (preserve existing committed files)
        for (path, entry) in &self.staged_entries {
            self.entries.insert(path.clone(), entry.clone());
        }
        // Remove deleted entries from committed state
        for path in &self.deleted_entries {
            self.entries.remove(path);
        }
        // Clear deleted entries after commit
        self.deleted_entries.clear();
        // Clear staged entries after moving them to committed state
        self.staged_entries.clear();
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

    /// Get file statuses by comparing index entries with current filesystem state
    /// Uses parallel processing for performance
    #[must_use]
    pub fn get_status_parallel(&self, _current_files: &[PathBuf]) -> Vec<FileStatus> {
        let mut statuses = Vec::new();
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));

        let index_statuses: Vec<FileStatus> = self
            .entries
            .par_iter()
            .filter_map(|(stored_path, stored_entry)| {
                let abs_path = if stored_path.is_relative() {
                    home.join(stored_path)
                } else {
                    stored_path.clone()
                };

                Self::check_file_status(&abs_path, stored_path, stored_entry)
            })
            .collect();

        statuses.extend(index_statuses);
        statuses
    }

    /// Check the status of a single file compared to its stored entry.
    ///
    /// Attempts to detect changes by hashing the file. Falls back to
    /// metadata comparison if hashing fails.
    ///
    /// # Returns
    ///
    /// - `Some(FileStatus::Deleted)` if file doesn't exist
    /// - `Some(FileStatus::Modified)` if file content changed
    /// - `None` if file is unchanged
    fn check_file_status(
        abs_path: &Path,
        stored_path: &Path,
        stored_entry: &FileEntry,
    ) -> Option<FileStatus> {
        // File doesn't exist - it's deleted
        if !abs_path.exists() {
            return Some(FileStatus::Deleted(stored_path.to_path_buf()));
        }

        // Try to hash the file to detect content changes
        match crate::storage::file_ops::hash_file(abs_path, stored_entry.cached_hash.as_ref()) {
            Ok((current_hash, _cache)) => {
                // Hash succeeded - compare hashes
                if current_hash == stored_entry.hash {
                    None
                } else {
                    Some(FileStatus::Modified(stored_path.to_path_buf()))
                }
            }
            Err(_) => {
                // Hash failed - fall back to metadata comparison
                Self::check_file_metadata(abs_path, stored_path, stored_entry)
            }
        }
    }

    /// Check file status using metadata when hashing fails.
    ///
    /// Compares modification time and file size as a fallback when
    /// file hashing is not possible.
    ///
    /// # Returns
    ///
    /// - `Some(FileStatus::Deleted)` if file doesn't exist
    /// - `Some(FileStatus::Modified)` if metadata changed
    /// - `None` if metadata matches (assume unchanged)
    fn check_file_metadata(
        abs_path: &Path,
        stored_path: &Path,
        stored_entry: &FileEntry,
    ) -> Option<FileStatus> {
        std::fs::metadata(abs_path).map_or_else(
            |_| Some(FileStatus::Deleted(stored_path.to_path_buf())),
            |metadata| {
                let mtime = Self::get_file_mtime(&metadata);

                if mtime != stored_entry.modified || metadata.len() != stored_entry.size {
                    Some(FileStatus::Modified(stored_path.to_path_buf()))
                } else {
                    // Metadata matches but we couldn't hash - assume unchanged
                    None
                }
            },
        )
    }

    /// Extract modification time from metadata.
    ///
    /// Converts the file's modification time to seconds since Unix epoch.
    /// Returns 0 if the time cannot be determined.
    ///
    /// # Arguments
    ///
    /// * `metadata` - File metadata containing modification time
    ///
    /// # Returns
    ///
    /// Modification time as seconds since Unix epoch, or 0 on error
    fn get_file_mtime(metadata: &std::fs::Metadata) -> i64 {
        metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .and_then(|d| i64::try_from(d.as_secs()).ok())
            .unwrap_or(0)
    }
}

/// Utility for computing differences between index states.
///
/// This struct provides methods for comparing two index snapshots
/// and determining which files were added, modified, or deleted.
pub struct IndexDiffer;

impl IndexDiffer {
    /// Compute the difference between two indices.
    ///
    /// Compares the committed entries of two indices and returns
    /// a list of file status changes.
    ///
    /// # Arguments
    ///
    /// * `old` - Previous index state
    /// * `new` - Current index state
    ///
    /// # Returns
    ///
    /// Vector of [`FileStatus`] indicating changes:
    /// - `FileStatus::Added` - File exists in new but not old
    /// - `FileStatus::Modified` - File hash changed between indices
    /// - `FileStatus::Deleted` - File exists in old but not new
    #[must_use]
    pub fn diff(old: &Index, new: &Index) -> Vec<FileStatus> {
        let mut statuses = Vec::new();

        for (path, new_entry) in &new.entries {
            match old.entries.get(path) {
                Some(old_entry) => {
                    if old_entry.hash != new_entry.hash {
                        statuses.push(FileStatus::Modified(path.clone()));
                    }
                }
                None => {
                    statuses.push(FileStatus::Added(path.clone()));
                }
            }
        }

        for path in old.entries.keys() {
            if !new.entries.contains_key(path) {
                statuses.push(FileStatus::Deleted(path.clone()));
            }
        }

        statuses
    }
}
