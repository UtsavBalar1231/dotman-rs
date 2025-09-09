pub mod index;
pub mod snapshots;
pub mod stash;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Cached hash information for a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedHash {
    /// The computed hash value
    pub hash: String,
    /// File size when hash was computed
    pub size_at_hash: u64,
    /// Modification time when hash was computed
    pub mtime_at_hash: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: PathBuf,
    pub hash: String,
    pub size: u64,
    pub modified: i64,
    pub mode: u32,
    /// Cached hash information for performance optimization
    pub cached_hash: Option<CachedHash>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub id: String,
    pub parent: Option<String>,
    pub message: String,
    pub author: String,
    pub timestamp: i64,
    pub tree_hash: String,
}

// Storage trait removed - was unused abstraction

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileStatus {
    Added(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
    Untracked(PathBuf),
}

impl FileStatus {
    /// Returns the path associated with the file status.
    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            Self::Added(p) | Self::Modified(p) | Self::Deleted(p) | Self::Untracked(p) => p,
        }
    }

    /// Returns a single-character representation of the file status.
    #[must_use]
    pub const fn status_char(&self) -> char {
        match self {
            Self::Added(_) => 'A',
            Self::Modified(_) => 'M',
            Self::Deleted(_) => 'D',
            Self::Untracked(_) => '?',
        }
    }
}

// Fast file operations using memory mapping and parallel processing
pub mod file_ops {
    use super::{CachedHash, Path, PathBuf, Result};
    use anyhow::Context;
    use memmap2::MmapOptions;
    use rayon::prelude::*;
    use std::fs::File;
    use xxhash_rust::xxh3::xxh3_128;

    /// Computes the XXH3 128-bit hash of raw bytes.
    #[must_use]
    pub fn hash_bytes(data: &[u8]) -> String {
        let hash = xxh3_128(data);
        format!("{hash:032x}")
    }

    /// Computes the hash of a file with caching support.
    ///
    /// If the cached hash is valid (file hasn't changed), returns the cached value.
    /// Otherwise, computes a new hash and returns it along with updated cache metadata.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or hashed.
    pub fn hash_file(path: &Path, cached: Option<&CachedHash>) -> Result<(String, CachedHash)> {
        hash_file_with_threshold(path, cached, 1_048_576) // Default 1MB
    }

    /// Computes the hash of a file with configurable mmap threshold.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or hashed.
    pub fn hash_file_with_threshold(
        path: &Path,
        cached: Option<&CachedHash>,
        mmap_threshold: usize,
    ) -> Result<(String, CachedHash)> {
        let metadata = std::fs::metadata(path)
            .with_context(|| format!("Failed to get metadata for: {}", path.display()))?;

        let size = metadata.len();
        let modified = i64::try_from(
            metadata
                .modified()
                .context("Failed to get file modification time")?
                .duration_since(std::time::UNIX_EPOCH)
                .context("Invalid file modification time")?
                .as_secs(),
        )
        .context("File modification time too large")?;

        // Check if we can use the cached hash
        if let Some(cached_hash) = cached
            && cached_hash.size_at_hash == size
            && cached_hash.mtime_at_hash == modified
        {
            // Cache hit - file hasn't changed
            return Ok((cached_hash.hash.clone(), cached_hash.clone()));
        }

        // Cache miss - compute new hash
        let hash = if size == 0 {
            String::from("0")
        } else if size < mmap_threshold as u64 {
            // Small file - read directly
            let content = std::fs::read(path)?;
            let hash = xxh3_128(&content);
            format!("{hash:032x}")
        } else {
            // Large file - use memory mapping
            let file = File::open(path)?;
            let mmap = unsafe { MmapOptions::new().map(&file)? };
            let hash = xxh3_128(&mmap);
            format!("{hash:032x}")
        };

        let new_cache = CachedHash {
            hash: hash.clone(),
            size_at_hash: size,
            mtime_at_hash: modified,
        };

        Ok((hash, new_cache))
    }

    /// Hash multiple files in parallel with caching support
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Any file cannot be read or hashed
    pub fn hash_files_parallel(
        paths: &[(PathBuf, Option<CachedHash>)],
    ) -> Result<Vec<(PathBuf, String, CachedHash)>> {
        paths
            .par_iter()
            .map(|(path, cached)| {
                let (hash, cache) = hash_file(path, cached.as_ref())?;
                Ok((path.clone(), hash, cache))
            })
            .collect::<Result<Vec<_>>>()
    }

    /// Fast file copy using hard links when possible
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be copied
    pub fn copy_file_fast(src: &Path, dst: &Path) -> Result<()> {
        copy_file_with_options(src, dst, true)
    }

    /// Copy file with configurable hard link usage
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be copied
    pub fn copy_file_with_options(src: &Path, dst: &Path, use_hard_links: bool) -> Result<()> {
        // Try to create hard link first if enabled (fastest)
        if use_hard_links && std::fs::hard_link(src, dst).is_ok() {
            return Ok(());
        }

        // Fall back to copy
        std::fs::copy(src, dst)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_file_status() {
        let status = FileStatus::Added(PathBuf::from("test.txt"));
        assert_eq!(status.status_char(), 'A');
        assert_eq!(status.path(), Path::new("test.txt"));
    }

    #[test]
    fn test_all_file_status_variants() {
        let added = FileStatus::Added(PathBuf::from("added.txt"));
        assert_eq!(added.status_char(), 'A');
        assert_eq!(added.path(), Path::new("added.txt"));

        let modified = FileStatus::Modified(PathBuf::from("modified.txt"));
        assert_eq!(modified.status_char(), 'M');
        assert_eq!(modified.path(), Path::new("modified.txt"));

        let deleted = FileStatus::Deleted(PathBuf::from("deleted.txt"));
        assert_eq!(deleted.status_char(), 'D');
        assert_eq!(deleted.path(), Path::new("deleted.txt"));

        let untracked = FileStatus::Untracked(PathBuf::from("untracked.txt"));
        assert_eq!(untracked.status_char(), '?');
        assert_eq!(untracked.path(), Path::new("untracked.txt"));
    }

    #[test]
    fn test_hash_file_with_cache() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "Hello, World!")?;

        // First call - no cache
        let (hash1, cache1) = file_ops::hash_file(&file_path, None)?;
        assert!(!hash1.is_empty());
        assert_eq!(hash1.len(), 32); // xxh3_128 produces 128-bit hash = 32 hex chars

        // Second call - with cache
        let (hash2, _cache2) = file_ops::hash_file(&file_path, Some(&cache1))?;
        assert_eq!(hash2, hash1); // Should return same hash

        Ok(())
    }

    #[test]
    fn test_hash_empty_file() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("empty.txt");
        std::fs::write(&file_path, "")?;

        let (hash, _cache) = file_ops::hash_file(&file_path, None)?;
        assert_eq!(hash, "0");

        Ok(())
    }

    #[test]
    fn test_hash_small_file() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("small.txt");
        std::fs::write(&file_path, "Small content")?;

        let (hash, _cache) = file_ops::hash_file(&file_path, None)?;
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 32);

        // Hash should be consistent
        let (hash2, _cache2) = file_ops::hash_file(&file_path, None)?;
        assert_eq!(hash, hash2);

        Ok(())
    }

    #[test]
    fn test_hash_large_file() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("large.txt");

        let large_content = "x".repeat(2_000_000);
        std::fs::write(&file_path, &large_content)?;

        let (hash, _cache) = file_ops::hash_file(&file_path, None)?;
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 32);

        // Hash should be consistent
        let (hash2, _cache2) = file_ops::hash_file(&file_path, None)?;
        assert_eq!(hash, hash2);

        Ok(())
    }

    #[test]
    fn test_hash_nonexistent_file() {
        let result = file_ops::hash_file(Path::new("/nonexistent/file.txt"), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_hash_files_parallel() -> Result<()> {
        let dir = tempdir()?;

        let mut paths = Vec::new();
        for i in 0..5 {
            let file_path = dir.path().join(format!("file{i}.txt"));
            std::fs::write(&file_path, format!("Content {i}"))?;
            paths.push((file_path, None));
        }

        let results = file_ops::hash_files_parallel(&paths)?;
        assert_eq!(results.len(), 5);

        // All hashes should be different (different content)
        let hashes: Vec<String> = results.iter().map(|(_, h, _)| h.clone()).collect();
        for i in 0..hashes.len() {
            for j in i + 1..hashes.len() {
                assert_ne!(hashes[i], hashes[j]);
            }
        }

        Ok(())
    }

    #[test]
    fn test_hash_files_parallel_empty() -> Result<()> {
        let results = file_ops::hash_files_parallel(&[])?;
        assert!(results.is_empty());

        Ok(())
    }

    #[test]
    fn test_hash_files_parallel_with_error() -> Result<()> {
        let dir = tempdir()?;

        let mut paths = Vec::new();
        let valid_path = dir.path().join("valid.txt");
        std::fs::write(&valid_path, "Valid content")?;
        paths.push((valid_path, None));

        // Add nonexistent file
        paths.push((PathBuf::from("/nonexistent/file.txt"), None));

        let result = file_ops::hash_files_parallel(&paths);
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_copy_file_fast() -> Result<()> {
        let dir = tempdir()?;
        let src = dir.path().join("source.txt");
        let dst = dir.path().join("dest.txt");

        std::fs::write(&src, "Test content")?;

        file_ops::copy_file_fast(&src, &dst)?;

        assert!(dst.exists());
        let content = std::fs::read_to_string(&dst)?;
        assert_eq!(content, "Test content");

        Ok(())
    }

    #[test]
    fn test_copy_file_fast_overwrite() -> Result<()> {
        let dir = tempdir()?;
        let src = dir.path().join("source.txt");
        let dst = dir.path().join("dest.txt");

        std::fs::write(&src, "New content")?;
        std::fs::write(&dst, "Old content")?;

        // Remove dst first to test copy
        std::fs::remove_file(&dst)?;

        file_ops::copy_file_fast(&src, &dst)?;

        let content = std::fs::read_to_string(&dst)?;
        assert_eq!(content, "New content");

        Ok(())
    }

    #[test]
    fn test_copy_file_fast_nonexistent_source() {
        let dir = tempdir().unwrap();
        let src = Path::new("/nonexistent/source.txt");
        let dst = dir.path().join("dest.txt");

        let result = file_ops::copy_file_fast(src, &dst);
        assert!(result.is_err());
    }

    #[test]
    fn test_file_entry_fields() {
        let entry = FileEntry {
            path: PathBuf::from("/test/path.txt"),
            hash: "test_hash".to_string(),
            size: 1024,
            modified: 1_234_567_890,
            mode: 0o644,
            cached_hash: None,
        };

        assert_eq!(entry.path, PathBuf::from("/test/path.txt"));
        assert_eq!(entry.hash, "test_hash");
        assert_eq!(entry.size, 1024);
        assert_eq!(entry.modified, 1_234_567_890);
        assert_eq!(entry.mode, 0o644);
        assert!(entry.cached_hash.is_none());
    }

    #[test]
    fn test_file_entry_with_cache() {
        let cached = CachedHash {
            hash: "cached_hash".to_string(),
            size_at_hash: 2048,
            mtime_at_hash: 1_234_567_890,
        };

        let entry = FileEntry {
            path: PathBuf::from("/test/cached.txt"),
            hash: "test_hash".to_string(),
            size: 2048,
            modified: 1_234_567_890,
            mode: 0o644,
            cached_hash: Some(cached),
        };

        assert!(entry.cached_hash.is_some());
        let cache = entry.cached_hash.unwrap();
        assert_eq!(cache.hash, "cached_hash");
        assert_eq!(cache.size_at_hash, 2048);
        assert_eq!(cache.mtime_at_hash, 1_234_567_890);
    }

    #[test]
    fn test_commit_fields() {
        let commit = Commit {
            id: "abc123".to_string(),
            parent: Some("parent123".to_string()),
            message: "Test commit".to_string(),
            author: "Test Author".to_string(),
            timestamp: 1_234_567_890,
            tree_hash: "tree_hash_123".to_string(),
        };

        assert_eq!(commit.id, "abc123");
        assert_eq!(commit.parent, Some("parent123".to_string()));
        assert_eq!(commit.message, "Test commit");
        assert_eq!(commit.author, "Test Author");
        assert_eq!(commit.timestamp, 1_234_567_890);
        assert_eq!(commit.tree_hash, "tree_hash_123");
    }

    #[test]
    fn test_commit_no_parent() {
        let commit = Commit {
            id: "root".to_string(),
            parent: None,
            message: "Initial commit".to_string(),
            author: "Author".to_string(),
            timestamp: 0,
            tree_hash: "tree".to_string(),
        };

        assert_eq!(commit.parent, None);
    }
}
