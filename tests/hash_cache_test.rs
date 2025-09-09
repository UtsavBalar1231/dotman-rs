use anyhow::Result;
use dotman::storage::{CachedHash, FileEntry, file_ops};
use std::path::PathBuf;
use tempfile::tempdir;

mod common;
use common::set_file_mtime;

#[test]
fn test_cached_hash_basic() -> Result<()> {
    let dir = tempdir()?;
    let file_path = dir.path().join("test.txt");
    std::fs::write(&file_path, "initial content")?;

    // First hash computation - no cache
    let (hash1, cache1) = file_ops::hash_file(&file_path, None)?;
    assert!(!hash1.is_empty());
    assert_eq!(cache1.hash, hash1);

    // Get file metadata for verification
    let metadata = std::fs::metadata(&file_path)?;
    assert_eq!(cache1.size_at_hash, metadata.len());

    // Second call with valid cache - should return cached value
    let (hash2, cache2) = file_ops::hash_file(&file_path, Some(&cache1))?;
    assert_eq!(hash2, hash1, "Hash should be the same from cache");
    assert_eq!(cache2.hash, cache1.hash);
    assert_eq!(cache2.size_at_hash, cache1.size_at_hash);
    assert_eq!(cache2.mtime_at_hash, cache1.mtime_at_hash);

    Ok(())
}

#[test]
fn test_cached_hash_invalidation_on_content_change() -> Result<()> {
    let dir = tempdir()?;
    let file_path = dir.path().join("changing.txt");
    std::fs::write(&file_path, "original")?;

    // Initial hash
    let (hash1, cache1) = file_ops::hash_file(&file_path, None)?;

    // Modify file content
    std::fs::write(&file_path, "modified content with different size")?;

    // Ensure different mtime by explicitly setting it
    set_file_mtime(&file_path, 1)?;

    // Hash with stale cache - should recompute
    let (hash2, cache2) = file_ops::hash_file(&file_path, Some(&cache1))?;
    assert_ne!(hash2, hash1, "Hash should differ after content change");
    assert_ne!(
        cache2.size_at_hash, cache1.size_at_hash,
        "Size should differ"
    );
    assert_ne!(
        cache2.mtime_at_hash, cache1.mtime_at_hash,
        "Modification time should differ"
    );

    Ok(())
}

#[test]
fn test_cached_hash_invalidation_same_size() -> Result<()> {
    let dir = tempdir()?;
    let file_path = dir.path().join("same_size.txt");
    std::fs::write(&file_path, "12345")?;

    // Initial hash
    let (hash1, cache1) = file_ops::hash_file(&file_path, None)?;

    // Modify file with same size but different content
    std::fs::write(&file_path, "abcde")?;

    // Ensure different mtime by explicitly setting it
    set_file_mtime(&file_path, 1)?;

    // Hash with stale cache - should recompute based on mtime
    let (hash2, cache2) = file_ops::hash_file(&file_path, Some(&cache1))?;
    assert_ne!(
        hash2, hash1,
        "Hash should differ even with same size due to different content"
    );
    assert_eq!(
        cache2.size_at_hash, cache1.size_at_hash,
        "Size should be the same"
    );
    assert_ne!(
        cache2.mtime_at_hash, cache1.mtime_at_hash,
        "Modification time should differ"
    );

    Ok(())
}

#[test]
#[allow(clippy::unnecessary_wraps)]
fn test_file_entry_with_cached_hash() -> Result<()> {
    let cached = CachedHash {
        hash: "abc123".to_string(),
        size_at_hash: 1024,
        mtime_at_hash: 1_234_567_890,
    };

    let entry = FileEntry {
        path: PathBuf::from("test.txt"),
        hash: "abc123".to_string(),
        size: 1024,
        modified: 1_234_567_890,
        mode: 0o644,
        cached_hash: Some(cached),
    };

    // Verify cached hash is stored
    assert!(entry.cached_hash.is_some());
    let stored_cache = entry.cached_hash.unwrap();
    assert_eq!(stored_cache.hash, "abc123");
    assert_eq!(stored_cache.size_at_hash, 1024);
    assert_eq!(stored_cache.mtime_at_hash, 1_234_567_890);

    Ok(())
}

#[test]
fn test_parallel_hashing_with_cache() -> Result<()> {
    let dir = tempdir()?;
    let mut files_with_cache = Vec::new();

    // Create test files
    for i in 0..5 {
        let file_path = dir.path().join(format!("file{i}.txt"));
        std::fs::write(&file_path, format!("content {i}"))?;

        // Some files have cache, some don't
        let cache = if i % 2 == 0 {
            Some(CachedHash {
                hash: format!("stale_hash_{i}"),
                size_at_hash: 0, // Invalid size to force rehash
                mtime_at_hash: 0,
            })
        } else {
            None
        };

        files_with_cache.push((file_path, cache));
    }

    // Hash all files in parallel
    let results = file_ops::hash_files_parallel(&files_with_cache)?;

    assert_eq!(results.len(), 5);

    // Verify all hashes are computed (not using stale cache)
    for (i, (path, hash, cache)) in results.iter().enumerate() {
        assert!(path.exists());
        assert!(!hash.is_empty());
        assert_ne!(hash, &format!("stale_hash_{i}")); // Should not use stale cache
        assert_eq!(cache.hash, *hash);
    }

    Ok(())
}

#[test]
fn test_cache_persistence_in_index() -> Result<()> {
    use dotman::storage::index::Index;

    let dir = tempdir()?;
    let index_path = dir.path().join("index.bin");

    // Create index with cached entries
    let mut index = Index::new();

    let entry1 = FileEntry {
        path: PathBuf::from("file1.txt"),
        hash: "hash1".to_string(),
        size: 100,
        modified: 1000,
        mode: 0o644,
        cached_hash: Some(CachedHash {
            hash: "hash1".to_string(),
            size_at_hash: 100,
            mtime_at_hash: 1000,
        }),
    };

    let entry2 = FileEntry {
        path: PathBuf::from("file2.txt"),
        hash: "hash2".to_string(),
        size: 200,
        modified: 2000,
        mode: 0o644,
        cached_hash: None, // No cache for this one
    };

    index.staged_entries.insert(entry1.path.clone(), entry1);
    index.staged_entries.insert(entry2.path.clone(), entry2);

    // Save index
    index.save(&index_path)?;

    // Load index back
    let loaded_index = Index::load(&index_path)?;

    // Verify cached hash is NOT preserved (by design, to avoid stale cache issues)
    // The cached_hash should only exist during runtime, not in persisted index
    let loaded_entry1 = loaded_index
        .staged_entries
        .get(&PathBuf::from("file1.txt"))
        .unwrap();
    assert!(
        loaded_entry1.cached_hash.is_none(),
        "cached_hash should not be persisted"
    );

    // Verify entry without cache also has none
    let loaded_entry2 = loaded_index
        .staged_entries
        .get(&PathBuf::from("file2.txt"))
        .unwrap();
    assert!(
        loaded_entry2.cached_hash.is_none(),
        "cached_hash should not be persisted"
    );

    Ok(())
}

#[test]
#[allow(clippy::unnecessary_wraps)]
fn test_cache_statistics() -> Result<()> {
    use dotman::storage::index::Index;

    let mut index = Index::new();

    // Empty index
    let (total, cached, hit_rate) = index.get_cache_stats();
    assert_eq!(total, 0);
    assert_eq!(cached, 0);
    assert!((hit_rate - 0.0).abs() < f64::EPSILON);

    // Add entries with cache
    for i in 0..3 {
        let entry = FileEntry {
            path: PathBuf::from(format!("cached{i}.txt")),
            hash: format!("hash{i}"),
            size: 100 * (u64::try_from(i).unwrap() + 1),
            modified: 1000 * (i64::from(i) + 1),
            mode: 0o644,
            cached_hash: Some(CachedHash {
                hash: format!("hash{i}"),
                size_at_hash: 100 * (u64::try_from(i).unwrap() + 1),
                mtime_at_hash: 1000 * (i64::from(i) + 1),
            }),
        };
        index.staged_entries.insert(entry.path.clone(), entry);
    }

    // Add entries without cache
    for i in 3..5 {
        let entry = FileEntry {
            path: PathBuf::from(format!("uncached{i}.txt")),
            hash: format!("hash{i}"),
            size: 100 * (u64::try_from(i).unwrap() + 1),
            modified: 1000 * (i64::from(i) + 1),
            mode: 0o644,
            cached_hash: None,
        };
        index.staged_entries.insert(entry.path.clone(), entry);
    }

    let (total, cached, hit_rate) = index.get_cache_stats();
    assert_eq!(total, 5);
    assert_eq!(cached, 3);
    assert!((hit_rate - 0.6).abs() < 0.01); // 60% hit rate

    Ok(())
}

#[test]
fn test_large_file_caching() -> Result<()> {
    let dir = tempdir()?;
    let file_path = dir.path().join("large.bin");

    // Create a large file (2MB)
    let large_content = vec![0u8; 2_000_000];
    std::fs::write(&file_path, &large_content)?;

    // First hash - no cache
    let (hash1, cache1) = file_ops::hash_file(&file_path, None)?;
    assert!(!hash1.is_empty());
    assert_eq!(cache1.size_at_hash, 2_000_000);

    // Second hash with cache - should be fast
    let start = std::time::Instant::now();
    let (hash2, _cache2) = file_ops::hash_file(&file_path, Some(&cache1))?;
    let cache_duration = start.elapsed();

    assert_eq!(hash2, hash1);

    // Third hash without cache - should be slower
    let start = std::time::Instant::now();
    let (hash3, _cache3) = file_ops::hash_file(&file_path, None)?;
    let no_cache_duration = start.elapsed();

    assert_eq!(hash3, hash1);

    // Cache lookup should be significantly faster than rehashing
    // This might not always be true in CI environments, so we just check it ran
    println!("Cache lookup: {cache_duration:?}, Full hash: {no_cache_duration:?}");

    Ok(())
}

#[test]
fn test_empty_file_caching() -> Result<()> {
    let dir = tempdir()?;
    let file_path = dir.path().join("empty.txt");
    std::fs::write(&file_path, "")?;

    // Hash empty file
    let (hash1, cache1) = file_ops::hash_file(&file_path, None)?;
    assert_eq!(hash1, "0"); // Empty files return "0"
    assert_eq!(cache1.hash, "0");
    assert_eq!(cache1.size_at_hash, 0);

    // Use cache for empty file
    let (hash2, cache2) = file_ops::hash_file(&file_path, Some(&cache1))?;
    assert_eq!(hash2, "0");
    assert_eq!(cache2.hash, "0");

    Ok(())
}
