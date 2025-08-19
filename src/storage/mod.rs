pub mod index;
pub mod snapshots;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: PathBuf,
    pub hash: String,
    pub size: u64,
    pub modified: i64,
    pub mode: u32,
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

pub trait Storage {
    fn init(&self, path: &Path) -> Result<()>;
    fn add_file(&mut self, path: &Path) -> Result<()>;
    fn remove_file(&mut self, path: &Path) -> Result<()>;
    fn get_status(&self) -> Result<Vec<FileStatus>>;
    fn commit(&mut self, message: &str) -> Result<String>;
    fn checkout(&mut self, commit_id: &str) -> Result<()>;
}

#[derive(Debug, Clone)]
pub enum FileStatus {
    Added(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
    Untracked(PathBuf),
}

impl FileStatus {
    pub fn path(&self) -> &Path {
        match self {
            Self::Added(p) | Self::Modified(p) | Self::Deleted(p) | Self::Untracked(p) => p,
        }
    }

    pub fn status_char(&self) -> char {
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
    use super::*;
    use memmap2::MmapOptions;
    use rayon::prelude::*;
    use std::fs::File;
    use xxhash_rust::xxh3::xxh3_128;

    pub fn hash_file(path: &Path) -> Result<String> {
        let file = File::open(path)?;
        let metadata = file.metadata()?;

        if metadata.len() == 0 {
            return Ok(String::from("0"));
        }

        if metadata.len() < 1_048_576 {
            // Small file - read directly
            let content = std::fs::read(path)?;
            let hash = xxh3_128(&content);
            Ok(format!("{:032x}", hash))
        } else {
            // Large file - use memory mapping
            let mmap = unsafe { MmapOptions::new().map(&file)? };
            let hash = xxh3_128(&mmap);
            Ok(format!("{:032x}", hash))
        }
    }

    pub fn hash_files_parallel(paths: &[PathBuf]) -> Result<Vec<(PathBuf, String)>> {
        paths
            .par_iter()
            .map(|path| {
                let hash = hash_file(path)?;
                Ok((path.clone(), hash))
            })
            .collect::<Result<Vec<_>>>()
    }

    pub fn copy_file_fast(src: &Path, dst: &Path) -> Result<()> {
        // Try to create hard link first (fastest)
        if std::fs::hard_link(src, dst).is_ok() {
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
    fn test_hash_file() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "Hello, World!")?;

        let hash = file_ops::hash_file(&file_path)?;
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 32); // xxh3_128 produces 128-bit hash = 32 hex chars

        Ok(())
    }
}
