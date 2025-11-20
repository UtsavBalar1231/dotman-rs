//! Tracking manifest for managing tracked directories and files.
//!
//! The `TrackingManifest` stores the user's intent about what should be tracked.
//! Unlike the Index which stores staging area state, the manifest persists
//! what directories and files the user wants dotman to monitor.
//!
//! # Architecture
//!
//! The manifest is the source of truth for "what should be tracked". When
//! running `dot status`, dotman:
//! 1. Loads the manifest to know what to scan
//! 2. Scans those directories for all files
//! 3. Compares against the HEAD snapshot to detect changes
//!
//! This enables proper directory tracking - when you `dot add ~/.config/nvim`,
//! all files in that directory are tracked, including files added later.

use crate::utils::serialization;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// File name for the tracking manifest
pub const MANIFEST_FILE: &str = "tracking.bin";

/// Tracking manifest storing user's tracking intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingManifest {
    /// Format version for future compatibility
    pub version: u32,
    /// Directories being tracked (e.g., "~/.config/nvim")
    pub tracked_directories: HashSet<PathBuf>,
    /// Individual files being tracked (not part of a tracked directory)
    pub tracked_files: HashSet<PathBuf>,
}

impl TrackingManifest {
    /// Current manifest format version
    const CURRENT_VERSION: u32 = 1;

    /// Create a new empty tracking manifest
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            tracked_directories: HashSet::new(),
            tracked_files: HashSet::new(),
        }
    }

    /// Add a directory to track
    ///
    /// This records the user's intent to track all files within this directory.
    /// When `dot status` runs, it will scan this directory and compare against
    /// the HEAD snapshot.
    ///
    /// # Arguments
    ///
    /// * `path` - Directory path to track (should be absolute or relative to home)
    pub fn add_directory(&mut self, path: PathBuf) {
        // Normalize the path to avoid duplicates
        let normalized = Self::normalize_path(path);

        // Remove any files that are now covered by this directory
        self.tracked_files.retain(|f| !f.starts_with(&normalized));

        self.tracked_directories.insert(normalized);
    }

    /// Add an individual file to track
    ///
    /// This is used when tracking a specific file that's not part of a
    /// tracked directory.
    ///
    /// # Arguments
    ///
    /// * `path` - File path to track (should be absolute or relative to home)
    pub fn add_file(&mut self, path: PathBuf) {
        let normalized = Self::normalize_path(path);

        // Don't add if already covered by a tracked directory
        if !self.is_covered_by_directory(&normalized) {
            self.tracked_files.insert(normalized);
        }
    }

    /// Remove a directory from tracking
    ///
    /// Returns `true` if the directory was tracked and removed
    pub fn remove_directory(&mut self, path: &Path) -> bool {
        let normalized = Self::normalize_path(path.to_path_buf());
        self.tracked_directories.remove(&normalized)
    }

    /// Remove a file from tracking
    ///
    /// Returns `true` if the file was tracked and removed
    pub fn remove_file(&mut self, path: &Path) -> bool {
        let normalized = Self::normalize_path(path.to_path_buf());
        self.tracked_files.remove(&normalized)
    }

    /// Check if a path is tracked
    ///
    /// Returns `true` if the path is either:
    /// - An explicitly tracked file
    /// - Inside a tracked directory
    #[must_use]
    pub fn is_tracked(&self, path: &Path) -> bool {
        let normalized = Self::normalize_path(path.to_path_buf());

        // Check if explicitly tracked as a file
        if self.tracked_files.contains(&normalized) {
            return true;
        }

        // Check if inside a tracked directory
        self.is_covered_by_directory(&normalized)
    }

    /// Check if a path is covered by any tracked directory
    fn is_covered_by_directory(&self, path: &Path) -> bool {
        self.tracked_directories
            .iter()
            .any(|dir| path.starts_with(dir))
    }

    /// Get all tracked directories
    #[must_use]
    pub const fn get_tracked_directories(&self) -> &HashSet<PathBuf> {
        &self.tracked_directories
    }

    /// Get all tracked individual files
    #[must_use]
    pub const fn get_tracked_files(&self) -> &HashSet<PathBuf> {
        &self.tracked_files
    }

    /// Check if any directories or files are tracked
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tracked_directories.is_empty() && self.tracked_files.is_empty()
    }

    /// Get count of tracked items (directories + files)
    #[must_use]
    pub fn tracked_count(&self) -> usize {
        self.tracked_directories.len() + self.tracked_files.len()
    }

    /// Normalize a path (resolve .. and . components)
    ///
    /// This helps avoid tracking the same path multiple times with different
    /// representations.
    const fn normalize_path(path: PathBuf) -> PathBuf {
        // For now, just return the path as-is
        // TODO: Implement proper path normalization if needed
        path
    }

    /// Save the manifest to disk
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Cannot serialize the manifest
    /// - Cannot write to the file
    pub fn save(&self, repo_path: &Path) -> Result<()> {
        let manifest_path = repo_path.join(MANIFEST_FILE);
        let data =
            serialization::serialize(self).context("Failed to serialize tracking manifest")?;

        std::fs::write(&manifest_path, data)
            .with_context(|| format!("Failed to write manifest to {}", manifest_path.display()))?;

        Ok(())
    }

    /// Load the manifest from disk
    ///
    /// Returns a new empty manifest if the file doesn't exist.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Cannot read the file (except if it doesn't exist)
    /// - Cannot deserialize the manifest
    pub fn load(repo_path: &Path) -> Result<Self> {
        let manifest_path = repo_path.join(MANIFEST_FILE);

        if !manifest_path.exists() {
            // No manifest file yet - return empty manifest
            return Ok(Self::new());
        }

        let data = std::fs::read(&manifest_path)
            .with_context(|| format!("Failed to read manifest from {}", manifest_path.display()))?;

        let manifest: Self =
            serialization::deserialize(&data).context("Failed to deserialize tracking manifest")?;

        // Validate version
        if manifest.version > Self::CURRENT_VERSION {
            anyhow::bail!(
                "Manifest version {} is newer than supported version {}. Please upgrade dotman.",
                manifest.version,
                Self::CURRENT_VERSION
            );
        }

        Ok(manifest)
    }

    /// Clear all tracked directories and files
    pub fn clear(&mut self) {
        self.tracked_directories.clear();
        self.tracked_files.clear();
    }
}

impl Default for TrackingManifest {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_new_manifest() {
        let manifest = TrackingManifest::new();
        assert_eq!(manifest.version, 1);
        assert!(manifest.is_empty());
        assert_eq!(manifest.tracked_count(), 0);
    }

    #[test]
    fn test_add_directory() {
        let mut manifest = TrackingManifest::new();
        manifest.add_directory(PathBuf::from("/home/user/.config"));

        assert!(!manifest.is_empty());
        assert_eq!(manifest.tracked_count(), 1);
        assert!(
            manifest
                .get_tracked_directories()
                .contains(&PathBuf::from("/home/user/.config"))
        );
    }

    #[test]
    fn test_add_file() {
        let mut manifest = TrackingManifest::new();
        manifest.add_file(PathBuf::from("/home/user/.bashrc"));

        assert!(!manifest.is_empty());
        assert_eq!(manifest.tracked_count(), 1);
        assert!(
            manifest
                .get_tracked_files()
                .contains(&PathBuf::from("/home/user/.bashrc"))
        );
    }

    #[test]
    fn test_is_tracked() {
        let mut manifest = TrackingManifest::new();
        manifest.add_directory(PathBuf::from("/home/user/.config"));
        manifest.add_file(PathBuf::from("/home/user/.bashrc"));

        // File inside tracked directory
        assert!(manifest.is_tracked(Path::new("/home/user/.config/nvim/init.lua")));

        // Explicitly tracked file
        assert!(manifest.is_tracked(Path::new("/home/user/.bashrc")));

        // Not tracked
        assert!(!manifest.is_tracked(Path::new("/home/user/Documents/file.txt")));
    }

    #[test]
    fn test_file_covered_by_directory() {
        let mut manifest = TrackingManifest::new();

        // Add a file first
        manifest.add_file(PathBuf::from("/home/user/.config/nvim/init.lua"));
        assert_eq!(manifest.tracked_files.len(), 1);

        // Now add its parent directory - file should be removed
        manifest.add_directory(PathBuf::from("/home/user/.config"));
        assert_eq!(manifest.tracked_files.len(), 0);
        assert_eq!(manifest.tracked_directories.len(), 1);

        // File is still tracked (via directory)
        assert!(manifest.is_tracked(Path::new("/home/user/.config/nvim/init.lua")));
    }

    #[test]
    fn test_remove_directory() {
        let mut manifest = TrackingManifest::new();
        manifest.add_directory(PathBuf::from("/home/user/.config"));

        assert!(manifest.remove_directory(Path::new("/home/user/.config")));
        assert!(!manifest.remove_directory(Path::new("/home/user/.config")));
        assert!(manifest.is_empty());
    }

    #[test]
    fn test_remove_file() {
        let mut manifest = TrackingManifest::new();
        manifest.add_file(PathBuf::from("/home/user/.bashrc"));

        assert!(manifest.remove_file(Path::new("/home/user/.bashrc")));
        assert!(!manifest.remove_file(Path::new("/home/user/.bashrc")));
        assert!(manifest.is_empty());
    }

    #[test]
    fn test_clear() {
        let mut manifest = TrackingManifest::new();
        manifest.add_directory(PathBuf::from("/home/user/.config"));
        manifest.add_file(PathBuf::from("/home/user/.bashrc"));

        manifest.clear();
        assert!(manifest.is_empty());
    }
}
