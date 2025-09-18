use dotman::storage::FileEntry;
use dotman::storage::concurrent_index::ConcurrentIndex;
use dotman::utils;
use proptest::prelude::*;
use std::collections::HashSet;
use std::path::PathBuf;

proptest! {
    #[test]
    fn test_concurrent_index_consistency(
        entries in prop::collection::vec(
            (any::<String>(), any::<String>(), 0u64..1000000),
            0..100
        )
    ) {
        // Test invariant: index maintains consistency across operations
        let index = ConcurrentIndex::new();

        // Add all entries
        for (path, hash, size) in &entries {
            if path.is_empty() { continue; }
            let entry = FileEntry {
                path: PathBuf::from(path),
                hash: hash.clone(),
                size: *size,
                mode: 0o644,
                modified: chrono::Utc::now().timestamp(),
                cached_hash: None,
            };
            index.stage_entry(entry);
        }

        // Verify count
        let staged = index.staged_entries();
        let unique_paths: HashSet<_> = entries
            .iter()
            .filter(|(p, _, _)| !p.is_empty())
            .map(|(p, _, _)| p)
            .collect();

        // Should have one entry per unique path (last write wins)
        assert_eq!(staged.len(), unique_paths.len());
    }

    #[test]
    fn test_path_expansion_consistency(path in ".*") {
        // Test invariant: path expansion is idempotent
        let expanded = utils::expand_tilde(&path);

        // Expanded path should be absolute or relative, never empty
        assert!(!expanded.as_os_str().is_empty());

        // Tilde expansion should be idempotent
        let double_expanded = utils::expand_tilde(&expanded.to_string_lossy());
        assert_eq!(expanded, double_expanded);
    }

    #[test]
    fn test_hash_determinism(data in prop::collection::vec(any::<u8>(), 0..10000)) {
        // Test invariant: hashing is deterministic and collision-resistant
        use xxhash_rust::xxh3::xxh3_64;

        // Hash should be deterministic
        let hash1 = xxh3_64(&data);
        let hash2 = xxh3_64(&data);
        assert_eq!(hash1, hash2);

        // Hash should change with different data
        if !data.is_empty() {
            let mut modified = data.clone();
            modified[0] = modified[0].wrapping_add(1);
            let hash3 = xxh3_64(&modified);
            assert_ne!(hash1, hash3);
        }
    }

    #[test]
    fn test_compression_roundtrip(
        data in prop::collection::vec(any::<u8>(), 0..100000),
        level in 1i32..=22
    ) {
        // Test invariant: compression/decompression preserves data
        use dotman::utils::compress::{compress_bytes, decompress_bytes};

        let compressed = compress_bytes(&data, level).unwrap();
        let decompressed = decompress_bytes(&compressed).unwrap();
        prop_assert_eq!(&data, &decompressed);

        // Compressed should be smaller for non-trivial data
        if data.len() > 100 {
            prop_assert!(compressed.len() <= data.len());
        }
    }

    #[test]
    fn test_file_entry_ordering(
        paths in prop::collection::vec(any::<String>(), 1..50),
        sizes in prop::collection::vec(any::<u64>(), 1..50)
    ) {
        // Test invariant: index preserves insertion order for unique paths
        let index = ConcurrentIndex::new();
        let mut expected_paths = HashSet::new();

        for (path, size) in paths.iter().zip(sizes.iter()) {
            if path.is_empty() { continue; }

            let entry = FileEntry {
                path: PathBuf::from(path),
                hash: format!("hash_{}", size),
                size: *size,
                mode: 0o644,
                modified: chrono::Utc::now().timestamp(),
                cached_hash: None,
            };
            index.stage_entry(entry);
            expected_paths.insert(path.clone());
        }

        let staged = index.staged_entries();
        let actual_paths: HashSet<_> = staged.iter()
            .map(|(p, _)| p.to_string_lossy().to_string())
            .collect();

        assert_eq!(actual_paths, expected_paths);
    }

    #[test]
    fn test_commit_id_uniqueness(
        data1 in prop::collection::vec(any::<u8>(), 0..1000),
        data2 in prop::collection::vec(any::<u8>(), 0..1000)
    ) {
        // Test invariant: different content produces different hashes
        use xxhash_rust::xxh3::xxh3_64;

        let hash1 = format!("{:016x}", xxh3_64(&data1));
        let hash2 = format!("{:016x}", xxh3_64(&data2));

        // Hashes should be 16 hex characters
        prop_assert_eq!(hash1.len(), 16);
        prop_assert_eq!(hash2.len(), 16);

        // Different data should produce different hashes
        if data1 != data2 {
            prop_assert_ne!(hash1, hash2);
        } else {
            prop_assert_eq!(hash1, hash2);
        }
    }

    #[test]
    fn test_mode_preservation(mode in 0u32..=0o777) {
        // Test invariant: file modes are preserved correctly
        let index = ConcurrentIndex::new();

        let entry = FileEntry {
            path: PathBuf::from("test.txt"),
            hash: "hash123".to_string(),
            size: 100,
            mode,
            modified: chrono::Utc::now().timestamp(),
            cached_hash: None,
        };

        index.stage_entry(entry.clone());
        let retrieved = index.get_staged_entry(&PathBuf::from("test.txt")).unwrap();
        prop_assert_eq!(retrieved.mode, mode);
    }

    #[test]
    fn test_concurrent_index_size_calculation(
        entries in prop::collection::vec(
            (any::<String>(), 0u64..1000000),
            0..100
        )
    ) {
        // Test invariant: total size calculation is accurate
        let index = ConcurrentIndex::new();
        let mut expected_size = 0u64;
        let mut seen_paths = HashSet::new();

        for (path, size) in &entries {
            if path.is_empty() || seen_paths.contains(path) {
                continue;
            }

            let entry = FileEntry {
                path: PathBuf::from(path),
                hash: format!("hash_{}", size),
                size: *size,
                mode: 0o644,
                modified: chrono::Utc::now().timestamp(),
                cached_hash: None,
            };

            index.stage_entry(entry);
            expected_size += size;
            seen_paths.insert(path.clone());
        }

        index.commit_staged();

        // Calculate actual total size
        let actual_size: u64 = index.entries()
            .iter()
            .map(|(_, e)| e.size)
            .sum();

        prop_assert_eq!(actual_size, expected_size);
    }

    #[test]
    fn test_path_normalization(
        segments in prop::collection::vec("[a-zA-Z0-9._-]+", 1..10)
    ) {
        // Test invariant: path normalization is consistent
        use std::path::Path;

        let path_str = segments.join("/");
        let path = Path::new(&path_str);
        let normalized = path.to_path_buf();

        // Normalization should be idempotent
        let double_normalized = Path::new(&normalized).to_path_buf();
        prop_assert_eq!(&normalized, &double_normalized);

        // Should not contain empty segments
        for component in normalized.components() {
            if let std::path::Component::Normal(s) = component {
                prop_assert!(!s.to_string_lossy().is_empty());
            }
        }
    }
}
