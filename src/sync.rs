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
            // Paths in snapshots are already relative to HOME (from how they're stored in the index)
            // If the path is absolute (shouldn't happen but handle it), make it relative
            let relative_path = if path.is_absolute() {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/home".to_string());
                path.strip_prefix(&home).with_context(|| {
                    format!(
                        "Cannot export file outside HOME directory: {}\nFile must be under {}",
                        path.display(),
                        home
                    )
                })?
            } else {
                path
            };

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

    /// Export current index state (staged files) to a target directory
    ///
    /// Note: This exports the staging area, not committed files.
    /// Use `export_commit()` to export from a snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to create directories
    /// - Failed to copy files
    pub fn export_current(&self, target_dir: &Path) -> Result<Vec<(PathBuf, PathBuf)>> {
        let mut exported_files = Vec::new();

        // Export staged files
        for (path, entry) in &self.index.staged_entries {
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

            // Read content from storage using the hash
            let content = self
                .snapshot_manager
                .read_object(&entry.hash)
                .with_context(|| {
                    format!(
                        "Failed to read staged object {} for {}",
                        entry.hash,
                        path.display()
                    )
                })?;

            // Write to target
            fs::write(&target_path, content)
                .with_context(|| format!("Failed to write {}", target_path.display()))?;

            // Set file permissions using cross-platform module
            let permissions = crate::utils::permissions::FilePermissions::from_mode(entry.mode);
            permissions.apply_to_path(&target_path, self.preserve_permissions)?;

            exported_files.push((path.clone(), relative_path.to_path_buf()));
        }

        Ok(exported_files)
    }
}

/// Import files from a directory into dotman storage
pub struct Importer<'a> {
    /// Mutable reference to the snapshot manager for object storage
    snapshot_manager: &'a mut SnapshotManager,
    /// Mutable reference to the file index
    index: &'a mut Index,
    /// Whether to preserve file permissions during import
    preserve_permissions: bool,
}

impl<'a> Importer<'a> {
    /// Create a new importer with default settings (permissions preserved)
    #[must_use]
    pub const fn new(snapshot_manager: &'a mut SnapshotManager, index: &'a mut Index) -> Self {
        Self::with_permissions(snapshot_manager, index, true)
    }

    /// Create a new importer with configurable permission preservation
    #[must_use]
    pub const fn with_permissions(
        snapshot_manager: &'a mut SnapshotManager,
        index: &'a mut Index,
        preserve_permissions: bool,
    ) -> Self {
        Self {
            snapshot_manager,
            index,
            preserve_permissions,
        }
    }

    /// Create a `FileEntry` from source file metadata
    fn create_file_entry(
        source_path: &Path,
        target_path: PathBuf,
        hash: String,
        preserve_permissions: bool,
    ) -> Result<crate::storage::FileEntry> {
        let metadata = fs::metadata(source_path)?;
        let mode = if preserve_permissions {
            crate::utils::permissions::FilePermissions::from_path(source_path)?.mode()
        } else {
            0o644
        };

        Ok(crate::storage::FileEntry {
            path: target_path,
            hash,
            size: metadata.len(),
            modified: i64::try_from(
                metadata
                    .modified()?
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_secs(),
            )
            .unwrap_or(i64::MAX),
            mode,
            cached_hash: None,
        })
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
            self.index.stage_entry(file_entry);

            imported_files.push(target_path);
        }

        Ok(imported_files)
    }

    /// Compare a directory with current index and import changes
    ///
    /// Copies files to home directory AND updates staging area.
    ///
    /// # Errors
    ///
    /// Returns an error if failed to walk directory or compare files
    pub fn import_changes(
        &mut self,
        source_dir: &Path,
        home_dir: &Path,
        follow_symlinks: bool,
    ) -> Result<ImportChanges> {
        let mut added = Vec::new();
        let mut modified = Vec::new();
        let mut deleted = Vec::new();
        let mut seen_files = std::collections::HashSet::new();

        for entry in walkdir::WalkDir::new(source_dir)
            .follow_links(follow_symlinks)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            let path = entry.path();

            if entry.file_type().is_dir() || path.components().any(|c| c.as_os_str() == ".git") {
                continue;
            }

            let relative_path = path.strip_prefix(source_dir)?;
            let target_path = home_dir.join(relative_path);
            seen_files.insert(target_path.clone());

            let (hash, _) = crate::storage::file_ops::hash_file(path, None)?;

            let is_modified = self
                .index
                .get_staged_entry(&target_path)
                .is_some_and(|e| e.hash != hash);
            let is_new = self.index.get_staged_entry(&target_path).is_none();

            if is_modified || is_new {
                if is_modified {
                    modified.push(target_path.clone());
                } else {
                    added.push(target_path.clone());
                }

                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(path, &target_path)?;

                if self.preserve_permissions {
                    let permissions = crate::utils::permissions::FilePermissions::from_path(path)?;
                    permissions.apply_to_path(&target_path, true)?;
                }

                let (target_hash, _) = crate::storage::file_ops::hash_file(&target_path, None)?;
                let file_entry = Self::create_file_entry(
                    &target_path,
                    target_path.clone(),
                    target_hash,
                    self.preserve_permissions,
                )?;
                self.index.stage_entry(file_entry);
            }
        }

        let staged_paths: Vec<PathBuf> = self.index.staged_entries.keys().cloned().collect();
        for staged_path in staged_paths {
            if !seen_files.contains(&staged_path) {
                deleted.push(staged_path.clone());
                self.index.staged_entries.remove(&staged_path);
            }
        }

        Ok(ImportChanges {
            added,
            modified,
            deleted,
        })
    }

    /// Stage files from a source directory WITHOUT copying to home directory
    ///
    /// Designed for pull operations - files are stored in objects/ and indexed,
    /// but not written to the working directory (that happens during checkout).
    ///
    /// # Errors
    ///
    /// Returns an error if failed to walk directory, hash files, or store objects
    pub fn stage_from_directory(
        &mut self,
        source_dir: &Path,
        home_dir: &Path,
        follow_symlinks: bool,
    ) -> Result<ImportChanges> {
        let mut added = Vec::new();
        let mut modified = Vec::new();
        let mut deleted = Vec::new();
        let mut seen_files = std::collections::HashSet::new();

        for entry in walkdir::WalkDir::new(source_dir)
            .follow_links(follow_symlinks)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            let path = entry.path();

            if entry.file_type().is_dir() || path.components().any(|c| c.as_os_str() == ".git") {
                continue;
            }

            let relative_path = path.strip_prefix(source_dir)?;
            let target_path = home_dir.join(relative_path);
            seen_files.insert(target_path.clone());

            let (hash, _) = crate::storage::file_ops::hash_file(path, None)?;

            let is_modified = self
                .index
                .get_staged_entry(&target_path)
                .is_some_and(|e| e.hash != hash);
            let is_new = self.index.get_staged_entry(&target_path).is_none();

            if is_modified || is_new {
                if is_modified {
                    modified.push(target_path.clone());
                } else {
                    added.push(target_path.clone());
                }

                self.snapshot_manager.store_object_from_path(path, &hash)?;
                let file_entry =
                    Self::create_file_entry(path, target_path, hash, self.preserve_permissions)?;
                self.index.stage_entry(file_entry);
            }
        }

        let staged_paths: Vec<PathBuf> = self.index.staged_entries.keys().cloned().collect();
        for staged_path in staged_paths {
            if !seen_files.contains(&staged_path) {
                deleted.push(staged_path.clone());
                self.index.staged_entries.remove(&staged_path);
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
