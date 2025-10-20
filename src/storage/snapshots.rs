use super::{Commit, FileEntry};
use crate::utils::serialization;
use anyhow::{Context, Result};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use zstd::stream::{decode_all, encode_all};

/// A complete snapshot of repository state at a commit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// The commit metadata
    pub commit: Commit,
    /// All files in the snapshot
    pub files: HashMap<PathBuf, SnapshotFile>,
}

/// Metadata for a file in a snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotFile {
    /// File content hash
    pub hash: String,
    /// Unix file permissions
    pub mode: u32,
    /// Content-addressed storage hash
    pub content_hash: String,
}

/// Manages snapshot storage and compression
pub struct SnapshotManager {
    /// Path to the dotman repository
    repo_path: PathBuf,
    /// Zstandard compression level (1-22)
    compression_level: i32,
    /// Whether to preserve file permissions when restoring
    preserve_permissions: bool,
}

impl SnapshotManager {
    /// Create a new snapshot manager with default settings
    ///
    /// # Arguments
    ///
    /// * `repo_path` - Path to the dotman repository
    /// * `compression_level` - Zstandard compression level (1-22, higher = better compression but slower)
    #[must_use]
    pub const fn new(repo_path: PathBuf, compression_level: i32) -> Self {
        Self::with_permissions(repo_path, compression_level, true)
    }

    /// Create a new snapshot manager with permission preservation setting
    ///
    /// # Arguments
    ///
    /// * `repo_path` - Path to the dotman repository
    /// * `compression_level` - Zstandard compression level (1-22)
    /// * `preserve_permissions` - Whether to preserve file permissions when restoring snapshots
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
    /// If `cleanup_files` is provided, removes files not present in the snapshot
    /// before restoring. This is useful when switching branches to ensure
    /// a clean working directory.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The snapshot cannot be loaded
    /// - Failed to create target directories
    /// - Failed to restore file contents
    /// - Failed to set file permissions
    /// - Failed to remove untracked files during cleanup
    pub fn restore_snapshot(
        &self,
        snapshot_id: &str,
        target_dir: &Path,
        cleanup_files: Option<&[PathBuf]>,
    ) -> Result<()> {
        let snapshot = self.load_snapshot(snapshot_id)?;

        // If cleanup_files is provided, remove files not in snapshot
        if let Some(current_files) = cleanup_files {
            let snapshot_files: std::collections::HashSet<PathBuf> =
                snapshot.files.keys().cloned().collect();

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
        }

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

    /// Store file content in the object store
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to read the source file
    /// - Failed to compress the content
    /// - Failed to write the object file
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

    /// Check if a snapshot exists by its ID
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

/// Removes unreferenced snapshots and objects
pub struct GarbageCollector {
    /// Path to the dotman repository
    repo_path: PathBuf,
}

impl GarbageCollector {
    /// Create a garbage collector for the repository
    ///
    /// # Arguments
    ///
    /// * `repo_path` - Path to the dotman repository
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
