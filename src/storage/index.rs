use super::{FileEntry, FileStatus};
use crate::utils::serialization;
use anyhow::{Context, Result};
use fs4::fs_std::FileExt;
use memmap2::MmapOptions;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Index {
    pub version: u32,
    pub entries: HashMap<PathBuf, FileEntry>,
    #[serde(default)]
    pub staged_entries: HashMap<PathBuf, FileEntry>,
}

impl Default for Index {
    fn default() -> Self {
        Self::new()
    }
}

impl Index {
    pub fn new() -> Self {
        Self {
            version: 1,
            entries: HashMap::new(),
            staged_entries: HashMap::new(),
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }

        let file = File::open(path)
            .with_context(|| format!("Failed to open index file: {}", path.display()))?;

        file.lock_shared()
            .context("Failed to acquire shared lock on index file")?;

        let metadata = file
            .metadata()
            .context("Failed to get index file metadata")?;

        let mut index: Index = if metadata.len() < 1024 {
            let data = std::fs::read(path)
                .with_context(|| format!("Failed to read index file: {}", path.display()))?;
            serialization::deserialize(&data).context("Failed to deserialize index")?
        } else {
            let mmap = unsafe {
                MmapOptions::new()
                    .map(&file)
                    .context("Failed to memory-map index file")?
            };
            serialization::deserialize(&mmap).context("Failed to deserialize index")?
        };

        file.unlock().context("Failed to unlock index file")?;

        if index.staged_entries.is_empty() && !index.entries.is_empty() {
            index.staged_entries = index.entries.clone();
        }

        Ok(index)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let data = serialization::serialize(self).context("Failed to serialize index")?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .with_context(|| {
                format!("Failed to open index file for writing: {}", path.display())
            })?;

        file.lock_exclusive()
            .context("Failed to acquire exclusive lock on index file")?;

        use std::io::Write;
        let mut file_writer = &file;
        file_writer
            .write_all(&data)
            .context("Failed to write index data")?;
        file_writer.flush().context("Failed to flush index data")?;

        file.unlock().context("Failed to unlock index file")?;

        Ok(())
    }

    pub fn save_merge(&self, path: &Path) -> Result<()> {
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
            serialization::deserialize::<Index>(&existing_data).unwrap_or_else(|_| Index::new())
        } else {
            Index::new()
        };

        for (path, entry) in &self.entries {
            final_index.entries.insert(path.clone(), entry.clone());
        }

        for (path, entry) in &self.staged_entries {
            final_index
                .staged_entries
                .insert(path.clone(), entry.clone());
        }

        let data =
            serialization::serialize(&final_index).context("Failed to serialize merged index")?;

        file.set_len(0).context("Failed to truncate index file")?;
        use std::io::Write;
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

    pub fn get_entry(&self, path: &Path) -> Option<&FileEntry> {
        self.entries.get(path)
    }

    pub fn stage_entry(&mut self, entry: FileEntry) {
        self.staged_entries.insert(entry.path.clone(), entry);
    }

    pub fn get_staged_entry(&self, path: &Path) -> Option<&FileEntry> {
        self.staged_entries.get(path)
    }

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

                if !abs_path.exists() {
                    Some(FileStatus::Deleted(stored_path.clone()))
                } else {
                    match crate::utils::hash::hash_file(&abs_path) {
                        Ok(current_hash) => {
                            if current_hash != stored_entry.hash {
                                Some(FileStatus::Modified(stored_path.clone()))
                            } else {
                                None
                            }
                        }
                        Err(_) => match std::fs::metadata(&abs_path) {
                            Ok(metadata) => {
                                let mtime = metadata
                                    .modified()
                                    .ok()
                                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                    .map(|d| d.as_secs() as i64)
                                    .unwrap_or(0);

                                if mtime != stored_entry.modified
                                    || metadata.len() != stored_entry.size
                                {
                                    Some(FileStatus::Modified(stored_path.clone()))
                                } else {
                                    None
                                }
                            }
                            Err(_) => Some(FileStatus::Deleted(stored_path.clone())),
                        },
                    }
                }
            })
            .collect();

        statuses.extend(index_statuses);
        statuses
    }
}

pub struct IndexDiffer;

impl IndexDiffer {
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
        index.add_entry(FileEntry {
            path: PathBuf::from("test.txt"),
            hash: "abc123".to_string(),
            size: 100,
            modified: 1234567890,
            mode: 0o644,
        });

        index.save(&index_path)?;

        let loaded = Index::load(&index_path)?;
        assert_eq!(loaded.entries.len(), 1);
        assert!(loaded.entries.contains_key(&PathBuf::from("test.txt")));

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
            },
            FileEntry {
                path: PathBuf::from("file2.txt"),
                hash: "hash2".to_string(),
                size: 200,
                modified: 2000,
                mode: 0o644,
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
        });
        old.add_entry(FileEntry {
            path: PathBuf::from("deleted.txt"),
            hash: "del_hash".to_string(),
            size: 50,
            modified: 500,
            mode: 0o644,
        });

        let mut new = Index::new();
        new.add_entry(FileEntry {
            path: PathBuf::from("existing.txt"),
            hash: "new_hash".to_string(),
            size: 150,
            modified: 2000,
            mode: 0o644,
        });
        new.add_entry(FileEntry {
            path: PathBuf::from("added.txt"),
            hash: "add_hash".to_string(),
            size: 75,
            modified: 1500,
            mode: 0o644,
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
        };
        let entry2 = FileEntry {
            path: PathBuf::from("duplicate.txt"),
            hash: "hash2".to_string(),
            size: 200,
            modified: 2000,
            mode: 0o644,
        };

        index.add_entry(entry1);
        index.add_entry(entry2.clone());

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

        for i in 0..10000 {
            index.add_entry(FileEntry {
                path: PathBuf::from(format!("file_{}.txt", i)),
                hash: format!("{:032x}", i),
                size: (i * 100) as u64,
                modified: i as i64,
                mode: 0o644,
            });
        }

        index.save(&index_path)?;
        let loaded = Index::load(&index_path)?;

        assert_eq!(loaded.entries.len(), 10000);
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
                let file_path = dir.path().join(format!("file_{}.txt", i));
                std::fs::write(&file_path, format!("content_{}", i)).unwrap();

                FileEntry {
                    path: PathBuf::from(format!("file_{}.txt", i)),
                    hash: format!("hash_{}", i),
                    size: 100,
                    modified: 1000,
                    mode: 0o644,
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

        let modified: Vec<_> = statuses
            .iter()
            .filter(|s| matches!(s, FileStatus::Modified(_)))
            .collect();

        assert_eq!(modified.len(), 10);

        std::fs::remove_file(dir.path().join("file_5.txt"))?;

        let statuses = index.get_status_parallel(&current_files);

        let deleted: Vec<_> = statuses
            .iter()
            .filter(|s| matches!(s, FileStatus::Deleted(_)))
            .collect();

        assert_eq!(deleted.len(), 1);

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
            modified: 1234567890,
            mode: 0o644,
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
        index.add_entry(FileEntry {
            path: PathBuf::from("test.txt"),
            hash: "test_hash".to_string(),
            size: 1234,
            modified: 1234567890,
            mode: 0o644,
        });

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
                    "Should reject partial write at {} bytes",
                    partial_size
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

        index.add_entry(FileEntry {
            path: file1.clone(),
            hash: "hash1".to_string(),
            size: 8,
            modified: 1234567890,
            mode: 0o644,
        });

        index.add_entry(FileEntry {
            path: file2.clone(),
            hash: "hash2".to_string(),
            size: 8,
            modified: 1234567891,
            mode: 0o644,
        });

        let index_path = dir.path().join("index.bin");
        index.save(&index_path)?;

        let mut loaded = Index::load(&index_path)?;
        if let Some(entry) = loaded.entries.values_mut().next() {
            entry.hash = "invalid_hash".to_string();
            entry.size = 999999; // Wrong size
            entry.modified = 0; // Wrong timestamp
        }

        loaded.save(&index_path)?;

        // Load corrupted index - should load but with invalid data
        let corrupted = Index::load(&index_path)?;

        // Verify the corruption is present in the loaded index
        // The corrupted entry should have the invalid values we set
        let has_corrupted = corrupted.entries.values().any(|entry| {
            entry.hash == "invalid_hash" && entry.size == 999999 && entry.modified == 0
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
