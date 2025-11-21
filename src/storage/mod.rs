pub mod concurrent_index;
pub mod index;
/// Snapshot management and compression
pub mod snapshots;
/// Stash storage and retrieval
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

/// Represents a tracked file entry in the index.
///
/// Contains all metadata needed to track a file, including its path, content hash,
/// size, modification time, permissions, and optional cached hash information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// Path to the file relative to repository root
    pub path: PathBuf,
    /// XXH3 hash of file content
    pub hash: String,
    /// File size in bytes
    pub size: u64,
    /// Unix timestamp of last modification
    pub modified: i64,
    /// Unix file permissions mode
    pub mode: u32,
    /// Cached hash information for performance optimization
    pub cached_hash: Option<CachedHash>,
}

/// Represents a commit snapshot in the repository.
///
/// Each commit captures the state of tracked files at a specific point in time,
/// along with metadata about the commit itself (message, author, timestamp, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    /// Unique commit identifier
    pub id: String,
    /// Parent commit ID if any
    pub parent: Option<String>,
    /// Commit message
    pub message: String,
    /// Author name and email
    pub author: String,
    /// Unix timestamp of commit creation
    pub timestamp: i64,
    /// Hash of the file tree at commit time
    pub tree_hash: String,
}

// Storage trait removed - was unused abstraction

/// Represents the status of a file in the working tree.
///
/// This enum categorizes files based on their state relative to the index
/// and the last commit, similar to Git's status tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileStatus {
    /// File newly added to tracking
    Added(PathBuf),
    /// File modified since last commit
    Modified(PathBuf),
    /// File deleted from tracking
    Deleted(PathBuf),
    /// File not tracked by dotman
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

/// File operations with memory mapping and parallel processing.
///
/// Provides file hashing and copying using memory-mapped I/O for files ≥1MB,
/// Rayon for parallelization, and xxHash3 for hashing. Caches hashes with
/// size and mtime to avoid recomputation for unchanged files.
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
            // Empty files get a 32-char zero hash (consistent with xxHash3 format)
            String::from("00000000000000000000000000000000")
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
