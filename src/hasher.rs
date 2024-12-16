use crate::config::config_cache::CacheEntry;
use blake3;
use dashmap::DashMap;
use rayon::prelude::*;
use std::{
    fmt, fs,
    io::{self, Read},
    path::{Path, PathBuf},
    sync,
};

struct HashBox(Box<[u8]>);

impl fmt::LowerHex for HashBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0
            .iter()
            .for_each(|byte| write!(f, "{:02x}", byte).expect("Failed to write to string"));
        Ok(())
    }
}

impl fmt::Display for HashBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0
            .iter()
            .for_each(|byte| write!(f, "{:02x}", byte).expect("Failed to write to string"));
        Ok(())
    }
}

pub fn list_dir_files<P>(p: P) -> Result<Vec<PathBuf>, io::Error>
where
    P: AsRef<Path>,
{
    Ok(walkdir::WalkDir::new(p)
        .into_iter()
        .filter_map(Result::ok) // Skip errors silently
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| !crate::is_git_related(entry.path())) // Ignore `.git`-related files
        .map(|entry| entry.into_path())
        .collect())
}

pub fn get_file_hash(
    path: &PathBuf,
    cache: &sync::Arc<DashMap<PathBuf, CacheEntry>>,
) -> Result<String, io::Error> {
    let mut hasher = blake3::Hasher::new();

    if let Some(cached_entry) = cache.get(path) {
        let metadata = fs::metadata(path)?;
        let modified = metadata.modified()?;

        if cached_entry.modified == modified {
            return Ok(cached_entry.hash.clone());
        }
    }

    let mut file = fs::File::open(path)?;
    let mut buf = [0u8; 4096];

    loop {
        let i = file.read(&mut buf)?;
        hasher.update(&buf[..i]);

        if i == 0 {
            let final_hash =
                HashBox(hasher.finalize().as_bytes().to_vec().into_boxed_slice()).to_string();
            let metadata = fs::metadata(path)?;
            let modified = metadata.modified()?;
            cache.insert(
                path.to_path_buf(),
                CacheEntry {
                    hash: final_hash.clone(),
                    modified,
                },
            );
            return Ok(final_hash);
        }
    }
}

pub fn get_files_hash(
    files: &[PathBuf],
    cache: &sync::Arc<DashMap<PathBuf, CacheEntry>>,
) -> Result<String, io::Error> {
    if files.is_empty() {
        return Ok(String::new());
    }

    let hashes: Vec<String> = files
        .par_iter()
        .map(|file| get_file_hash(file, cache))
        .collect::<Result<Vec<_>, _>>()?;

    let mut final_hasher = blake3::Hasher::new();
    hashes.iter().for_each(|hash| {
        final_hasher.update(hash.as_bytes());
    });

    Ok(HashBox(
        final_hasher
            .finalize()
            .as_bytes()
            .to_vec()
            .into_boxed_slice(),
    )
    .to_string())
}

pub fn get_complete_dir_hash(
    dir_path: &PathBuf,
    cache: &sync::Arc<DashMap<PathBuf, CacheEntry>>,
) -> Result<String, io::Error> {
    get_files_hash(&list_dir_files(dir_path)?, cache)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashmap::DashMap;
    use fs::File;
    use std::io::Write;
    use sync::Arc;
    use tempfile::tempdir;

    #[test]
    fn test_hasher_empty_directory() {
        let temp_dir = tempdir().unwrap();
        let cache = Arc::new(DashMap::new());
        let hash = get_complete_dir_hash(&temp_dir.path().to_path_buf(), &cache).unwrap();
        assert!(
            hash.is_empty(),
            "Empty directory should return an empty hash."
        );
    }

    #[test]
    fn test_hasher_nested_directories() {
        let temp_dir = tempdir().unwrap();
        let nested_dir = temp_dir.path().join("nested");
        fs::create_dir_all(&nested_dir).unwrap();

        let file1 = nested_dir.join("file1.txt");
        let file2 = nested_dir.join("file2.txt");

        File::create(&file1)
            .unwrap()
            .write_all(b"content1")
            .unwrap();
        File::create(&file2)
            .unwrap()
            .write_all(b"content2")
            .unwrap();

        let cache = Arc::new(DashMap::new());
        let hash = get_complete_dir_hash(&nested_dir, &cache).unwrap();

        assert!(
            !hash.is_empty(),
            "Nested directory hash should not be empty."
        );
        assert_eq!(cache.len(), 2, "Cache should contain hashes for all files.");
    }

    #[test]
    fn test_hasher_large_file() {
        let temp_dir = tempdir().unwrap();
        let large_file = temp_dir.path().join("large_file.txt");

        let mut file = File::create(&large_file).unwrap();
        let large_content = vec![b'x'; 10 * 1024 * 1024]; // 10 MB file
        file.write_all(&large_content).unwrap();

        let cache = Arc::new(DashMap::new());
        let hash = get_file_hash(&large_file, &cache).unwrap();

        assert!(!hash.is_empty(), "Hash for large file should not be empty.");
        assert_eq!(
            cache.len(),
            1,
            "Cache should contain the hash for the large file."
        );
    }

    #[test]
    fn test_hasher_cache_reuse() {
        use std::thread::sleep;
        use std::time::Duration;
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("file.txt");

        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"initial content").unwrap();
        // Allow the OS to update the file metadata
        sleep(Duration::from_millis(100));

        let cache = Arc::new(DashMap::new());
        // First hash computation
        let hash1 = get_file_hash(&file_path, &cache).unwrap();
        assert!(!hash1.is_empty());

        // Modify the file after hashing
        file.write_all(b"modified content").unwrap();
        // Allow the OS to update the file metadata
        sleep(Duration::from_millis(100));

        // Recompute hash
        let hash2 = get_file_hash(&file_path, &cache).unwrap();

        assert_ne!(
            hash1, hash2,
            "Hash should be different after file modification."
        );
    }

    #[test]
    fn test_hasher_git_directory_exclusion() {
        let temp_dir = tempdir().unwrap();
        let git_dir = temp_dir.path().join(".git");
        let normal_file = temp_dir.path().join("file.txt");
        let hidden_file = temp_dir.path().join(".hidden.txt");

        fs::create_dir_all(&git_dir).unwrap();
        File::create(&normal_file)
            .unwrap()
            .write_all(b"normal content")
            .unwrap();
        File::create(&hidden_file)
            .unwrap()
            .write_all(b"hidden content")
            .unwrap();

        let cache = Arc::new(DashMap::new());
        let hash = get_complete_dir_hash(&temp_dir.path().to_path_buf(), &cache).unwrap();

        assert!(
            !hash.is_empty(),
            "Hash should not be empty for non-hidden files."
        );

        assert_eq!(cache.len(), 2, "Cache should ignore the .git directory.");
        assert!(cache.contains_key(&normal_file));
        assert!(!cache.contains_key(&git_dir));
        assert!(cache.contains_key(&hidden_file));
    }

    #[test]
    fn test_hasher_parallel_hashing() {
        let temp_dir = tempdir().unwrap();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");

        File::create(&file1)
            .unwrap()
            .write_all(b"file1 content")
            .unwrap();
        File::create(&file2)
            .unwrap()
            .write_all(b"file2 content")
            .unwrap();

        let files = vec![file1.clone(), file2.clone()];
        let cache = Arc::new(DashMap::new());

        let hash = get_files_hash(&files, &cache).unwrap();
        assert!(
            !hash.is_empty(),
            "Hash should not be empty for parallel file hashing."
        );
        assert_eq!(
            cache.len(),
            2,
            "Cache should contain hashes for both files."
        );
    }

    #[test]
    fn test_hasher_non_existent_paths() {
        let invalid_path = PathBuf::from("non_existent_file.txt");

        let cache = Arc::new(DashMap::new());

        let result = get_file_hash(&invalid_path, &cache);
        assert!(
            result.is_err(),
            "Hashing a non-existent file should return an error."
        );
    }

    #[test]
    fn test_hasher_no_read_permission() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("no_permission_file.txt");

        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"restricted content").unwrap();

        // Remove read permission
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&file_path).unwrap().permissions();
            perms.set_mode(0o000);
            fs::set_permissions(&file_path, perms).unwrap();
        }

        let cache = Arc::new(DashMap::new());

        let result = get_file_hash(&file_path, &cache);
        assert!(
            result.is_err(),
            "Hashing a file without read permission should return an error."
        );
    }

    #[test]
    fn test_hasher_get_file_hash() {
        // Create a temporary directory and file for the test
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test_file.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "test content").unwrap();

        // Test without cache
        let cache = Arc::new(DashMap::new()); // Use Arc<DashMap>
        let hash = get_file_hash(&file_path, &cache).unwrap();
        assert!(!hash.is_empty());

        // Test with cache
        let cached_hash = get_file_hash(&file_path, &cache).unwrap();
        assert!(!cached_hash.is_empty());

        // Ensure the cache contains one entry and matches the computed hash
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&file_path).unwrap().value().hash, cached_hash);
    }

    #[test]
    fn test_hasher_get_complete_dir_hash() {
        // Create a temporary directory and file for the test
        let temp_dir = tempdir().unwrap();
        let dir_path = temp_dir.path().join("test_dir");
        fs::create_dir_all(&dir_path).unwrap();
        let file_in_dir = dir_path.join("test_file.txt");
        File::create(&file_in_dir).unwrap();

        // Test without cache
        let cache = Arc::new(DashMap::new()); // Use Arc<DashMap>
        let hash = get_complete_dir_hash(&dir_path, &cache).unwrap();
        assert!(!hash.is_empty());
    }
}
