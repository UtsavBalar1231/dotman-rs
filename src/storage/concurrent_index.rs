use super::FileEntry;
use crate::storage::index::Index;
use anyhow::Result;
use dashmap::DashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Thread-safe concurrent index using `DashMap` for lock-free operations
#[derive(Debug, Clone)]
pub struct ConcurrentIndex {
    entries: Arc<DashMap<PathBuf, FileEntry>>,
    staged_entries: Arc<DashMap<PathBuf, FileEntry>>,
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
        for entry in self.staged_entries.iter() {
            self.entries
                .insert(entry.key().clone(), entry.value().clone());
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};
    use std::thread;
    use tempfile::tempdir;

    #[test]
    fn test_concurrent_access() {
        let index = Arc::new(ConcurrentIndex::new());
        let barrier = Arc::new(Barrier::new(10));
        let mut handles = vec![];

        for i in 0..10u32 {
            let index_clone = Arc::clone(&index);
            let barrier_clone = Arc::clone(&barrier);

            let handle = thread::spawn(move || {
                barrier_clone.wait();

                let path = PathBuf::from(format!("file_{i}.txt"));
                let entry = FileEntry {
                    path: path.clone(),
                    hash: format!("hash_{i}"),
                    size: u64::from(i),
                    modified: 0,
                    mode: 0o644,
                    cached_hash: None,
                };

                index_clone.stage_entry(entry);

                // Verify the entry was added
                assert!(index_clone.get_staged_entry(&path).is_some());
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all entries were added
        assert_eq!(index.staged_entries().len(), 10);
    }

    #[test]
    fn test_conversion() -> Result<()> {
        let dir = tempdir()?;
        let _index_path = dir.path().join("index.bin");

        // Create a regular index
        let mut index = Index::new();
        let entry = FileEntry {
            path: PathBuf::from("test.txt"),
            hash: "test_hash".to_string(),
            size: 100,
            modified: 0,
            mode: 0o644,
            cached_hash: None,
        };
        index.entries.insert(PathBuf::from("test.txt"), entry);

        // Convert to concurrent and back
        let concurrent = ConcurrentIndex::from_index(index);
        let converted = concurrent.to_index();

        assert_eq!(converted.entries.len(), 1);
        assert!(converted.entries.contains_key(&PathBuf::from("test.txt")));

        Ok(())
    }
}
