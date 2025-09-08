use anyhow::Result;
use memmap2::MmapOptions;
use rayon::prelude::*;
use std::fs::File;
use std::path::Path;
use xxhash_rust::xxh3::{Xxh3, xxh3_128};

pub fn hash_bytes(data: &[u8]) -> String {
    let hash = xxh3_128(data);
    format!("{:032x}", hash)
}

pub fn hash_file(path: &Path) -> Result<String> {
    let file = File::open(path)?;
    let metadata = file.metadata()?;

    if metadata.len() == 0 {
        return Ok(hash_bytes(b""));
    }

    if metadata.len() < 1_048_576 {
        let content = std::fs::read(path)?;
        Ok(hash_bytes(&content))
    } else {
        let mmap = unsafe { MmapOptions::new().map(&file)? };
        Ok(hash_bytes(&mmap))
    }
}

pub fn hash_file_streaming(path: &Path) -> Result<String> {
    use std::io::Read;

    let mut file = File::open(path)?;
    let mut hasher = Xxh3::new();
    let mut buffer = vec![0u8; 65536];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let hash = hasher.digest128();
    Ok(format!("{:032x}", hash))
}

pub fn hash_files_parallel(paths: &[&Path]) -> Result<Vec<(String, String)>> {
    paths
        .par_iter()
        .map(|path| {
            let hash = hash_file(path)?;
            Ok((path.to_string_lossy().to_string(), hash))
        })
        .collect()
}

pub fn verify_hash(path: &Path, expected_hash: &str) -> Result<bool> {
    let actual_hash = hash_file(path)?;
    Ok(actual_hash == expected_hash)
}

pub struct Deduplicator {
    seen_hashes: dashmap::DashSet<String>,
}

impl Default for Deduplicator {
    fn default() -> Self {
        Self::new()
    }
}

impl Deduplicator {
    pub fn new() -> Self {
        Self {
            seen_hashes: dashmap::DashSet::new(),
        }
    }

    pub fn is_duplicate(&self, hash: &str) -> bool {
        !self.seen_hashes.insert(hash.to_string())
    }

    pub fn add_hash(&self, hash: String) -> bool {
        self.seen_hashes.insert(hash)
    }

    pub fn deduplicate_files<'a>(&self, paths: &[&'a Path]) -> Result<Vec<&'a Path>> {
        let mut unique_files = Vec::new();

        for path in paths {
            let hash = hash_file(path)?;
            if self.add_hash(hash) {
                unique_files.push(*path);
            }
        }

        Ok(unique_files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_hash_bytes() {
        let data = b"Hello, World!";
        let hash1 = hash_bytes(data);
        let hash2 = hash_bytes(data);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 32);

        let different_data = b"Different data";
        let hash3 = hash_bytes(different_data);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_hash_file() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.txt");
        let content = "Test content for hashing";
        std::fs::write(&file_path, content)?;

        let hash = hash_file(&file_path)?;
        assert_eq!(hash.len(), 32);

        let hash2 = hash_file(&file_path)?;
        assert_eq!(hash, hash2);

        Ok(())
    }

    #[test]
    fn test_hash_file_streaming() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.txt");
        let content = "Test content for streaming hash";
        std::fs::write(&file_path, content)?;

        let hash1 = hash_file(&file_path)?;
        let hash2 = hash_file_streaming(&file_path)?;
        assert_eq!(hash1, hash2);

        Ok(())
    }

    #[test]
    fn test_verify_hash() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "Test content")?;

        let hash = hash_file(&file_path)?;
        assert!(verify_hash(&file_path, &hash)?);
        assert!(!verify_hash(&file_path, "wrong_hash")?);

        Ok(())
    }

    #[test]
    fn test_deduplicator() -> Result<()> {
        let dir = tempdir()?;

        let file1 = dir.path().join("file1.txt");
        let file2 = dir.path().join("file2.txt");
        let file3 = dir.path().join("file3.txt");

        std::fs::write(&file1, "Same content")?;
        std::fs::write(&file2, "Same content")?;
        std::fs::write(&file3, "Different content")?;

        let dedup = Deduplicator::new();
        let paths = vec![file1.as_path(), file2.as_path(), file3.as_path()];
        let unique = dedup.deduplicate_files(&paths)?;

        assert_eq!(unique.len(), 2);

        Ok(())
    }
}
