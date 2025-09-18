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
