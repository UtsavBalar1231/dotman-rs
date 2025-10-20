use crate::storage::index::Index;
use crate::storage::snapshots::SnapshotManager;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Export files from dotman storage to a directory
pub struct Exporter<'a> {
    /// Reference to the snapshot manager for reading file objects
    snapshot_manager: &'a SnapshotManager,
    /// Reference to the file index
    index: &'a Index,
    /// Whether to preserve file permissions during export
    preserve_permissions: bool,
}

impl<'a> Exporter<'a> {
    /// Create a new exporter with default settings
    ///
    /// Permissions are preserved by default.
    ///
    /// # Arguments
    ///
    /// * `snapshot_manager` - Reference to the snapshot manager for reading objects
    /// * `index` - Reference to the file index
    ///
    /// # Returns
    ///
    /// A new `Exporter` instance with permission preservation enabled
    #[must_use]
    pub const fn new(snapshot_manager: &'a SnapshotManager, index: &'a Index) -> Self {
        Self::with_permissions(snapshot_manager, index, true)
    }

    /// Create a new exporter with configurable permission preservation
    ///
    /// # Arguments
    ///
    /// * `snapshot_manager` - Reference to the snapshot manager for reading objects
    /// * `index` - Reference to the file index
    /// * `preserve_permissions` - Whether to preserve file permissions during export
    ///
    /// # Returns
    ///
    /// A new `Exporter` instance with the specified permission preservation setting
    #[must_use]
    pub const fn with_permissions(
        snapshot_manager: &'a SnapshotManager,
        index: &'a Index,
        preserve_permissions: bool,
    ) -> Self {
        Self {
            snapshot_manager,
            index,
            preserve_permissions,
        }
    }

    /// Export all files from a commit to a target directory
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to load snapshot
    /// - Failed to create directories
    /// - Failed to read or write files
    pub fn export_commit(
        &self,
        commit_id: &str,
        target_dir: &Path,
    ) -> Result<Vec<(PathBuf, PathBuf)>> {
        let snapshot = self
            .snapshot_manager
            .load_snapshot(commit_id)
            .with_context(|| format!("Failed to load snapshot for commit {commit_id}"))?;

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

            // Set file permissions using cross-platform module
            let permissions = crate::utils::permissions::FilePermissions::from_mode(file.mode);
            permissions.apply_to_path(&target_path, self.preserve_permissions)?;

            exported_files.push((path.clone(), relative_path.to_path_buf()));
        }

        Ok(exported_files)
    }

    /// Export current index state to a target directory
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to create directories
    /// - Failed to copy files
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
    /// Mutable reference to the snapshot manager (unused, kept for future use)
    _snapshot_manager: &'a mut SnapshotManager,
    /// Mutable reference to the file index
    index: &'a mut Index,
    /// Whether to preserve file permissions during import
    preserve_permissions: bool,
}

impl<'a> Importer<'a> {
    /// Create a new importer with default settings
    ///
    /// Permissions are preserved by default.
    ///
    /// # Arguments
    ///
    /// * `snapshot_manager` - Mutable reference to the snapshot manager
    /// * `index` - Mutable reference to the file index
    ///
    /// # Returns
    ///
    /// A new `Importer` instance with permission preservation enabled
    #[must_use]
    pub const fn new(snapshot_manager: &'a mut SnapshotManager, index: &'a mut Index) -> Self {
        Self::with_permissions(snapshot_manager, index, true)
    }

    /// Create a new importer with configurable permission preservation
    ///
    /// # Arguments
    ///
    /// * `snapshot_manager` - Mutable reference to the snapshot manager
    /// * `index` - Mutable reference to the file index
    /// * `preserve_permissions` - Whether to preserve file permissions during import
    ///
    /// # Returns
    ///
    /// A new `Importer` instance with the specified permission preservation setting
    #[must_use]
    pub const fn with_permissions(
        snapshot_manager: &'a mut SnapshotManager,
        index: &'a mut Index,
        preserve_permissions: bool,
    ) -> Self {
        Self {
            _snapshot_manager: snapshot_manager,
            index,
            preserve_permissions,
        }
    }

    /// Import files from a source directory
    ///
    /// # Errors
    ///
    /// Returns an error if failed to walk directory or add files to index
    pub fn import_directory(
        &mut self,
        source_dir: &Path,
        home_dir: &Path,
        follow_symlinks: bool,
    ) -> Result<Vec<PathBuf>> {
        let mut imported_files = Vec::new();

        // Walk the source directory
        for entry in walkdir::WalkDir::new(source_dir)
            .follow_links(follow_symlinks)
            .into_iter()
            .filter_map(std::result::Result::ok)
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

            // Copy permissions using cross-platform module
            if self.preserve_permissions {
                let permissions = crate::utils::permissions::FilePermissions::from_path(path)?;
                permissions.apply_to_path(&target_path, true)?;
            }

            let metadata = fs::metadata(&target_path)?;
            let file_entry = crate::storage::FileEntry {
                path: target_path.clone(),
                hash: {
                    let (hash, _cache) = crate::storage::file_ops::hash_file(&target_path, None)?;
                    hash
                },
                size: metadata.len(),
                modified: i64::try_from(
                    metadata
                        .modified()?
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_secs(),
                )
                .unwrap_or(i64::MAX),
                mode: {
                    let permissions =
                        crate::utils::permissions::FilePermissions::from_path(&target_path)?;
                    permissions.mode()
                },
                cached_hash: None,
            };
            self.index.add_entry(file_entry);

            imported_files.push(target_path);
        }

        Ok(imported_files)
    }

    /// Compare a directory with current index and import changes
    ///
    /// # Errors
    ///
    /// Returns an error if failed to walk directory or compare files
    #[allow(clippy::too_many_lines)]
    pub fn import_changes(
        &mut self,
        source_dir: &Path,
        home_dir: &Path,
        follow_symlinks: bool,
    ) -> Result<ImportChanges> {
        let mut added = Vec::new();
        let mut modified = Vec::new();
        let mut deleted = Vec::new();

        // Track files we've seen in the source
        let mut seen_files = std::collections::HashSet::new();

        // Walk the source directory
        for entry in walkdir::WalkDir::new(source_dir)
            .follow_links(follow_symlinks)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            let path = entry.path();

            // Skip directories and .git
            if entry.file_type().is_dir() || path.components().any(|c| c.as_os_str() == ".git") {
                continue;
            }

            let relative_path = path.strip_prefix(source_dir)?;
            let target_path = home_dir.join(relative_path);

            seen_files.insert(target_path.clone());

            if let Some(index_entry) = self.index.get_entry(&target_path) {
                // File exists, check if modified
                let content = fs::read(path)?;
                let (hash, _cache) = crate::storage::file_ops::hash_file(path, None)?;

                if hash != index_entry.hash {
                    // File modified
                    modified.push(target_path.clone());

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
                        hash: {
                            let (hash, _cache) =
                                crate::storage::file_ops::hash_file(&target_path, None)?;
                            hash
                        },
                        size: metadata.len(),
                        modified: i64::try_from(
                            metadata
                                .modified()?
                                .duration_since(std::time::UNIX_EPOCH)?
                                .as_secs(),
                        )
                        .unwrap_or(i64::MAX),
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
                        cached_hash: None,
                    };
                    self.index.add_entry(file_entry);
                }
            } else {
                // New file
                added.push(target_path.clone());

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

                let metadata = fs::metadata(&target_path)?;
                let file_entry = crate::storage::FileEntry {
                    path: target_path.clone(),
                    hash: {
                        let (hash, _cache) =
                            crate::storage::file_ops::hash_file(&target_path, None)?;
                        hash
                    },
                    size: metadata.len(),
                    modified: i64::try_from(
                        metadata
                            .modified()?
                            .duration_since(std::time::UNIX_EPOCH)?
                            .as_secs(),
                    )
                    .unwrap_or(i64::MAX),
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
                    cached_hash: None,
                };
                self.index.add_entry(file_entry);
            }
        }

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
    /// Files that were added during the import
    pub added: Vec<PathBuf>,
    /// Files that were modified during the import
    pub modified: Vec<PathBuf>,
    /// Files that were deleted during the import
    pub deleted: Vec<PathBuf>,
}

impl ImportChanges {
    /// Check if the import contains no changes
    ///
    /// # Returns
    ///
    /// `true` if there are no added, modified, or deleted files, `false` otherwise
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty() && self.deleted.is_empty()
    }

    /// Generate a human-readable summary of the changes
    ///
    /// # Returns
    ///
    /// A comma-separated string describing the number of added, modified, and deleted files.
    /// Returns "No changes" if the import is empty.
    #[must_use]
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
