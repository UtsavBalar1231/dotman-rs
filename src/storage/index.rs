use super::{FileEntry, FileStatus};
use crate::utils::serialization;
use anyhow::{Context, Result};
use fs4::fs_std::FileExt;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::unsafe_derive_deserialize)]
pub struct Index {
    pub version: u32,
    pub entries: HashMap<PathBuf, FileEntry>,
    pub staged_entries: HashMap<PathBuf, FileEntry>,
}

impl Default for Index {
    fn default() -> Self {
        Self::new()
    }
}

impl Index {
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: 1,
            entries: HashMap::new(),
            staged_entries: HashMap::new(),
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

        // Read the entire file first to avoid locking issues
        let data = std::fs::read(path)
            .with_context(|| format!("Failed to read index file: {}", path.display()))?;

        let mut index: Self =
            serialization::deserialize(&data).context("Failed to deserialize index")?;

        if index.staged_entries.is_empty() && !index.entries.is_empty() {
            index.staged_entries = index.entries.clone();
        }

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
            serialization::deserialize::<Self>(&existing_data).unwrap_or_else(|_| Self::new())
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

    pub fn remove_entry(&mut self, path: &Path) -> Option<FileEntry> {
        self.entries.remove(path)
    }

    #[must_use]
    pub fn get_entry(&self, path: &Path) -> Option<&FileEntry> {
        self.entries.get(path)
    }

    pub fn stage_entry(&mut self, entry: FileEntry) {
        self.staged_entries.insert(entry.path.clone(), entry);
    }

    #[must_use]
    pub fn get_staged_entry(&self, path: &Path) -> Option<&FileEntry> {
        self.staged_entries.get(path)
    }

    #[must_use]
    pub fn has_staged_changes(&self) -> bool {
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
            if !self.staged_entries.contains_key(path) {
                return true;
            }
        }

        false
    }

    pub fn commit_staged(&mut self) {
        self.entries = self.staged_entries.clone();
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

                if abs_path.exists() {
                    crate::storage::file_ops::hash_file(
                        &abs_path,
                        stored_entry.cached_hash.as_ref(),
                    )
                    .map_or_else(
                        |_| {
                            std::fs::metadata(&abs_path).map_or_else(
                                |_| Some(FileStatus::Deleted(stored_path.clone())),
                                |metadata| {
                                    let mtime = metadata
                                        .modified()
                                        .ok()
                                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                        .map_or(0, |d| {
                                            i64::try_from(d.as_secs()).unwrap_or(i64::MAX)
                                        });

                                    if mtime != stored_entry.modified
                                        || metadata.len() != stored_entry.size
                                    {
                                        Some(FileStatus::Modified(stored_path.clone()))
                                    } else {
                                        None
                                    }
                                },
                            )
                        },
                        |(current_hash, _cache)| {
                            if current_hash == stored_entry.hash {
                                None
                            } else {
                                Some(FileStatus::Modified(stored_path.clone()))
                            }
                        },
                    )
                } else {
                    Some(FileStatus::Deleted(stored_path.clone()))
                }
            })
            .collect();

        statuses.extend(index_statuses);
        statuses
    }
}

pub struct IndexDiffer;

impl IndexDiffer {
    /// Compute the difference between two indices
    /// Returns a list of file statuses indicating added, modified, or deleted files
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_index_save_load() -> Result<()> {
        let dir = tempdir()?;
        let index_path = dir.path().join("index.bin");

        let mut index = Index::new();
        let entry = FileEntry {
            path: PathBuf::from("test.txt"),
            hash: "abc123".to_string(),
            size: 100,
            modified: 1_234_567_890,
            mode: 0o644,
            cached_hash: None,
        };
        // Add to both entries and staged_entries as would happen in real usage
        index.add_entry(entry.clone());
        index.stage_entry(entry);

        index.save(&index_path)?;

        let loaded = Index::load(&index_path)?;
        // stage_entry adds to staged_entries, and on load it gets copied to entries
        assert_eq!(loaded.entries.len(), 1);
        assert!(loaded.entries.contains_key(&PathBuf::from("test.txt")));
        assert_eq!(loaded.staged_entries.len(), 1);

        Ok(())
    }

    #[test]
    fn test_index_add_parallel() {
        let mut index = Index::new();

        let entries = vec![
            FileEntry {
                path: PathBuf::from("file1.txt"),
                hash: "hash1".to_string(),
                size: 100,
                modified: 1000,
                mode: 0o644,
                cached_hash: None,
            },
            FileEntry {
                path: PathBuf::from("file2.txt"),
                hash: "hash2".to_string(),
                size: 200,
                modified: 2000,
                mode: 0o644,
                cached_hash: None,
            },
        ];

        index.add_entries_parallel(entries);

        assert_eq!(index.entries.len(), 2);
    }

    #[test]
    fn test_index_differ() {
        let mut old = Index::new();
        old.add_entry(FileEntry {
            path: PathBuf::from("existing.txt"),
            hash: "old_hash".to_string(),
            size: 100,
            modified: 1000,
            mode: 0o644,
            cached_hash: None,
        });
        old.add_entry(FileEntry {
            path: PathBuf::from("deleted.txt"),
            hash: "del_hash".to_string(),
            size: 50,
            modified: 500,
            mode: 0o644,
            cached_hash: None,
        });

        let mut new = Index::new();
        new.add_entry(FileEntry {
            path: PathBuf::from("existing.txt"),
            hash: "new_hash".to_string(),
            size: 150,
            modified: 2000,
            mode: 0o644,
            cached_hash: None,
        });
        new.add_entry(FileEntry {
            path: PathBuf::from("added.txt"),
            hash: "add_hash".to_string(),
            size: 75,
            modified: 1500,
            mode: 0o644,
            cached_hash: None,
        });

        let diff = IndexDiffer::diff(&old, &new);

        assert_eq!(diff.len(), 3);
        assert!(
            diff.iter().any(
                |s| matches!(s, FileStatus::Modified(p) if p == &PathBuf::from("existing.txt"))
            )
        );
        assert!(
            diff.iter()
                .any(|s| matches!(s, FileStatus::Added(p) if p == &PathBuf::from("added.txt")))
        );
        assert!(
            diff.iter()
                .any(|s| matches!(s, FileStatus::Deleted(p) if p == &PathBuf::from("deleted.txt")))
        );
    }

    #[test]
    fn test_index_empty() {
        let index = Index::new();
        assert_eq!(index.entries.len(), 0);
        assert_eq!(index.version, 1);
    }

    #[test]
    fn test_index_duplicate_entries() {
        let mut index = Index::new();
        let entry1 = FileEntry {
            path: PathBuf::from("duplicate.txt"),
            hash: "hash1".to_string(),
            size: 100,
            modified: 1000,
            mode: 0o644,
            cached_hash: None,
        };
        let entry2 = FileEntry {
            path: PathBuf::from("duplicate.txt"),
            hash: "hash2".to_string(),
            size: 200,
            modified: 2000,
            mode: 0o644,
            cached_hash: None,
        };

        index.add_entry(entry1);
        index.add_entry(entry2);

        assert_eq!(index.entries.len(), 1);
        let stored = index.get_entry(&PathBuf::from("duplicate.txt")).unwrap();
        assert_eq!(stored.hash, "hash2");
        assert_eq!(stored.size, 200);
    }

    #[test]
    fn test_index_remove_nonexistent() {
        let mut index = Index::new();
        let removed = index.remove_entry(&PathBuf::from("nonexistent.txt"));
        assert!(removed.is_none());
    }

    #[test]
    fn test_index_large_scale() -> Result<()> {
        let dir = tempdir()?;
        let index_path = dir.path().join("large_index.bin");

        let mut index = Index::new();

        for i in 0..10_000 {
            let entry = FileEntry {
                path: PathBuf::from(format!("file_{i}.txt")),
                hash: format!("{i:032x}"),
                size: u64::try_from(i).unwrap_or(0) * 100,
                modified: i64::from(i),
                mode: 0o644,
                cached_hash: None,
            };
            index.add_entry(entry);
        }

        index.save(&index_path)?;
        let loaded = Index::load(&index_path)?;

        assert_eq!(loaded.entries.len(), 10_000);
        assert!(loaded.get_entry(&PathBuf::from("file_5000.txt")).is_some());

        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_index_parallel_status() -> Result<()> {
        use tempfile::tempdir;

        let dir = tempdir()?;
        let mut index = Index::new();

        let entries: Vec<FileEntry> = (0..10)
            .map(|i| {
                let file_path = dir.path().join(format!("file_{i}.txt"));
                std::fs::write(&file_path, format!("content_{i}")).unwrap();

                FileEntry {
                    path: PathBuf::from(format!("file_{i}.txt")),
                    hash: format!("hash_{i}"),
                    size: 100,
                    modified: 1000,
                    mode: 0o644,
                    cached_hash: None,
                }
            })
            .collect();

        index.add_entries_parallel(entries.clone());

        let current_files: Vec<PathBuf> =
            entries.iter().map(|e| dir.path().join(&e.path)).collect();

        unsafe {
            std::env::set_var("HOME", dir.path());
        }

        let statuses = index.get_status_parallel(&current_files);

        let modified: usize = statuses
            .iter()
            .filter(|s| matches!(s, FileStatus::Modified(_)))
            .count();

        assert_eq!(modified, 10);

        std::fs::remove_file(dir.path().join("file_5.txt"))?;

        let statuses = index.get_status_parallel(&current_files);

        let deleted: usize = statuses
            .iter()
            .filter(|s| matches!(s, FileStatus::Deleted(_)))
            .count();

        assert_eq!(deleted, 1);

        Ok(())
    }

    #[test]
    fn test_index_corrupt_handling() -> Result<()> {
        let dir = tempdir()?;
        let index_path = dir.path().join("corrupt.bin");

        std::fs::write(&index_path, b"This is not a valid index file")?;

        let result = Index::load(&index_path);
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_recover_from_corrupted_index() -> Result<()> {
        let dir = tempdir()?;

        let mut index = Index::new();
        index.add_entry(FileEntry {
            path: PathBuf::from("test.txt"),
            hash: "valid_hash".to_string(),
            size: 100,
            modified: 1_234_567_890,
            mode: 0o644,
            cached_hash: None,
        });

        let index_path = dir.path().join("index.bin");
        index.save(&index_path)?;

        std::fs::write(
            &index_path,
            b"This is corrupted binary data that's not valid bincode",
        )?;

        let result = Index::load(&index_path);
        assert!(result.is_err());

        let recovered_index = Index::new();
        recovered_index.save(&index_path)?;

        let loaded = Index::load(&index_path)?;
        assert_eq!(loaded.entries.len(), 0);
        assert_eq!(loaded.version, 1);

        Ok(())
    }

    #[test]
    fn test_partial_write_corruption() -> Result<()> {
        let dir = tempdir()?;

        let mut index = Index::new();
        let entry = FileEntry {
            path: PathBuf::from("test.txt"),
            hash: "test_hash".to_string(),
            size: 1234,
            modified: 1_234_567_890,
            mode: 0o644,
            cached_hash: None,
        };
        index.add_entry(entry);

        let index_path = dir.path().join("index.bin");
        index.save(&index_path)?;

        let valid_data = std::fs::read(&index_path)?;

        // Simulate various partial write scenarios
        for partial_size in [1, 4, 8, 16, 32, valid_data.len() / 4, valid_data.len() / 2] {
            if partial_size < valid_data.len() {
                std::fs::write(&index_path, &valid_data[..partial_size])?;

                let result = Index::load(&index_path);
                assert!(
                    result.is_err(),
                    "Should reject partial write at {partial_size} bytes",
                );
            }
        }

        // Restore valid data and verify it still loads
        std::fs::write(&index_path, &valid_data)?;
        let restored = Index::load(&index_path)?;
        assert_eq!(restored.entries.len(), 1);

        Ok(())
    }

    #[test]
    fn test_index_consistency_validation() -> Result<()> {
        let dir = tempdir()?;

        // Create index with known entries
        let mut index = Index::new();

        let file1 = dir.path().join("file1.txt");
        let file2 = dir.path().join("file2.txt");
        std::fs::write(&file1, "content1")?;
        std::fs::write(&file2, "content2")?;

        let entry1 = FileEntry {
            path: file1,
            hash: "hash1".to_string(),
            size: 8,
            modified: 1_234_567_890,
            mode: 0o644,
            cached_hash: None,
        };
        index.add_entry(entry1);

        let entry2 = FileEntry {
            path: file2,
            hash: "hash2".to_string(),
            size: 8,
            modified: 1_234_567_891,
            mode: 0o644,
            cached_hash: None,
        };
        index.add_entry(entry2);

        let index_path = dir.path().join("index.bin");
        index.save(&index_path)?;

        let mut loaded = Index::load(&index_path)?;
        if let Some(entry) = loaded.entries.values_mut().next() {
            entry.hash = "invalid_hash".to_string();
            entry.size = 999_999; // Wrong size
            entry.modified = 0; // Wrong timestamp
        }

        loaded.save(&index_path)?;

        // Load corrupted index - should load but with invalid data
        let corrupted = Index::load(&index_path)?;

        // Verify the corruption is present in the loaded index
        // The corrupted entry should have the invalid values we set
        let has_corrupted = corrupted.entries.values().any(|entry| {
            entry.hash == "invalid_hash" && entry.size == 999_999 && entry.modified == 0
        });

        assert!(
            has_corrupted,
            "Index should contain the corrupted entry with invalid values"
        );

        // This demonstrates that validation logic would need to check
        // actual file hashes against stored hashes

        Ok(())
    }
}
