pub mod concurrent_index;
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
