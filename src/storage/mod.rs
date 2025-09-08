pub mod index;
pub mod snapshots;
pub mod stash;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    fn test_hash_file() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "Hello, World!")?;

        let hash = file_ops::hash_file(&file_path)?;
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 32); // xxh3_128 produces 128-bit hash = 32 hex chars

        Ok(())
    }

    #[test]
    fn test_hash_empty_file() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("empty.txt");
        std::fs::write(&file_path, "")?;

        let hash = file_ops::hash_file(&file_path)?;
        assert_eq!(hash, "0");

        Ok(())
    }

    #[test]
    fn test_hash_small_file() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("small.txt");
        std::fs::write(&file_path, "Small content")?;

        let hash = file_ops::hash_file(&file_path)?;
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 32);

        // Hash should be consistent
        let hash2 = file_ops::hash_file(&file_path)?;
        assert_eq!(hash, hash2);

        Ok(())
    }

    #[test]
    fn test_hash_large_file() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("large.txt");

        let large_content = "x".repeat(2_000_000);
        std::fs::write(&file_path, &large_content)?;

        let hash = file_ops::hash_file(&file_path)?;
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 32);

        // Hash should be consistent
        let hash2 = file_ops::hash_file(&file_path)?;
        assert_eq!(hash, hash2);

        Ok(())
    }

    #[test]
    fn test_hash_nonexistent_file() {
        let result = file_ops::hash_file(Path::new("/nonexistent/file.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn test_hash_files_parallel() -> Result<()> {
        let dir = tempdir()?;

        let mut paths = Vec::new();
        for i in 0..5 {
            let file_path = dir.path().join(format!("file{}.txt", i));
            std::fs::write(&file_path, format!("Content {}", i))?;
            paths.push(file_path);
        }

        let results = file_ops::hash_files_parallel(&paths)?;
        assert_eq!(results.len(), 5);

        // All hashes should be different (different content)
        let hashes: Vec<String> = results.iter().map(|(_, h)| h.clone()).collect();
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
        paths.push(valid_path);

        // Add nonexistent file
        paths.push(PathBuf::from("/nonexistent/file.txt"));

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
            modified: 1234567890,
            mode: 0o644,
        };

        assert_eq!(entry.path, PathBuf::from("/test/path.txt"));
        assert_eq!(entry.hash, "test_hash");
        assert_eq!(entry.size, 1024);
        assert_eq!(entry.modified, 1234567890);
        assert_eq!(entry.mode, 0o644);
    }

    #[test]
    fn test_commit_fields() {
        let commit = Commit {
            id: "abc123".to_string(),
            parent: Some("parent123".to_string()),
            message: "Test commit".to_string(),
            author: "Test Author".to_string(),
            timestamp: 1234567890,
            tree_hash: "tree_hash_123".to_string(),
        };

        assert_eq!(commit.id, "abc123");
        assert_eq!(commit.parent, Some("parent123".to_string()));
        assert_eq!(commit.message, "Test commit");
        assert_eq!(commit.author, "Test Author");
        assert_eq!(commit.timestamp, 1234567890);
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
