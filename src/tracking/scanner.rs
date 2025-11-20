//! Directory scanner for discovering files in tracked locations.
//!
//! The `DirectoryScanner` walks through tracked directories and collects
//! all files that should be managed by dotman. It respects ignore patterns
//! and provides parallel scanning for performance.

use crate::tracking::manifest::TrackingManifest;
use crate::utils::should_ignore;
use anyhow::{Context, Result};
use rayon::prelude::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Scanner for enumerating files in tracked directories
pub struct DirectoryScanner {
    /// The tracking manifest defining what to scan
    manifest: TrackingManifest,
    /// Patterns to ignore during scanning
    ignore_patterns: Vec<String>,
    /// Whether to follow symbolic links
    follow_symlinks: bool,
    /// Home directory for path resolution
    home_dir: PathBuf,
}

impl DirectoryScanner {
    /// Create a new directory scanner
    ///
    /// # Arguments
    ///
    /// * `manifest` - Tracking manifest defining what to scan
    /// * `ignore_patterns` - Patterns to exclude (e.g., "*.log", ".git")
    /// * `follow_symlinks` - Whether to follow symbolic links
    /// * `home_dir` - Home directory for resolving relative paths
    #[must_use]
    pub const fn new(
        manifest: TrackingManifest,
        ignore_patterns: Vec<String>,
        follow_symlinks: bool,
        home_dir: PathBuf,
    ) -> Self {
        Self {
            manifest,
            ignore_patterns,
            follow_symlinks,
            home_dir,
        }
    }

    /// Scan all tracked locations and return absolute paths to all files
    ///
    /// This walks through all tracked directories and includes all explicitly
    /// tracked files.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Cannot access a tracked directory
    /// - File traversal fails
    pub fn scan_all_files(&self) -> Result<Vec<PathBuf>> {
        let mut all_files = HashSet::new();

        // Scan tracked directories in parallel
        let dir_files: Result<Vec<Vec<PathBuf>>> = self
            .manifest
            .get_tracked_directories()
            .par_iter()
            .map(|dir| self.scan_directory(dir))
            .collect();

        // Collect all files from directories
        for files in dir_files? {
            all_files.extend(files);
        }

        // Add explicitly tracked individual files
        for file in self.manifest.get_tracked_files() {
            let abs_path = self.resolve_path(file);
            if abs_path.exists() && abs_path.is_file() {
                all_files.insert(abs_path);
            }
        }

        Ok(all_files.into_iter().collect())
    }

    /// Scan a single directory and return all file paths
    ///
    /// # Arguments
    ///
    /// * `dir` - Directory to scan
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Cannot access the directory
    /// - Directory traversal fails
    fn scan_directory(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        let abs_dir = self.resolve_path(dir);

        if !abs_dir.exists() {
            // Directory doesn't exist - return empty (not an error)
            return Ok(Vec::new());
        }

        if !abs_dir.is_dir() {
            anyhow::bail!("Tracked path is not a directory: {}", abs_dir.display());
        }

        let mut files = Vec::new();

        for entry in WalkDir::new(&abs_dir)
            .follow_links(self.follow_symlinks)
            .into_iter()
            .filter_entry(|e| !self.should_skip_entry(e))
        {
            let entry = entry.with_context(|| {
                format!("Failed to read directory entry in {}", abs_dir.display())
            })?;

            if entry.file_type().is_file() {
                files.push(entry.path().to_path_buf());
            }
        }

        Ok(files)
    }

    /// Check if a directory entry should be skipped
    fn should_skip_entry(&self, entry: &walkdir::DirEntry) -> bool {
        let path = entry.path();

        // Skip dotman repository itself
        if path.ends_with(".dotman") {
            return true;
        }

        // Check against ignore patterns
        let relative_path = path.strip_prefix(&self.home_dir).unwrap_or(path);
        should_ignore(relative_path, &self.ignore_patterns)
    }

    /// Resolve a path to absolute form
    ///
    /// If the path is relative, it's resolved relative to the home directory.
    fn resolve_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.home_dir.join(path)
        }
    }

    /// Get the tracking manifest
    #[must_use]
    pub const fn manifest(&self) -> &TrackingManifest {
        &self.manifest
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_structure(temp_dir: &Path) -> Result<()> {
        // Create directory structure:
        // temp/
        //   config/
        //     nvim/
        //       init.lua
        //       lazy-lock.json
        //     kitty/
        //       kitty.conf
        //   .bashrc
        //   .gitignore

        let config = temp_dir.join("config");
        let nvim = config.join("nvim");
        let kitty = config.join("kitty");

        fs::create_dir_all(&nvim)?;
        fs::create_dir_all(&kitty)?;

        fs::write(nvim.join("init.lua"), "-- init")?;
        fs::write(nvim.join("lazy-lock.json"), "{}")?;
        fs::write(kitty.join("kitty.conf"), "# config")?;
        fs::write(temp_dir.join(".bashrc"), "# bashrc")?;
        fs::write(temp_dir.join(".gitignore"), "*.log")?;

        Ok(())
    }

    #[test]
    fn test_scan_directory() -> Result<()> {
        let temp_dir = TempDir::new()?;
        create_test_structure(temp_dir.path())?;

        let mut manifest = TrackingManifest::new();
        manifest.add_directory(temp_dir.path().join("config/nvim"));

        let scanner =
            DirectoryScanner::new(manifest, Vec::new(), false, temp_dir.path().to_path_buf());

        let files = scanner.scan_all_files()?;

        assert_eq!(files.len(), 2); // init.lua and lazy-lock.json
        assert!(files.iter().any(|f| f.ends_with("init.lua")));
        assert!(files.iter().any(|f| f.ends_with("lazy-lock.json")));

        Ok(())
    }

    #[test]
    fn test_scan_multiple_directories() -> Result<()> {
        let temp_dir = TempDir::new()?;
        create_test_structure(temp_dir.path())?;

        let mut manifest = TrackingManifest::new();
        manifest.add_directory(temp_dir.path().join("config/nvim"));
        manifest.add_directory(temp_dir.path().join("config/kitty"));

        let scanner =
            DirectoryScanner::new(manifest, Vec::new(), false, temp_dir.path().to_path_buf());

        let files = scanner.scan_all_files()?;

        assert_eq!(files.len(), 3); // nvim (2) + kitty (1)
        assert!(files.iter().any(|f| f.ends_with("init.lua")));
        assert!(files.iter().any(|f| f.ends_with("lazy-lock.json")));
        assert!(files.iter().any(|f| f.ends_with("kitty.conf")));

        Ok(())
    }

    #[test]
    fn test_scan_with_individual_files() -> Result<()> {
        let temp_dir = TempDir::new()?;
        create_test_structure(temp_dir.path())?;

        let mut manifest = TrackingManifest::new();
        manifest.add_directory(temp_dir.path().join("config/nvim"));
        manifest.add_file(temp_dir.path().join(".bashrc"));

        let scanner =
            DirectoryScanner::new(manifest, Vec::new(), false, temp_dir.path().to_path_buf());

        let files = scanner.scan_all_files()?;

        assert_eq!(files.len(), 3); // nvim (2) + .bashrc (1)
        assert!(files.iter().any(|f| f.ends_with("init.lua")));
        assert!(files.iter().any(|f| f.ends_with("lazy-lock.json")));
        assert!(files.iter().any(|f| f.ends_with(".bashrc")));

        Ok(())
    }

    #[test]
    fn test_scan_with_ignore_patterns() -> Result<()> {
        let temp_dir = TempDir::new()?;
        create_test_structure(temp_dir.path())?;

        // Add a .log file that should be ignored
        fs::write(temp_dir.path().join("config/nvim/debug.log"), "logs")?;

        let mut manifest = TrackingManifest::new();
        manifest.add_directory(temp_dir.path().join("config/nvim"));

        let scanner = DirectoryScanner::new(
            manifest,
            vec!["*.log".to_string()],
            false,
            temp_dir.path().to_path_buf(),
        );

        let files = scanner.scan_all_files()?;

        // Should not include debug.log
        assert_eq!(files.len(), 2);
        assert!(!files.iter().any(|f| f.ends_with("debug.log")));

        Ok(())
    }

    #[test]
    fn test_scan_nonexistent_directory() -> Result<()> {
        let temp_dir = TempDir::new()?;

        let mut manifest = TrackingManifest::new();
        manifest.add_directory(temp_dir.path().join("nonexistent"));

        let scanner =
            DirectoryScanner::new(manifest, Vec::new(), false, temp_dir.path().to_path_buf());

        let files = scanner.scan_all_files()?;
        assert_eq!(files.len(), 0); // No error, just empty

        Ok(())
    }
}
