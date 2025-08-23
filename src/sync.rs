use crate::storage::index::Index;
use crate::storage::snapshots::SnapshotManager;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Export files from dotman storage to a directory
pub struct Exporter<'a> {
    snapshot_manager: &'a SnapshotManager,
    index: &'a Index,
}

impl<'a> Exporter<'a> {
    pub fn new(snapshot_manager: &'a SnapshotManager, index: &'a Index) -> Self {
        Self {
            snapshot_manager,
            index,
        }
    }

    /// Export all files from a commit to a target directory
    pub fn export_commit(
        &self,
        commit_id: &str,
        target_dir: &Path,
    ) -> Result<Vec<(PathBuf, PathBuf)>> {
        // Load the snapshot for this commit
        let snapshot = self
            .snapshot_manager
            .load_snapshot(commit_id)
            .with_context(|| format!("Failed to load snapshot for commit {}", commit_id))?;

        let mut exported_files = Vec::new();

        // Export each file in the snapshot
        for (path, file) in &snapshot.files {
            let relative_path = path
                .strip_prefix(std::env::var("HOME").unwrap_or_else(|_| "/home".to_string()))
                .unwrap_or(path);

            let target_path = target_dir.join(relative_path);

            // Create parent directories
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create directories for {}", target_path.display())
                })?;
            }

            // Read file content from storage
            let content = self
                .snapshot_manager
                .read_object(&file.content_hash)
                .with_context(|| {
                    format!(
                        "Failed to read object {} for {}",
                        file.content_hash,
                        path.display()
                    )
                })?;

            // Write to target
            fs::write(&target_path, content)
                .with_context(|| format!("Failed to write {}", target_path.display()))?;

            // Set file permissions
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let permissions = std::fs::Permissions::from_mode(file.mode);
                fs::set_permissions(&target_path, permissions).with_context(|| {
                    format!("Failed to set permissions for {}", target_path.display())
                })?;
            }

            exported_files.push((path.clone(), relative_path.to_path_buf()));
        }

        Ok(exported_files)
    }

    /// Export current index state to a target directory
    pub fn export_current(&self, target_dir: &Path) -> Result<Vec<(PathBuf, PathBuf)>> {
        let mut exported_files = Vec::new();

        for path in self.index.entries.keys() {
            let relative_path = path
                .strip_prefix(std::env::var("HOME").unwrap_or_else(|_| "/home".to_string()))
                .unwrap_or(path);

            let target_path = target_dir.join(relative_path);

            // Create parent directories
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create directories for {}", target_path.display())
                })?;
            }

            // For current state, we read from the actual file location
            if path.exists() {
                fs::copy(path, &target_path).with_context(|| {
                    format!(
                        "Failed to copy {} to {}",
                        path.display(),
                        target_path.display()
                    )
                })?;

                // Preserve permissions
                #[cfg(unix)]
                {
                    let metadata = fs::metadata(path)?;
                    let permissions = metadata.permissions();
                    fs::set_permissions(&target_path, permissions)?;
                }

                exported_files.push((path.clone(), relative_path.to_path_buf()));
            }
        }

        Ok(exported_files)
    }
}

/// Import files from a directory into dotman storage
pub struct Importer<'a> {
    _snapshot_manager: &'a mut SnapshotManager,
    index: &'a mut Index,
}

impl<'a> Importer<'a> {
    pub fn new(snapshot_manager: &'a mut SnapshotManager, index: &'a mut Index) -> Self {
        Self {
            _snapshot_manager: snapshot_manager,
            index,
        }
    }

    /// Import files from a source directory
    pub fn import_directory(&mut self, source_dir: &Path, home_dir: &Path) -> Result<Vec<PathBuf>> {
        let mut imported_files = Vec::new();

        // Walk the source directory
        for entry in walkdir::WalkDir::new(source_dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Skip the root directory itself and .git directory
            if path == source_dir || path.components().any(|c| c.as_os_str() == ".git") {
                continue;
            }

            // Skip directories (we only track files)
            if entry.file_type().is_dir() {
                continue;
            }

            // Calculate the relative path and target path
            let relative_path = path
                .strip_prefix(source_dir)
                .with_context(|| format!("Failed to strip prefix from {}", path.display()))?;

            let target_path = home_dir.join(relative_path);

            // Create parent directories if needed
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create parent dirs for {}", target_path.display())
                })?;
            }

            // Copy the file to its target location
            fs::copy(path, &target_path).with_context(|| {
                format!(
                    "Failed to copy {} to {}",
                    path.display(),
                    target_path.display()
                )
            })?;

            // Copy permissions
            #[cfg(unix)]
            {
                let metadata = fs::metadata(path)?;
                let permissions = metadata.permissions();
                fs::set_permissions(&target_path, permissions)?;
            }

            // Add to index
            // Add to index
            let metadata = fs::metadata(&target_path)?;
            let file_entry = crate::storage::FileEntry {
                path: target_path.clone(),
                hash: crate::storage::file_ops::hash_file(&target_path)?,
                size: metadata.len(),
                modified: metadata
                    .modified()?
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_secs() as i64,
                mode: {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        metadata.permissions().mode()
                    }
                    #[cfg(not(unix))]
                    {
                        0o644
                    }
                },
            };
            self.index.add_entry(file_entry);

            imported_files.push(target_path);
        }

        Ok(imported_files)
    }

    /// Compare a directory with current index and import changes
    pub fn import_changes(&mut self, source_dir: &Path, home_dir: &Path) -> Result<ImportChanges> {
        let mut added = Vec::new();
        let mut modified = Vec::new();
        let mut deleted = Vec::new();

        // Track files we've seen in the source
        let mut seen_files = std::collections::HashSet::new();

        // Walk the source directory
        for entry in walkdir::WalkDir::new(source_dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Skip directories and .git
            if entry.file_type().is_dir() || path.components().any(|c| c.as_os_str() == ".git") {
                continue;
            }

            let relative_path = path.strip_prefix(source_dir)?;
            let target_path = home_dir.join(relative_path);

            seen_files.insert(target_path.clone());

            // Check if file exists in index
            if let Some(index_entry) = self.index.get_entry(&target_path) {
                // File exists, check if modified
                let content = fs::read(path)?;
                let hash = crate::storage::file_ops::hash_file(path)?;

                if hash != index_entry.hash {
                    // File modified
                    modified.push(target_path.clone());

                    // Update the file
                    if let Some(parent) = target_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&target_path, content)?;

                    #[cfg(unix)]
                    {
                        let metadata = fs::metadata(path)?;
                        let permissions = metadata.permissions();
                        fs::set_permissions(&target_path, permissions)?;
                    }

                    // Update index entry
                    let metadata = fs::metadata(&target_path)?;
                    let file_entry = crate::storage::FileEntry {
                        path: target_path.clone(),
                        hash: crate::storage::file_ops::hash_file(&target_path)?,
                        size: metadata.len(),
                        modified: metadata
                            .modified()?
                            .duration_since(std::time::UNIX_EPOCH)?
                            .as_secs() as i64,
                        mode: {
                            #[cfg(unix)]
                            {
                                use std::os::unix::fs::PermissionsExt;
                                metadata.permissions().mode()
                            }
                            #[cfg(not(unix))]
                            {
                                0o644
                            }
                        },
                    };
                    self.index.add_entry(file_entry);
                }
            } else {
                // New file
                added.push(target_path.clone());

                // Add the file
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(path, &target_path)?;

                #[cfg(unix)]
                {
                    let metadata = fs::metadata(path)?;
                    let permissions = metadata.permissions();
                    fs::set_permissions(&target_path, permissions)?;
                }

                // Add new file to index
                let metadata = fs::metadata(&target_path)?;
                let file_entry = crate::storage::FileEntry {
                    path: target_path.clone(),
                    hash: crate::storage::file_ops::hash_file(&target_path)?,
                    size: metadata.len(),
                    modified: metadata
                        .modified()?
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_secs() as i64,
                    mode: {
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            metadata.permissions().mode()
                        }
                        #[cfg(not(unix))]
                        {
                            0o644
                        }
                    },
                };
                self.index.add_entry(file_entry);
            }
        }

        // Check for deleted files
        // Need to clone the keys to avoid borrowing issues
        let indexed_paths: Vec<PathBuf> = self.index.entries.keys().cloned().collect();
        for indexed_path in &indexed_paths {
            if !seen_files.contains(indexed_path) {
                deleted.push(indexed_path.clone());
                self.index.remove_entry(indexed_path);

                // Optionally remove the file from the filesystem
                // For now, we'll keep it to be safe
            }
        }

        Ok(ImportChanges {
            added,
            modified,
            deleted,
        })
    }
}

/// Result of importing changes from a directory
#[derive(Debug)]
pub struct ImportChanges {
    pub added: Vec<PathBuf>,
    pub modified: Vec<PathBuf>,
    pub deleted: Vec<PathBuf>,
}

impl ImportChanges {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty() && self.deleted.is_empty()
    }

    pub fn summary(&self) -> String {
        let mut parts = Vec::new();

        if !self.added.is_empty() {
            parts.push(format!("{} added", self.added.len()));
        }
        if !self.modified.is_empty() {
            parts.push(format!("{} modified", self.modified.len()));
        }
        if !self.deleted.is_empty() {
            parts.push(format!("{} deleted", self.deleted.len()));
        }

        if parts.is_empty() {
            "No changes".to_string()
        } else {
            parts.join(", ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    #[serial_test::serial]
    fn test_export_current() -> Result<()> {
        let temp = tempdir()?;
        let home = temp.path().join("home");
        fs::create_dir_all(&home)?;

        // Create test files
        let file1 = home.join("test1.txt");
        fs::write(&file1, "content1")?;

        // Create index with proper FileEntry
        let mut index = Index::new();
        let metadata = fs::metadata(&file1)?;
        let file_entry = crate::storage::FileEntry {
            path: file1.clone(),
            hash: crate::storage::file_ops::hash_file(&file1)?,
            size: metadata.len(),
            modified: metadata
                .modified()?
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs() as i64,
            mode: {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    metadata.permissions().mode()
                }
                #[cfg(not(unix))]
                {
                    0o644
                }
            },
        };
        index.add_entry(file_entry);

        // Create snapshot manager with compression level
        let storage_path = temp.path().join(".dotman");
        fs::create_dir_all(&storage_path)?;
        let snapshot_manager = SnapshotManager::new(storage_path, 3);

        // Export
        let export_dir = temp.path().join("export");
        let exporter = Exporter::new(&snapshot_manager, &index);

        unsafe {
            std::env::set_var("HOME", &home);
        }
        let exported = exporter.export_current(&export_dir)?;

        assert_eq!(exported.len(), 1);
        assert!(export_dir.join("test1.txt").exists());

        Ok(())
    }

    #[test]
    fn test_export_commit() -> Result<()> {
        let temp = tempdir()?;
        let storage_path = temp.path().join(".dotman");
        fs::create_dir_all(&storage_path)?;
        fs::create_dir_all(storage_path.join("objects"))?;
        fs::create_dir_all(storage_path.join("commits"))?;

        // Create a test commit with snapshot
        let snapshot_manager = SnapshotManager::new(storage_path.clone(), 3);
        let index = Index::new();

        // Create a test snapshot
        use crate::storage::{
            Commit,
            snapshots::{Snapshot, SnapshotFile},
        };
        use std::collections::HashMap;

        let commit = Commit {
            id: "test_commit_123".to_string(),
            parent: None,
            message: "Test commit".to_string(),
            author: "Test User".to_string(),
            timestamp: 1234567890,
            tree_hash: "test_tree".to_string(),
        };

        let mut files = HashMap::new();
        files.insert(
            PathBuf::from("/home/test/file.txt"),
            SnapshotFile {
                hash: "test_hash".to_string(),
                mode: 0o644,
                content_hash: "content_hash_123".to_string(),
            },
        );

        let snapshot = Snapshot {
            commit: commit.clone(),
            files,
        };

        // Save snapshot
        let snapshot_data = crate::utils::serialization::serialize(&snapshot)?;
        let compressed = zstd::stream::encode_all(&snapshot_data[..], 3)?;
        fs::write(storage_path.join("commits/test_commit_123.zst"), compressed)?;

        // Save fake object content
        let content = b"test content";
        let compressed_content = zstd::stream::encode_all(&content[..], 3)?;
        fs::write(
            storage_path.join("objects/content_hash_123.zst"),
            compressed_content,
        )?;

        // Export the commit
        let export_dir = temp.path().join("export");
        let exporter = Exporter::new(&snapshot_manager, &index);

        unsafe {
            std::env::set_var("HOME", "/home");
        }
        let exported = exporter.export_commit("test_commit_123", &export_dir)?;

        assert_eq!(exported.len(), 1);
        assert!(export_dir.join("test/file.txt").exists());

        Ok(())
    }

    #[test]
    fn test_import_directory() -> Result<()> {
        let temp = tempdir()?;
        let home = temp.path().join("home");
        fs::create_dir_all(&home)?;

        let source_dir = temp.path().join("source");
        fs::create_dir_all(&source_dir)?;

        // Create test files in source
        fs::write(source_dir.join("file1.txt"), "content1")?;
        fs::create_dir_all(source_dir.join("subdir"))?;
        fs::write(source_dir.join("subdir/file2.txt"), "content2")?;

        // Create git directory to skip
        fs::create_dir_all(source_dir.join(".git"))?;
        fs::write(source_dir.join(".git/config"), "git config")?;

        // Import
        let storage_path = temp.path().join(".dotman");
        fs::create_dir_all(&storage_path)?;
        let mut snapshot_manager = SnapshotManager::new(storage_path, 3);
        let mut index = Index::new();
        let mut importer = Importer::new(&mut snapshot_manager, &mut index);

        let imported = importer.import_directory(&source_dir, &home)?;

        // Should have imported 2 files (skipping .git)
        assert_eq!(imported.len(), 2);
        assert!(home.join("file1.txt").exists());
        assert!(home.join("subdir/file2.txt").exists());
        assert!(!home.join(".git/config").exists());

        Ok(())
    }

    #[test]
    fn test_import_changes() -> Result<()> {
        let temp = tempdir()?;
        let home = temp.path().join("home");
        fs::create_dir_all(&home)?;

        // Create initial index with one file
        let existing_file = home.join("existing.txt");
        fs::write(&existing_file, "existing content")?;

        let mut index = Index::new();
        let metadata = fs::metadata(&existing_file)?;
        let file_entry = crate::storage::FileEntry {
            path: existing_file.clone(),
            hash: crate::storage::file_ops::hash_file(&existing_file)?,
            size: metadata.len(),
            modified: metadata
                .modified()?
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs() as i64,
            mode: 0o644,
        };
        index.add_entry(file_entry);

        // Create source directory with changes
        let source_dir = temp.path().join("source");
        fs::create_dir_all(&source_dir)?;

        // Modified file
        fs::write(source_dir.join("existing.txt"), "modified content")?;

        // New file
        fs::write(source_dir.join("new.txt"), "new content")?;

        // Import changes
        let storage_path = temp.path().join(".dotman");
        fs::create_dir_all(&storage_path)?;
        let mut snapshot_manager = SnapshotManager::new(storage_path, 3);
        let mut importer = Importer::new(&mut snapshot_manager, &mut index);

        let changes = importer.import_changes(&source_dir, &home)?;

        assert_eq!(changes.added.len(), 1);
        assert_eq!(changes.modified.len(), 1);
        assert_eq!(changes.deleted.len(), 0);

        assert!(changes.added.contains(&home.join("new.txt")));
        assert!(changes.modified.contains(&home.join("existing.txt")));

        // Verify files were actually updated
        assert_eq!(
            fs::read_to_string(home.join("existing.txt"))?,
            "modified content"
        );
        assert_eq!(fs::read_to_string(home.join("new.txt"))?, "new content");

        Ok(())
    }
}
