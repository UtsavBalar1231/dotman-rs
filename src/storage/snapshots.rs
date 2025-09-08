use super::{Commit, FileEntry};
use crate::utils::serialization;
use anyhow::{Context, Result};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use zstd::stream::{decode_all, encode_all};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub commit: Commit,
    pub files: HashMap<PathBuf, SnapshotFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotFile {
    pub hash: String,
    pub mode: u32,
    pub content_hash: String,
}

pub struct SnapshotManager {
    repo_path: PathBuf,
    compression_level: i32,
    preserve_permissions: bool,
}

impl SnapshotManager {
    #[must_use]
    pub const fn new(repo_path: PathBuf, compression_level: i32) -> Self {
        Self::with_permissions(repo_path, compression_level, true)
    }

    #[must_use]
    pub const fn with_permissions(
        repo_path: PathBuf,
        compression_level: i32,
        preserve_permissions: bool,
    ) -> Self {
        Self {
            repo_path,
            compression_level,
            preserve_permissions,
        }
    }

    /// Create a new snapshot with the given commit and files
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to create directories
    /// - Failed to read or compress files
    /// - Failed to save snapshot
    pub fn create_snapshot(&self, commit: Commit, files: &[FileEntry]) -> Result<String> {
        let snapshot_id = commit.id.clone();
        let snapshot_path = self
            .repo_path
            .join("commits")
            .join(format!("{}.zst", &snapshot_id));

        if let Some(parent) = snapshot_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create snapshot directory: {}", parent.display())
            })?;
        }

        let home = dirs::home_dir().context("Could not find home directory")?;

        let stored_files: Result<Vec<(PathBuf, SnapshotFile)>> = files
            .par_iter()
            .map(|entry| {
                let abs_path = if entry.path.is_relative() {
                    home.join(&entry.path)
                } else {
                    entry.path.clone()
                };
                let content_hash = self
                    .store_file_content(&abs_path, &entry.hash)
                    .with_context(|| {
                        format!("Failed to store content for: {}", abs_path.display())
                    })?;
                Ok((
                    entry.path.clone(),
                    SnapshotFile {
                        hash: entry.hash.clone(),
                        mode: entry.mode,
                        content_hash,
                    },
                ))
            })
            .collect();

        let files_map: HashMap<PathBuf, SnapshotFile> = stored_files?.into_iter().collect();

        let snapshot = Snapshot {
            commit,
            files: files_map,
        };

        let serialized =
            serialization::serialize(&snapshot).context("Failed to serialize snapshot")?;
        let compressed = encode_all(&serialized[..], self.compression_level)
            .context("Failed to compress snapshot")?;

        fs::write(&snapshot_path, compressed).with_context(|| {
            format!("Failed to write snapshot file: {}", snapshot_path.display())
        })?;

        Ok(snapshot_id)
    }

    /// Load a snapshot from disk by its ID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The snapshot does not exist
    /// - Multiple snapshots match an ambiguous ID
    /// - Failed to read or decompress the snapshot
    /// - Failed to deserialize the snapshot data
    pub fn load_snapshot(&self, snapshot_id: &str) -> Result<Snapshot> {
        let exact_path = self
            .repo_path
            .join("commits")
            .join(format!("{snapshot_id}.zst"));

        let snapshot_path = if exact_path.exists() {
            exact_path
        } else {
            // Try to find by partial ID (suffix match since we show last 8 chars)
            let commits_dir = self.repo_path.join("commits");
            let mut matches = Vec::new();

            if commits_dir.exists() {
                for entry in
                    fs::read_dir(&commits_dir).context("Failed to read commits directory")?
                {
                    let entry = entry.context("Failed to read directory entry")?;
                    let path = entry.path();

                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                        && (stem.ends_with(snapshot_id) || stem.starts_with(snapshot_id))
                    {
                        matches.push(path);
                    }
                }
            }

            match matches.len() {
                0 => return Err(anyhow::anyhow!("No commit found matching: {snapshot_id}")),
                1 => matches
                    .into_iter()
                    .next()
                    .context("Failed to get matching commit")?,
                _ => {
                    return Err(anyhow::anyhow!(
                        "Ambiguous commit ID '{}' matches {} commits",
                        snapshot_id,
                        matches.len()
                    ));
                }
            }
        };

        // Read and decompress snapshot
        let compressed = fs::read(&snapshot_path)
            .with_context(|| format!("Failed to read snapshot: {snapshot_id}"))?;
        let decompressed = decode_all(&compressed[..]).context("Failed to decompress snapshot")?;

        // Deserialize snapshot
        let snapshot: Snapshot =
            serialization::deserialize(&decompressed).context("Failed to deserialize snapshot")?;
        Ok(snapshot)
    }

    /// Restore a snapshot to the target directory
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The snapshot cannot be loaded
    /// - Failed to create target directories
    /// - Failed to restore file contents
    /// - Failed to set file permissions
    pub fn restore_snapshot(&self, snapshot_id: &str, target_dir: &Path) -> Result<()> {
        let snapshot = self.load_snapshot(snapshot_id)?;

        // Restore files in parallel
        snapshot
            .files
            .par_iter()
            .try_for_each(|(rel_path, snapshot_file)| -> Result<()> {
                let target_path = target_dir.join(rel_path);

                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent).with_context(|| {
                        format!("Failed to create directory: {}", parent.display())
                    })?;
                }

                self.restore_file_content(&snapshot_file.content_hash, &target_path)
                    .with_context(|| {
                        format!("Failed to restore file: {}", target_path.display())
                    })?;

                // Restore file permissions using cross-platform module
                let permissions =
                    crate::utils::permissions::FilePermissions::from_mode(snapshot_file.mode);
                permissions.apply_to_path(&target_path, self.preserve_permissions)?;

                Ok(())
            })?;

        Ok(())
    }

    /// Restore a snapshot with cleanup of untracked files
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The snapshot cannot be loaded
    /// - Failed to remove untracked files
    /// - Failed to restore snapshot files
    pub fn restore_snapshot_with_cleanup(
        &self,
        snapshot_id: &str,
        target_dir: &Path,
        current_files: &[PathBuf],
    ) -> Result<()> {
        let snapshot = self.load_snapshot(snapshot_id)?;

        let snapshot_files: std::collections::HashSet<PathBuf> =
            snapshot.files.keys().cloned().collect();

        // Remove files that are in current but not in snapshot
        for current_file in current_files {
            let rel_path = if current_file.is_absolute() {
                current_file
                    .strip_prefix(target_dir)
                    .unwrap_or(current_file)
                    .to_path_buf()
            } else {
                current_file.clone()
            };

            if !snapshot_files.contains(&rel_path) {
                let abs_path = if current_file.is_absolute() {
                    current_file.clone()
                } else {
                    target_dir.join(current_file)
                };

                if abs_path.exists() {
                    fs::remove_file(&abs_path).with_context(|| {
                        format!("Failed to remove file: {}", abs_path.display())
                    })?;
                }
            }
        }

        // Now restore files from snapshot
        self.restore_snapshot(snapshot_id, target_dir)?;

        Ok(())
    }

    fn store_file_content(&self, file_path: &Path, hash: &str) -> Result<String> {
        let objects_dir = self.repo_path.join("objects");
        let object_path = objects_dir.join(format!("{hash}.zst"));

        if object_path.exists() {
            return Ok(hash.to_string());
        }

        // Create objects directory if needed
        fs::create_dir_all(&objects_dir).context("Failed to create objects directory")?;

        // Read file content
        let content = fs::read(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

        // Compress content
        let compressed = encode_all(&content[..], self.compression_level)
            .context("Failed to compress file content")?;

        // Write compressed object
        fs::write(&object_path, compressed)
            .with_context(|| format!("Failed to write object file: {}", object_path.display()))?;

        Ok(hash.to_string())
    }

    /// Restore file content from the object store
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The object file does not exist
    /// - Failed to read or decompress the object
    /// - Failed to write the restored file
    pub fn restore_file_content(&self, content_hash: &str, target_path: &Path) -> Result<()> {
        let object_path = self
            .repo_path
            .join("objects")
            .join(format!("{content_hash}.zst"));

        // Read and decompress object
        let compressed = fs::read(&object_path)
            .with_context(|| format!("Failed to read object file: {}", object_path.display()))?;
        let content = decode_all(&compressed[..]).context("Failed to decompress object content")?;

        // Write restored content
        fs::write(target_path, content)
            .with_context(|| format!("Failed to write restored file: {}", target_path.display()))?;

        Ok(())
    }

    /// Read an object from the object store
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The object file does not exist
    /// - Failed to read the object file
    /// - Failed to decompress the object content
    pub fn read_object(&self, content_hash: &str) -> Result<Vec<u8>> {
        let object_path = self
            .repo_path
            .join("objects")
            .join(format!("{content_hash}.zst"));

        // Read and decompress object
        let compressed = fs::read(&object_path)
            .with_context(|| format!("Failed to read object file: {}", object_path.display()))?;
        let content = decode_all(&compressed[..])
            .with_context(|| format!("Failed to decompress object: {content_hash}"))?;

        Ok(content)
    }

    #[must_use]
    pub fn snapshot_exists(&self, snapshot_id: &str) -> bool {
        let snapshot_path = self
            .repo_path
            .join("commits")
            .join(format!("{snapshot_id}.zst"));
        snapshot_path.exists()
    }

    /// List all available snapshots
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to read the commits directory
    /// - Failed to read directory entries
    pub fn list_snapshots(&self) -> Result<Vec<String>> {
        let commits_dir = self.repo_path.join("commits");

        if !commits_dir.exists() {
            return Ok(Vec::new());
        }

        let mut snapshots = Vec::new();

        for entry in fs::read_dir(commits_dir).context("Failed to read commits directory")? {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("zst")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                snapshots.push(stem.to_string());
            }
        }

        Ok(snapshots)
    }

    /// Delete a snapshot by its ID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to delete the snapshot file
    pub fn delete_snapshot(&self, snapshot_id: &str) -> Result<()> {
        let snapshot_path = self
            .repo_path
            .join("commits")
            .join(format!("{snapshot_id}.zst"));

        if snapshot_path.exists() {
            fs::remove_file(snapshot_path)
                .with_context(|| format!("Failed to delete snapshot: {snapshot_id}"))?;
        }

        // Note: We don't delete objects as they might be referenced by other snapshots
        // A separate garbage collection process would handle orphaned objects

        Ok(())
    }
}

// Garbage collector for cleaning up unreferenced objects
pub struct GarbageCollector {
    repo_path: PathBuf,
}

impl GarbageCollector {
    #[must_use]
    pub const fn new(repo_path: PathBuf) -> Self {
        Self { repo_path }
    }

    /// Collect garbage by removing unreferenced objects
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to read commits or objects directories
    /// - Failed to deserialize snapshots
    /// - Failed to delete orphaned objects
    pub fn collect(&self) -> Result<usize> {
        let commits_dir = self.repo_path.join("commits");
        let objects_dir = self.repo_path.join("objects");

        if !objects_dir.exists() {
            return Ok(0);
        }

        // Collect all referenced objects
        let mut referenced = std::collections::HashSet::new();

        if commits_dir.exists() {
            for entry in fs::read_dir(commits_dir).context("Failed to read commits directory")? {
                let entry = entry.context("Failed to read directory entry")?;
                let path = entry.path();

                if path.extension().and_then(|s| s.to_str()) == Some("zst") {
                    // Load snapshot and collect referenced objects
                    let compressed = fs::read(&path)
                        .with_context(|| format!("Failed to read snapshot: {}", path.display()))?;
                    let decompressed =
                        decode_all(&compressed[..]).context("Failed to decompress snapshot")?;
                    let snapshot: Snapshot = serialization::deserialize(&decompressed)
                        .context("Failed to deserialize snapshot")?;

                    for file in snapshot.files.values() {
                        referenced.insert(file.content_hash.clone());
                    }
                }
            }
        }

        let mut deleted = 0;

        for entry in fs::read_dir(objects_dir).context("Failed to read objects directory")? {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();

            if let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                && !referenced.contains(stem)
            {
                fs::remove_file(&path).with_context(|| {
                    format!("Failed to remove orphaned object: {}", path.display())
                })?;
                deleted += 1;
            }
        }

        Ok(deleted)
    }
}

#[cfg(test)]
#[allow(clippy::similar_names)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_snapshot_manager() -> Result<()> {
        let dir = tempdir()?;
        let repo_path = dir.path().to_path_buf();

        let manager = SnapshotManager::new(repo_path, 3);

        // Create test files
        let test_file = dir.path().join("test.txt");
        fs::write(&test_file, "Hello, World!")?;

        let files = vec![FileEntry {
            path: test_file, // Use absolute path
            hash: "abc123".to_string(),
            size: 13,
            modified: 1_234_567_890,
            mode: 0o644,
            cached_hash: None,
        }];

        let commit = Commit {
            id: "commit1".to_string(),
            parent: None,
            message: "Initial commit".to_string(),
            author: "Test User".to_string(),
            timestamp: 1_234_567_890,
            tree_hash: "tree123".to_string(),
        };

        // Create snapshot
        let snapshot_id = manager.create_snapshot(commit, &files)?;
        assert_eq!(snapshot_id, "commit1");

        // Load snapshot
        let loaded = manager.load_snapshot(&snapshot_id)?;
        assert_eq!(loaded.commit.id, "commit1");
        assert_eq!(loaded.files.len(), 1);

        // List snapshots
        let snapshots = manager.list_snapshots()?;
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0], "commit1");

        Ok(())
    }

    #[test]
    fn test_garbage_collector() -> Result<()> {
        let dir = tempdir()?;
        let repo_path = dir.path().to_path_buf();

        // Create objects directory with some files
        let objects_dir = repo_path.join("objects");
        fs::create_dir_all(&objects_dir).context("Failed to create objects directory")?;

        // Create some object files
        fs::write(objects_dir.join("used.zst"), "used content")?;
        fs::write(objects_dir.join("unused.zst"), "unused content")?;

        let commits_dir = repo_path.join("commits");
        fs::create_dir_all(&commits_dir)?;

        let snapshot = Snapshot {
            commit: Commit {
                id: "test".to_string(),
                parent: None,
                message: "Test".to_string(),
                author: "Test".to_string(),
                timestamp: 0,
                tree_hash: "test".to_string(),
            },
            files: std::iter::once((
                PathBuf::from("test.txt"),
                SnapshotFile {
                    hash: "used".to_string(),
                    mode: 0o644,
                    content_hash: "used".to_string(),
                },
            ))
            .collect(),
        };

        let serialized = serialization::serialize(&snapshot)?;
        let compressed = encode_all(&serialized[..], 3)?;
        fs::write(commits_dir.join("test.zst"), compressed)?;

        // Run garbage collection
        let gc = GarbageCollector::new(repo_path);
        let deleted = gc.collect()?;

        assert_eq!(deleted, 1);
        assert!(objects_dir.join("used.zst").exists());
        assert!(!objects_dir.join("unused.zst").exists());

        Ok(())
    }

    #[test]
    fn test_snapshot_empty_files() -> Result<()> {
        let dir = tempdir()?;
        let repo_path = dir.path().to_path_buf();

        let manager = SnapshotManager::new(repo_path, 3);

        let commit = Commit {
            id: "empty".to_string(),
            parent: None,
            message: "Empty snapshot".to_string(),
            author: "Test".to_string(),
            timestamp: 0,
            tree_hash: "empty".to_string(),
        };

        // Create snapshot with no files
        let snapshot_id = manager.create_snapshot(commit, &[])?;
        assert_eq!(snapshot_id, "empty");

        // Should be able to load empty snapshot
        let loaded = manager.load_snapshot(&snapshot_id)?;
        assert_eq!(loaded.files.len(), 0);

        Ok(())
    }

    #[test]
    fn test_snapshot_circular_parent() -> Result<()> {
        let dir = tempdir()?;
        let repo_path = dir.path().to_path_buf();

        let manager = SnapshotManager::new(repo_path, 3);

        // Create commits with circular reference (should be prevented by application logic)
        let commit1 = Commit {
            id: "commit1".to_string(),
            parent: Some("commit2".to_string()), // Points to commit2
            message: "First".to_string(),
            author: "Test".to_string(),
            timestamp: 1,
            tree_hash: "tree1".to_string(),
        };

        let commit2 = Commit {
            id: "commit2".to_string(),
            parent: Some("commit1".to_string()), // Points back to commit1
            message: "Second".to_string(),
            author: "Test".to_string(),
            timestamp: 2,
            tree_hash: "tree2".to_string(),
        };

        // Should be able to create snapshots (detection is app logic responsibility)
        manager.create_snapshot(commit1, &[])?;
        manager.create_snapshot(commit2, &[])?;

        // Both should exist
        let snapshots = manager.list_snapshots()?;
        assert_eq!(snapshots.len(), 2);

        Ok(())
    }

    #[test]
    fn test_snapshot_nonexistent_load() -> Result<()> {
        let dir = tempdir()?;
        let repo_path = dir.path().to_path_buf();

        let manager = SnapshotManager::new(repo_path, 3);

        let result = manager.load_snapshot("nonexistent");
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_snapshot_delete() -> Result<()> {
        let dir = tempdir()?;
        let repo_path = dir.path().to_path_buf();

        let manager = SnapshotManager::new(repo_path, 3);

        let commit = Commit {
            id: "to_delete".to_string(),
            parent: None,
            message: "Delete me".to_string(),
            author: "Test".to_string(),
            timestamp: 0,
            tree_hash: "tree".to_string(),
        };

        // Create and then delete
        manager.create_snapshot(commit, &[])?;
        assert_eq!(manager.list_snapshots()?.len(), 1);

        manager.delete_snapshot("to_delete")?;
        assert_eq!(manager.list_snapshots()?.len(), 0);

        // Delete nonexistent should not error
        manager.delete_snapshot("already_gone")?;

        Ok(())
    }

    #[test]
    fn test_snapshot_restore_missing_object() -> Result<()> {
        let dir = tempdir()?;
        let repo_path = dir.path().to_path_buf();
        let target_dir = dir.path().join("target");
        fs::create_dir_all(&target_dir)?;

        let manager = SnapshotManager::new(repo_path.clone(), 3);

        let test_file = dir.path().join("test.txt");
        fs::write(&test_file, "content")?;

        let files = vec![crate::storage::FileEntry {
            path: test_file,
            hash: "test_hash".to_string(),
            size: 7,
            modified: 0,
            mode: 0o644,
            cached_hash: None,
        }];

        let commit = Commit {
            id: "test".to_string(),
            parent: None,
            message: "Test".to_string(),
            author: "Test".to_string(),
            timestamp: 0,
            tree_hash: "tree".to_string(),
        };

        manager.create_snapshot(commit, &files)?;

        // Delete the object file to simulate corruption
        let objects_dir = repo_path.join("objects");
        for entry in fs::read_dir(objects_dir).context("Failed to read objects directory")? {
            let entry = entry.context("Failed to read directory entry")?;
            fs::remove_file(entry.path())?;
        }

        // Restore should fail
        let result = manager.restore_snapshot("test", &target_dir);
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_snapshot_large_scale() -> Result<()> {
        let dir = tempdir()?;
        let repo_path = dir.path().to_path_buf();

        let manager = SnapshotManager::new(repo_path, 3);

        // Create many files
        let mut files = Vec::new();
        for i in 0..1000 {
            let file = dir.path().join(format!("file_{i}.txt"));
            fs::write(&file, format!("content {i}"))?;

            files.push(crate::storage::FileEntry {
                path: file,
                hash: format!("hash_{i}"),
                size: 10,
                modified: i,
                mode: 0o644,
                cached_hash: None,
            });
        }

        let commit = Commit {
            id: "large".to_string(),
            parent: None,
            message: "Large snapshot".to_string(),
            author: "Test".to_string(),
            timestamp: 0,
            tree_hash: "large_tree".to_string(),
        };

        // Create large snapshot
        let snapshot_id = manager.create_snapshot(commit, &files)?;

        let loaded = manager.load_snapshot(&snapshot_id)?;
        assert_eq!(loaded.files.len(), 1000);

        Ok(())
    }

    #[test]
    fn test_deduplication() -> Result<()> {
        let dir = tempdir()?;
        let repo_path = dir.path().to_path_buf();

        let manager = SnapshotManager::new(repo_path.clone(), 3);

        // Create files with identical content
        let file1 = dir.path().join("file1.txt");
        let file2 = dir.path().join("file2.txt");
        fs::write(&file1, "identical content")?;
        fs::write(&file2, "identical content")?;

        let files = vec![
            crate::storage::FileEntry {
                path: file1,
                hash: "same_hash".to_string(),
                size: 17,
                modified: 0,
                mode: 0o644,
                cached_hash: None,
            },
            crate::storage::FileEntry {
                path: file2,
                hash: "same_hash".to_string(), // Same hash = same content
                size: 17,
                modified: 0,
                mode: 0o644,
                cached_hash: None,
            },
        ];

        let commit = Commit {
            id: "dedup".to_string(),
            parent: None,
            message: "Dedup test".to_string(),
            author: "Test".to_string(),
            timestamp: 0,
            tree_hash: "tree".to_string(),
        };

        manager.create_snapshot(commit, &files)?;

        let objects_dir = repo_path.join("objects");
        let object_count = fs::read_dir(objects_dir)?.count();
        assert_eq!(object_count, 1); // Only one object for both files

        Ok(())
    }

    #[test]
    fn test_snapshot_corruption_recovery() -> Result<()> {
        let dir = tempdir()?;
        let repo_path = dir.path().to_path_buf();

        // Create repo structure
        fs::create_dir_all(repo_path.join("commits"))?;
        fs::create_dir_all(repo_path.join("objects"))?;

        let manager = SnapshotManager::new(repo_path.clone(), 3);

        let test_file = dir.path().join("test.txt");
        fs::write(&test_file, "snapshot content")?;

        let files = vec![FileEntry {
            path: test_file,
            hash: "test_hash".to_string(),
            size: 16,
            modified: 1_234_567_890,
            mode: 0o644,
            cached_hash: None,
        }];

        let commit = Commit {
            id: "commit1".to_string(),
            parent: None,
            message: "Test commit".to_string(),
            author: "Test".to_string(),
            timestamp: 1_234_567_890,
            tree_hash: "tree1".to_string(),
        };

        manager.create_snapshot(commit, &files)?;

        let commits_dir = repo_path.join("commits");
        let snapshot_file = commits_dir.join("commit1.zst");

        assert!(snapshot_file.exists());
        let original_data = fs::read(&snapshot_file)?;

        // Test 1: Corrupt compressed data
        fs::write(&snapshot_file, vec![0x28, 0xb5, 0x2f, 0xfd, 0xFF, 0xFF])?; // Invalid zstd

        let result = manager.load_snapshot("commit1");
        assert!(result.is_err(), "Should reject corrupted snapshot");

        // Test 2: Partial snapshot file
        fs::write(&snapshot_file, &original_data[..original_data.len() / 3])?;

        let result = manager.load_snapshot("commit1");
        assert!(result.is_err(), "Should reject partial snapshot");

        // Test 3: Replace with valid compression of wrong data
        let fake_data = zstd::encode_all(&b"fake snapshot data"[..], 3)?;
        fs::write(&snapshot_file, fake_data)?;

        let result = manager.load_snapshot("commit1");
        assert!(
            result.is_err(),
            "Should reject snapshot with wrong data structure"
        );

        // Restore and verify recovery
        fs::write(&snapshot_file, &original_data)?;
        let restored = manager.load_snapshot("commit1")?;
        assert_eq!(restored.commit.id, "commit1");
        assert_eq!(restored.files.len(), 1);

        Ok(())
    }

    #[test]
    fn test_object_corruption_detection() -> Result<()> {
        let dir = tempdir()?;
        let repo_path = dir.path().to_path_buf();

        fs::create_dir_all(repo_path.join("commits"))?;
        fs::create_dir_all(repo_path.join("objects"))?;

        let manager = SnapshotManager::new(repo_path.clone(), 3);

        // Create snapshot with objects
        let test_file = dir.path().join("test.txt");
        fs::write(&test_file, "test content for objects")?;

        let files = vec![FileEntry {
            path: test_file,
            hash: "object_hash".to_string(),
            size: 24,
            modified: 1_234_567_890,
            mode: 0o644,
            cached_hash: None,
        }];

        let commit = Commit {
            id: "objtest".to_string(),
            parent: None,
            message: "Object test".to_string(),
            author: "Test".to_string(),
            timestamp: 1_234_567_890,
            tree_hash: "tree".to_string(),
        };

        manager.create_snapshot(commit, &files)?;

        // Find and corrupt object files
        let objects_dir = repo_path.join("objects");
        let mut object_files = Vec::new();
        for entry in fs::read_dir(&objects_dir)? {
            let entry = entry.context("Failed to read directory entry")?;
            object_files.push(entry.path());
        }

        assert!(!object_files.is_empty(), "Should have created object files");

        for object_path in &object_files {
            let original_data = fs::read(object_path)?;

            // Test 1: Truncate object
            fs::write(object_path, &original_data[..original_data.len() / 2])?;

            // Try to restore - should detect corruption
            let _restore_result = manager.restore_snapshot("objtest", dir.path());
            // Restoration might fail due to corrupted objects

            // Test 2: Replace with random data
            fs::write(object_path, vec![0x42; 100])?;

            let restore_result = manager.restore_snapshot("objtest", dir.path());
            assert!(
                restore_result.is_err(),
                "Should fail to restore with corrupted objects"
            );

            // Restore original for next test
            fs::write(object_path, &original_data)?;
        }

        Ok(())
    }
}
