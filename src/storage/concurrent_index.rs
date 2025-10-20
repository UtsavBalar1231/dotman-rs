//! Thread-safe concurrent index operations.
//!
//! This module provides a lock-free concurrent wrapper around the standard [`Index`]
//! using [`DashMap`] for parallel file operations. It's designed for scenarios where
//! multiple threads need to modify the index simultaneously.
//!
//! # Design
//!
//! The [`ConcurrentIndex`] uses three concurrent data structures:
//! - `entries`: Committed files (in repository)
//! - `staged_entries`: Files staged for next commit
//! - `deleted_entries`: Files marked for deletion
//!
//! All operations are lock-free and thread-safe, making it ideal for parallel
//! file processing with [`rayon`].
//!
//! # Performance
//!
//! - Lock-free reads and writes using [`DashMap`]
//! - Automatic memory-mapped I/O for large files
//! - Cached hash optimization for unchanged files
//!
//! # Examples
//!
//! ```no_run
//! use dotman::storage::concurrent_index::ConcurrentIndex;
//! use std::path::PathBuf;
//!
//! # fn main() -> anyhow::Result<()> {
//! let index = ConcurrentIndex::load(&PathBuf::from(".dotman/index.bin"))?;
//!
//! // Thread-safe staging
//! // Multiple threads can call this simultaneously
//! # Ok(())
//! # }
//! ```

use super::FileEntry;
use crate::storage::index::Index;
use anyhow::Result;
use dashmap::DashMap;
use dashmap::DashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Thread-safe concurrent index using `DashMap` for lock-free operations
#[derive(Debug, Clone)]
pub struct ConcurrentIndex {
    /// Committed files currently tracked in the repository
    entries: Arc<DashMap<PathBuf, FileEntry>>,
    /// Files staged for the next commit
    staged_entries: Arc<DashMap<PathBuf, FileEntry>>,
    /// Files marked for deletion in the next commit
    deleted_entries: Arc<DashSet<PathBuf>>,
}

impl Default for ConcurrentIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl ConcurrentIndex {
    /// Create a new empty concurrent index
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            staged_entries: Arc::new(DashMap::new()),
            deleted_entries: Arc::new(DashSet::new()),
        }
    }

    /// Load a concurrent index from disk
    ///
    /// # Errors
    ///
    /// Returns an error if failed to load the index from disk
    pub fn load(path: &Path) -> Result<Self> {
        let index = Index::load(path)?;
        Ok(Self::from_index(index))
    }

    /// Create a concurrent index from a regular index
    #[must_use]
    pub fn from_index(index: Index) -> Self {
        let concurrent = Self::new();

        for (path, entry) in index.entries {
            concurrent.entries.insert(path, entry);
        }

        for (path, entry) in index.staged_entries {
            concurrent.staged_entries.insert(path, entry);
        }

        for path in index.deleted_entries {
            concurrent.deleted_entries.insert(path);
        }

        concurrent
    }

    /// Convert to a regular index for serialization
    #[must_use]
    pub fn to_index(&self) -> Index {
        let mut index = Index::new();

        for entry in self.entries.iter() {
            index
                .entries
                .insert(entry.key().clone(), entry.value().clone());
        }

        for entry in self.staged_entries.iter() {
            index
                .staged_entries
                .insert(entry.key().clone(), entry.value().clone());
        }

        for entry in self.deleted_entries.iter() {
            index.deleted_entries.insert(entry.key().clone());
        }

        index
    }

    /// Save the concurrent index to disk
    ///
    /// # Errors
    ///
    /// Returns an error if failed to save the index to disk
    pub fn save(&self, path: &Path) -> Result<()> {
        let index = self.to_index();
        index.save(path)
    }

    /// Save and merge the concurrent index with existing disk index
    ///
    /// # Errors
    ///
    /// Returns an error if failed to save or merge the index
    pub fn save_merge(&self, path: &Path) -> Result<()> {
        let index = self.to_index();
        index.save_merge(path)
    }

    /// Stage a file entry
    pub fn stage_entry(&self, entry: FileEntry) {
        let path = entry.path.clone();
        self.staged_entries.insert(path, entry);
    }

    /// Get a staged entry
    #[must_use]
    pub fn get_staged_entry(&self, path: &Path) -> Option<FileEntry> {
        self.staged_entries.get(path).map(|e| e.clone())
    }

    /// Get a committed entry
    #[must_use]
    pub fn get_entry(&self, path: &Path) -> Option<FileEntry> {
        self.entries.get(path).map(|e| e.clone())
    }

    /// Remove a staged entry
    #[must_use]
    pub fn remove_staged(&self, path: &Path) -> Option<FileEntry> {
        self.staged_entries.remove(path).map(|(_, v)| v)
    }

    /// Remove a committed entry
    #[must_use]
    pub fn remove_entry(&self, path: &Path) -> Option<FileEntry> {
        self.entries.remove(path).map(|(_, v)| v)
    }

    /// Check if there are any staged changes
    #[must_use]
    pub fn has_staged_changes(&self) -> bool {
        !self.staged_entries.is_empty()
    }

    /// Get all staged entries
    #[must_use]
    pub fn staged_entries(&self) -> Vec<(PathBuf, FileEntry)> {
        self.staged_entries
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    /// Get all committed entries
    #[must_use]
    pub fn entries(&self) -> Vec<(PathBuf, FileEntry)> {
        self.entries
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    /// Clear all staged entries
    pub fn clear_staged(&self) {
        self.staged_entries.clear();
    }

    /// Commit staged entries to the main index
    pub fn commit_staged(&self) {
        // Merge staged_entries into entries (preserve existing committed files)
        for entry in self.staged_entries.iter() {
            self.entries
                .insert(entry.key().clone(), entry.value().clone());
        }

        // Remove deleted entries from committed state
        for entry in self.deleted_entries.iter() {
            self.entries.remove(entry.key());
        }

        // Clear deleted entries after commit
        self.deleted_entries.clear();

        // Clear staged entries after moving them to committed state
        self.staged_entries.clear();
    }

    /// Get the number of tracked files
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the index is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Mark a file as deleted
    pub fn mark_deleted(&self, path: &PathBuf) {
        self.deleted_entries.insert(path.clone());
        // Remove from staged entries if present
        self.staged_entries.remove(path);
    }

    /// Unmark a file as deleted
    #[must_use]
    pub fn unmark_deleted(&self, path: &Path) -> bool {
        self.deleted_entries.remove(path).is_some()
    }

    /// Check if a file is marked for deletion
    #[must_use]
    pub fn is_deleted(&self, path: &Path) -> bool {
        self.deleted_entries.contains(path)
    }

    /// Get all deleted entries as a Vec
    #[must_use]
    pub fn get_deleted_entries(&self) -> Vec<PathBuf> {
        self.deleted_entries
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }
}
