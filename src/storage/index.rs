use super::{FileEntry, FileStatus};
use crate::utils::serialization;
use anyhow::{Context, Result};
use dashmap::DashMap;
use fs4::fs_std::FileExt;
use memmap2::MmapOptions;
use parking_lot::RwLock;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Index {
    pub version: u32,
    pub entries: HashMap<PathBuf, FileEntry>,
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
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }

        let file = File::open(path)?;

        // Acquire shared lock for reading (allows multiple readers)
        file.lock_shared()?;

        let metadata = file.metadata()?;

        let result = if metadata.len() < 1024 {
            // Small index - read directly
            let data = std::fs::read(path)?;
            serialization::deserialize(&data).context("Failed to deserialize index")
        } else {
            // Large index - use memory mapping
            let mmap = unsafe { MmapOptions::new().map(&file)? };
            serialization::deserialize(&mmap).context("Failed to deserialize index")
        };

        // Unlock the file
        file.unlock()?;

        result
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        // Serialize the index data
        let data = serialization::serialize(self)?;

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Open or create file with exclusive access
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;

        // Acquire exclusive lock (only one writer at a time)
        file.lock_exclusive()?;

        // Write the data
        use std::io::Write;
        let mut file_writer = &file;
        file_writer.write_all(&data)?;
        file_writer.flush()?;

        // Unlock the file
        file.unlock()?;

        Ok(())
    }

    pub fn save_merge(&self, path: &Path) -> Result<()> {
        // This method merges with existing index - useful for concurrent adds
        // First, ensure the file exists (create if not)
        if !path.exists() {
            std::fs::write(path, [])?;
        }

        // Open file for reading and writing
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)?;

        // Acquire exclusive lock (only one writer at a time)
        file.lock_exclusive()?;

        // Load existing index while we have the lock
        let mut final_index = if path.exists() && file.metadata()?.len() > 0 {
            let existing_data = std::fs::read(path)?;
            serialization::deserialize::<Index>(&existing_data).unwrap_or_else(|_| Index::new())
        } else {
            Index::new()
        };

        // Merge our entries into the existing index
        for (path, entry) in &self.entries {
            final_index.entries.insert(path.clone(), entry.clone());
        }

        // Serialize and write the merged index
        let data = serialization::serialize(&final_index)?;

        // Truncate and write
        file.set_len(0)?;
        use std::io::Write;
        let mut file_writer = &file;
        file_writer.write_all(&data)?;
        file_writer.flush()?;

        // Unlock the file
        file.unlock()?;

        Ok(())
    }

    pub fn add_entry(&mut self, entry: FileEntry) {
        self.entries.insert(entry.path.clone(), entry);
    }

    pub fn remove_entry(&mut self, path: &Path) -> Option<FileEntry> {
        self.entries.remove(path)
    }

    pub fn get_entry(&self, path: &Path) -> Option<&FileEntry> {
        self.entries.get(path)
    }
}

// High-performance concurrent index for parallel operations
pub struct ConcurrentIndex {
    entries: Arc<DashMap<PathBuf, FileEntry>>,
    version: Arc<RwLock<u32>>,
}

impl Default for ConcurrentIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl ConcurrentIndex {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            version: Arc::new(RwLock::new(1)),
        }
    }

    pub fn from_index(index: Index) -> Self {
        let entries = Arc::new(DashMap::new());
        for (path, entry) in index.entries {
            entries.insert(path, entry);
        }

        Self {
            entries,
            version: Arc::new(RwLock::new(index.version)),
        }
    }

    pub fn to_index(&self) -> Index {
        let mut entries = HashMap::new();
        for entry in self.entries.iter() {
            entries.insert(entry.key().clone(), entry.value().clone());
        }

        Index {
            version: *self.version.read(),
            entries,
        }
    }

    pub fn add_entry(&self, entry: FileEntry) {
        self.entries.insert(entry.path.clone(), entry);
    }

    pub fn add_entries_parallel(&self, entries: Vec<FileEntry>) {
        entries.into_par_iter().for_each(|entry| {
            self.entries.insert(entry.path.clone(), entry);
        });
    }

    pub fn get_status_parallel(&self, _current_files: &[PathBuf]) -> Vec<FileStatus> {
        let mut statuses = Vec::new();

        // Get home directory to resolve relative paths
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));

        // Check for modified and deleted files in parallel
        let index_statuses: Vec<FileStatus> = self
            .entries
            .iter()
            .par_bridge()
            .filter_map(|entry| {
                let stored_path = entry.key();
                let stored_entry = entry.value();

                // Convert stored relative path to absolute for file operations
                let abs_path = if stored_path.is_relative() {
                    home.join(stored_path)
                } else {
                    stored_path.clone()
                };

                if !abs_path.exists() {
                    Some(FileStatus::Deleted(stored_path.clone()))
                } else {
                    // Check if modified by comparing hash
                    match crate::utils::hash::hash_file(&abs_path) {
                        Ok(current_hash) => {
                            if current_hash != stored_entry.hash {
                                // File content has changed
                                Some(FileStatus::Modified(stored_path.clone()))
                            } else {
                                None
                            }
                        }
                        Err(_) => {
                            // If we can't hash the file, check metadata as fallback
                            match std::fs::metadata(&abs_path) {
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
                                        // File has likely been modified
                                        Some(FileStatus::Modified(stored_path.clone()))
                                    } else {
                                        None
                                    }
                                }
                                Err(_) => Some(FileStatus::Deleted(stored_path.clone())),
                            }
                        }
                    }
                }
            })
            .collect();

        statuses.extend(index_statuses);

        // We don't check for untracked files anymore
        // Only explicitly added files are tracked
        // The current_files parameter only contains already tracked files

        statuses
    }
}

// Fast index differ for comparing two indices
pub struct IndexDiffer;

impl IndexDiffer {
    pub fn diff(old: &Index, new: &Index) -> Vec<FileStatus> {
        let mut statuses = Vec::new();

        // Find added and modified files
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

        // Find deleted files
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
    fn test_concurrent_index() {
        let index = ConcurrentIndex::new();

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

        let serialized = index.to_index();
        assert_eq!(serialized.entries.len(), 2);
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

        // Should replace the first entry
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

        // Add 10,000 entries
        for i in 0..10000 {
            index.add_entry(FileEntry {
                path: PathBuf::from(format!("file_{}.txt", i)),
                hash: format!("{:032x}", i),
                size: (i * 100) as u64,
                modified: i as i64,
                mode: 0o644,
            });
        }

        // Save and load
        index.save(&index_path)?;
        let loaded = Index::load(&index_path)?;

        assert_eq!(loaded.entries.len(), 10000);
        assert!(loaded.get_entry(&PathBuf::from("file_5000.txt")).is_some());

        Ok(())
    }

    #[test]
    fn test_concurrent_index_parallel_status() -> Result<()> {
        use tempfile::tempdir;

        let dir = tempdir()?;
        let index = ConcurrentIndex::new();

        // Create actual files and add entries
        let entries: Vec<FileEntry> = (0..10)
            .map(|i| {
                let file_path = dir.path().join(format!("file_{}.txt", i));
                std::fs::write(&file_path, format!("content_{}", i)).unwrap();

                // Store relative paths in the index
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

        // Pass the absolute paths of tracked files (as get_current_files would do)
        let current_files: Vec<PathBuf> =
            entries.iter().map(|e| dir.path().join(&e.path)).collect();

        // Mock home directory as the temp dir for this test
        unsafe {
            std::env::set_var("HOME", dir.path());
        }

        let statuses = index.get_status_parallel(&current_files);

        // All files exist with unchanged content (we're using fake hashes)
        // So they should all be reported as modified
        let modified: Vec<_> = statuses
            .iter()
            .filter(|s| matches!(s, FileStatus::Modified(_)))
            .collect();

        // All 10 files should be detected as modified (hash mismatch)
        assert_eq!(modified.len(), 10);

        // Now delete a file and check again
        std::fs::remove_file(dir.path().join("file_5.txt"))?;

        let statuses = index.get_status_parallel(&current_files);

        let deleted: Vec<_> = statuses
            .iter()
            .filter(|s| matches!(s, FileStatus::Deleted(_)))
            .collect();

        // One file should be deleted
        assert_eq!(deleted.len(), 1);

        Ok(())
    }

    #[test]
    fn test_index_corrupt_handling() -> Result<()> {
        let dir = tempdir()?;
        let index_path = dir.path().join("corrupt.bin");

        // Write garbage data
        std::fs::write(&index_path, b"This is not a valid index file")?;

        // Should fail to load
        let result = Index::load(&index_path);
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_index_concurrent_modifications() -> Result<()> {
        use std::sync::Arc;
        use std::thread;

        let index = Arc::new(ConcurrentIndex::new());

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let index_clone = index.clone();
                thread::spawn(move || {
                    for j in 0..100 {
                        let entry = FileEntry {
                            path: PathBuf::from(format!("thread_{}_file_{}.txt", i, j)),
                            hash: format!("hash_{}_{}", i, j),
                            size: 100,
                            modified: 1000,
                            mode: 0o644,
                        };
                        index_clone.add_entry(entry);
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        let final_index = index.to_index();
        assert_eq!(final_index.entries.len(), 1000); // 10 threads * 100 files

        Ok(())
    }
}
