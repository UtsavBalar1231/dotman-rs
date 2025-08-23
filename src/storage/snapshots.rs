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
}

impl SnapshotManager {
    pub fn new(repo_path: PathBuf, compression_level: i32) -> Self {
        Self {
            repo_path,
            compression_level,
        }
    }

    pub fn create_snapshot(&self, commit: Commit, files: &[FileEntry]) -> Result<String> {
        let snapshot_id = commit.id.clone();
        let snapshot_path = self
            .repo_path
            .join("commits")
            .join(format!("{}.zst", &snapshot_id));

        // Create snapshot directory if it doesn't exist
        if let Some(parent) = snapshot_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Get home directory to resolve relative paths
        let home =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;

        // Store file contents in parallel
        let stored_files: Result<Vec<(PathBuf, SnapshotFile)>> = files
            .par_iter()
            .map(|entry| {
                // Convert relative path to absolute for reading file content
                let abs_path = if entry.path.is_relative() {
                    home.join(&entry.path)
                } else {
                    entry.path.clone()
                };
                let content_hash = self.store_file_content(&abs_path, &entry.hash)?;
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

        // Serialize and compress snapshot
        let serialized = serialization::serialize(&snapshot)?;
        let compressed = encode_all(&serialized[..], self.compression_level)?;

        // Write compressed snapshot
        fs::write(&snapshot_path, compressed)?;

        Ok(snapshot_id)
    }

    pub fn load_snapshot(&self, snapshot_id: &str) -> Result<Snapshot> {
        // First try exact match
        let exact_path = self
            .repo_path
            .join("commits")
            .join(format!("{}.zst", snapshot_id));

        let snapshot_path = if exact_path.exists() {
            exact_path
        } else {
            // Try to find by partial ID (suffix match since we show last 8 chars)
            let commits_dir = self.repo_path.join("commits");
            let mut matches = Vec::new();

            if commits_dir.exists() {
                for entry in fs::read_dir(&commits_dir)? {
                    let entry = entry?;
                    let path = entry.path();

                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        // Check if this commit ID ends with the provided partial ID
                        if stem.ends_with(snapshot_id) || stem.starts_with(snapshot_id) {
                            matches.push(path);
                        }
                    }
                }
            }

            match matches.len() {
                0 => anyhow::bail!("No commit found matching: {}", snapshot_id),
                1 => matches.into_iter().next().unwrap(),
                _ => anyhow::bail!(
                    "Ambiguous commit ID '{}' matches {} commits",
                    snapshot_id,
                    matches.len()
                ),
            }
        };

        // Read and decompress snapshot
        let compressed = fs::read(&snapshot_path)
            .with_context(|| format!("Failed to read snapshot: {}", snapshot_id))?;
        let decompressed = decode_all(&compressed[..])?;

        // Deserialize snapshot
        let snapshot: Snapshot = serialization::deserialize(&decompressed)?;
        Ok(snapshot)
    }

    pub fn restore_snapshot(&self, snapshot_id: &str, target_dir: &Path) -> Result<()> {
        let snapshot = self.load_snapshot(snapshot_id)?;

        // Restore files in parallel
        snapshot
            .files
            .par_iter()
            .try_for_each(|(rel_path, snapshot_file)| -> Result<()> {
                let target_path = target_dir.join(rel_path);

                // Create parent directory if needed
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                // Restore file content
                self.restore_file_content(&snapshot_file.content_hash, &target_path)?;

                // Restore file permissions
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let permissions = fs::Permissions::from_mode(snapshot_file.mode);
                    fs::set_permissions(&target_path, permissions)?;
                }

                Ok(())
            })?;

        Ok(())
    }

    fn store_file_content(&self, file_path: &Path, hash: &str) -> Result<String> {
        let objects_dir = self.repo_path.join("objects");
        let object_path = objects_dir.join(format!("{}.zst", hash));

        // Check if object already exists (deduplication)
        if object_path.exists() {
            return Ok(hash.to_string());
        }

        // Create objects directory if needed
        fs::create_dir_all(&objects_dir)?;

        // Read file content
        let content = fs::read(file_path)?;

        // Compress content
        let compressed = encode_all(&content[..], self.compression_level)?;

        // Write compressed object
        fs::write(&object_path, compressed)?;

        Ok(hash.to_string())
    }

    fn restore_file_content(&self, content_hash: &str, target_path: &Path) -> Result<()> {
        let object_path = self
            .repo_path
            .join("objects")
            .join(format!("{}.zst", content_hash));

        // Read and decompress object
        let compressed = fs::read(&object_path)?;
        let content = decode_all(&compressed[..])?;

        // Write restored content
        fs::write(target_path, content)?;

        Ok(())
    }

    pub fn list_snapshots(&self) -> Result<Vec<String>> {
        let commits_dir = self.repo_path.join("commits");

        if !commits_dir.exists() {
            return Ok(Vec::new());
        }

        let mut snapshots = Vec::new();

        for entry in fs::read_dir(commits_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("zst")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                snapshots.push(stem.to_string());
            }
        }

        Ok(snapshots)
    }

    pub fn delete_snapshot(&self, snapshot_id: &str) -> Result<()> {
        let snapshot_path = self
            .repo_path
            .join("commits")
            .join(format!("{}.zst", snapshot_id));

        if snapshot_path.exists() {
            fs::remove_file(snapshot_path)?;
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
    pub fn new(repo_path: PathBuf) -> Self {
        Self { repo_path }
    }

    pub fn collect(&self) -> Result<usize> {
        let commits_dir = self.repo_path.join("commits");
        let objects_dir = self.repo_path.join("objects");

        if !objects_dir.exists() {
            return Ok(0);
        }

        // Collect all referenced objects
        let mut referenced = std::collections::HashSet::new();

        if commits_dir.exists() {
            for entry in fs::read_dir(commits_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.extension().and_then(|s| s.to_str()) == Some("zst") {
                    // Load snapshot and collect referenced objects
                    let compressed = fs::read(&path)?;
                    let decompressed = decode_all(&compressed[..])?;
                    let snapshot: Snapshot = serialization::deserialize(&decompressed)?;

                    for file in snapshot.files.values() {
                        referenced.insert(file.content_hash.clone());
                    }
                }
            }
        }

        // Delete unreferenced objects
        let mut deleted = 0;

        for entry in fs::read_dir(objects_dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                && !referenced.contains(stem)
            {
                fs::remove_file(path)?;
                deleted += 1;
            }
        }

        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_snapshot_manager() -> Result<()> {
        let dir = tempdir()?;
        let repo_path = dir.path().to_path_buf();

        let manager = SnapshotManager::new(repo_path.clone(), 3);

        // Create test files
        let test_file = dir.path().join("test.txt");
        fs::write(&test_file, "Hello, World!")?;

        let files = vec![FileEntry {
            path: test_file.clone(), // Use absolute path
            hash: "abc123".to_string(),
            size: 13,
            modified: 1234567890,
            mode: 0o644,
        }];

        let commit = Commit {
            id: "commit1".to_string(),
            parent: None,
            message: "Initial commit".to_string(),
            author: "Test User".to_string(),
            timestamp: 1234567890,
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
        fs::create_dir_all(&objects_dir)?;

        // Create some object files
        fs::write(objects_dir.join("used.zst"), "used content")?;
        fs::write(objects_dir.join("unused.zst"), "unused content")?;

        // Create a snapshot that references only one object
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
            files: [(
                PathBuf::from("test.txt"),
                SnapshotFile {
                    hash: "used".to_string(),
                    mode: 0o644,
                    content_hash: "used".to_string(),
                },
            )]
            .into_iter()
            .collect(),
        };

        let serialized = serialization::serialize(&snapshot)?;
        let compressed = encode_all(&serialized[..], 3)?;
        fs::write(commits_dir.join("test.zst"), compressed)?;

        // Run garbage collection
        let gc = GarbageCollector::new(repo_path.clone());
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

        let manager = SnapshotManager::new(repo_path.clone(), 3);

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

        let manager = SnapshotManager::new(repo_path.clone(), 3);

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

        let manager = SnapshotManager::new(repo_path.clone(), 3);

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

        // Create a test file
        let test_file = dir.path().join("test.txt");
        fs::write(&test_file, "content")?;

        let files = vec![crate::storage::FileEntry {
            path: test_file.clone(),
            hash: "test_hash".to_string(),
            size: 7,
            modified: 0,
            mode: 0o644,
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
        for entry in fs::read_dir(objects_dir)? {
            let entry = entry?;
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

        let manager = SnapshotManager::new(repo_path.clone(), 3);

        // Create many files
        let mut files = Vec::new();
        for i in 0..1000 {
            let file = dir.path().join(format!("file_{}.txt", i));
            fs::write(&file, format!("content {}", i))?;

            files.push(crate::storage::FileEntry {
                path: file,
                hash: format!("hash_{}", i),
                size: 10,
                modified: i,
                mode: 0o644,
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

        // Load and verify
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
                path: file1.clone(),
                hash: "same_hash".to_string(),
                size: 17,
                modified: 0,
                mode: 0o644,
            },
            crate::storage::FileEntry {
                path: file2.clone(),
                hash: "same_hash".to_string(), // Same hash = same content
                size: 17,
                modified: 0,
                mode: 0o644,
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

        // Check that only one object file was created (deduplication)
        let objects_dir = repo_path.join("objects");
        let object_count = fs::read_dir(objects_dir)?.count();
        assert_eq!(object_count, 1); // Only one object for both files

        Ok(())
    }
}
