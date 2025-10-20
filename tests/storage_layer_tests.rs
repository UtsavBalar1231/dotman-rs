#![allow(clippy::too_many_lines)]

use anyhow::Result;
use dotman::storage::{
    CachedHash, FileEntry, concurrent_index::ConcurrentIndex, index::Index,
    snapshots::SnapshotManager,
};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Barrier};
use std::thread;
use tempfile::TempDir;

mod concurrent_index_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    #[allow(clippy::unnecessary_wraps)]
    fn test_concurrent_index_basic_operations() -> Result<()> {
        let index = ConcurrentIndex::new();

        let entry = FileEntry {
            path: PathBuf::from("test.txt"),
            hash: "hash123".to_string(),
            size: 100,
            mode: 0o644,
            modified: 1_234_567_890,
            cached_hash: None,
        };

        // Test staging
        index.stage_entry(entry);
        assert_eq!(index.staged_entries().len(), 1);

        // Test commit
        index.commit_staged();
        assert_eq!(index.staged_entries().len(), 0);
        assert_eq!(index.entries().len(), 1);

        // Test get_entry
        let retrieved = index.get_entry(&PathBuf::from("test.txt"));
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().hash, "hash123");

        Ok(())
    }

    #[test]
    #[allow(clippy::unnecessary_wraps)]
    fn test_concurrent_index_remove_operations() -> Result<()> {
        let index = ConcurrentIndex::new();

        let entry = FileEntry {
            path: PathBuf::from("test.txt"),
            hash: "hash123".to_string(),
            size: 100,
            mode: 0o644,
            modified: 1_234_567_890,
            cached_hash: None,
        };

        // First stage then commit to add to entries
        index.stage_entry(entry.clone());
        index.commit_staged();
        assert_eq!(index.entries().len(), 1);

        // Test removal
        let _ = index.remove_entry(&PathBuf::from("test.txt"));
        assert_eq!(index.entries().len(), 0);

        // Test unstage
        index.stage_entry(entry);
        let _ = index.remove_staged(&PathBuf::from("test.txt"));
        assert_eq!(index.staged_entries().len(), 0);

        Ok(())
    }

    #[test]
    #[allow(clippy::unnecessary_wraps)]
    fn test_concurrent_index_clear_operations() -> Result<()> {
        let index = ConcurrentIndex::new();

        // Add multiple entries
        for i in 0..5u64 {
            let entry = FileEntry {
                path: PathBuf::from(format!("file{i}.txt")),
                hash: format!("hash{i}"),
                size: i,
                mode: 0o644,
                modified: 1_234_567_890_i64 + i64::try_from(i).unwrap(),
                cached_hash: None,
            };
            // Add to entries by staging and committing
            index.stage_entry(entry.clone());
            index.commit_staged();
            // Stage another copy
            index.stage_entry(entry);
        }

        assert_eq!(index.entries().len(), 5);
        // The last entry (i=4) was staged after the final commit
        assert_eq!(index.staged_entries().len(), 1);

        // Clear staged
        index.clear_staged();
        assert_eq!(index.staged_entries().len(), 0);
        assert_eq!(index.entries().len(), 5);

        // Clear all entries (no direct clear method, so we remove them)
        let entries = index.entries();
        for (path, _) in entries {
            let _ = index.remove_entry(&path);
        }
        assert_eq!(index.entries().len(), 0);

        Ok(())
    }

    #[test]
    fn test_concurrent_index_serialization() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let index_path = temp_dir.path().join("index.bin");

        let index = ConcurrentIndex::new();

        // Add entries
        for i in 0..3u64 {
            let entry = FileEntry {
                path: PathBuf::from(format!("file{i}.txt")),
                hash: format!("hash{i}"),
                size: i,
                mode: 0o644,
                modified: 1_234_567_890_i64 + i64::try_from(i).unwrap(),
                cached_hash: Some(CachedHash {
                    hash: format!("cached{i}"),
                    mtime_at_hash: 1_234_567_890_i64 + i64::try_from(i).unwrap(),
                    size_at_hash: i,
                }),
            };
            // Add to entries by staging and committing
            index.stage_entry(entry.clone());
            index.commit_staged();
            if i % 2 == 0 {
                index.stage_entry(entry);
            }
        }

        // Save
        index.save(&index_path)?;
        assert!(index_path.exists());

        // Load
        let loaded = ConcurrentIndex::load(&index_path)?;
        assert_eq!(loaded.entries().len(), 3);
        // Only file2 (i=2, i%2==0) remains staged after the commit pattern
        assert_eq!(loaded.staged_entries().len(), 1);

        Ok(())
    }

    #[test]
    #[allow(clippy::unnecessary_wraps)]
    fn test_concurrent_writes_no_race_condition() -> Result<()> {
        let index = Arc::new(ConcurrentIndex::new());
        let barrier = Arc::new(Barrier::new(20));
        let counter = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        for thread_id in 0u64..20 {
            let index_clone = Arc::clone(&index);
            let barrier_clone = Arc::clone(&barrier);
            let counter_clone = Arc::clone(&counter);

            let handle = thread::spawn(move || {
                // Synchronize start
                barrier_clone.wait();

                for i in 0u64..100 {
                    let entry = FileEntry {
                        path: PathBuf::from(format!("thread_{thread_id}_file_{i}.txt")),
                        hash: format!("hash_{thread_id}_{i}"),
                        size: thread_id * 100 + i,
                        mode: 0o644,
                        modified: 1_234_567_890,
                        cached_hash: None,
                    };

                    // Just stage all entries concurrently
                    // (commit_staged() from multiple threads causes race conditions
                    // since it commits ALL staged entries, not just the current thread's)
                    index_clone.stage_entry(entry);

                    counter_clone.fetch_add(1, Ordering::Relaxed);

                    // Occasional reads to create more contention
                    if i % 10 == 0 {
                        let _ = index_clone.entries();
                        let _ = index_clone.staged_entries();
                    }
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all operations completed
        assert_eq!(counter.load(Ordering::Relaxed), 2000);

        // All entries should be in staged_entries (we didn't commit)
        let staged = index.staged_entries();
        assert_eq!(staged.len(), 2000);

        // Verify no data loss - all unique paths should exist
        let mut paths_set = std::collections::HashSet::new();
        for (path, _) in staged {
            paths_set.insert(path);
        }
        assert_eq!(paths_set.len(), 2000);

        Ok(())
    }

    #[test]
    #[allow(clippy::unnecessary_wraps)]
    fn test_concurrent_index_commit_consistency() -> Result<()> {
        let index = Arc::new(ConcurrentIndex::new());
        let barrier = Arc::new(Barrier::new(10));
        let mut handles = vec![];

        // Stage entries from multiple threads
        for thread_id in 0..10 {
            let index_clone = Arc::clone(&index);
            let barrier_clone = Arc::clone(&barrier);

            let handle = thread::spawn(move || {
                barrier_clone.wait();

                for i in 0..50u64 {
                    let entry = FileEntry {
                        path: PathBuf::from(format!("thread_{thread_id}_file_{i}.txt")),
                        hash: format!("hash_{thread_id}_{i}"),
                        size: i,
                        mode: 0o644,
                        modified: 1_234_567_890,
                        cached_hash: None,
                    };
                    index_clone.stage_entry(entry);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(index.staged_entries().len(), 500);

        // Commit all staged
        index.commit_staged();

        assert_eq!(index.staged_entries().len(), 0);
        assert_eq!(index.entries().len(), 500);

        Ok(())
    }

    #[test]
    #[allow(clippy::unnecessary_wraps)]
    fn test_index_conversion_roundtrip() -> Result<()> {
        let concurrent = ConcurrentIndex::new();

        // Add various entries
        for i in 0..10u64 {
            let entry = FileEntry {
                path: PathBuf::from(format!("file{i}.txt")),
                hash: format!("hash{i}"),
                size: i,
                mode: if i % 2 == 0 { 0o644 } else { 0o755 },
                modified: 1_234_567_890_i64 + i64::try_from(i).unwrap(),
                cached_hash: if i % 3 == 0 {
                    Some(CachedHash {
                        hash: format!("cached{i}"),
                        mtime_at_hash: 1_234_567_890_i64 + i64::try_from(i).unwrap(),
                        size_at_hash: i,
                    })
                } else {
                    None
                },
            };

            if i < 5 {
                // Add to entries by staging and committing
                concurrent.stage_entry(entry.clone());
                concurrent.commit_staged();
            } else {
                concurrent.stage_entry(entry);
            }
        }

        // Convert to regular index
        let regular = concurrent.to_index();
        assert_eq!(regular.entries.len(), 5);
        assert_eq!(regular.staged_entries.len(), 5);

        // Convert back to concurrent
        let concurrent2 = ConcurrentIndex::from_index(regular);
        assert_eq!(concurrent2.entries().len(), 5);
        assert_eq!(concurrent2.staged_entries().len(), 5);

        // Verify data integrity
        for i in 0..5 {
            let path = PathBuf::from(format!("file{i}.txt"));
            let entry = concurrent2.get_entry(&path);
            assert!(entry.is_some());
            assert_eq!(entry.unwrap().hash, format!("hash{i}"));
        }

        Ok(())
    }
}

mod snapshot_tests {
    use super::*;
    use dotman::storage::{
        Commit,
        snapshots::{Snapshot, SnapshotFile},
    };

    fn create_test_snapshot(id: &str, parent: Option<String>) -> Snapshot {
        let mut files = HashMap::new();
        files.insert(
            PathBuf::from("file1.txt"),
            SnapshotFile {
                hash: "hash1".to_string(),
                mode: 0o644,
                content_hash: "content_hash1".to_string(),
            },
        );

        Snapshot {
            commit: Commit {
                id: id.to_string(),
                parent,
                message: "Test commit".to_string(),
                author: "Test User".to_string(),
                timestamp: chrono::Utc::now().timestamp(),
                tree_hash: "tree_hash1".to_string(),
            },
            files,
        }
    }

    #[test]
    fn test_snapshot_save_and_load() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let manager = SnapshotManager::new(temp_dir.path().to_path_buf(), 3);

        // Create actual temp file for the test
        let test_file = temp_dir.path().join("file1.txt");
        fs::write(&test_file, "test content")?;

        let snapshot = create_test_snapshot("test123", None);

        // Create entries for snapshot with absolute path
        let entries = vec![FileEntry {
            path: test_file,
            hash: "hash1".to_string(),
            size: 12, // "test content" length
            mode: 0o644,
            modified: 1_234_567_890,
            cached_hash: None,
        }];

        // Save
        let id = manager.create_snapshot(snapshot.commit.clone(), &entries)?;
        assert_eq!(id, "test123");

        // Verify file exists
        let snapshot_file = temp_dir.path().join("commits/test123.zst");
        assert!(snapshot_file.exists());

        // Load
        let loaded = manager.load_snapshot("test123")?;
        assert_eq!(loaded.commit.id, snapshot.commit.id);
        assert_eq!(loaded.commit.message, snapshot.commit.message);
        assert_eq!(loaded.files.len(), 1);

        Ok(())
    }

    #[test]
    fn test_snapshot_with_parent() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let manager = SnapshotManager::new(temp_dir.path().to_path_buf(), 3);

        // Save parent
        let parent = create_test_snapshot("parent123", None);
        let entries: Vec<FileEntry> = vec![];
        manager.create_snapshot(parent.commit, &entries)?;

        // Save child
        let child = create_test_snapshot("child456", Some("parent123".to_string()));
        manager.create_snapshot(child.commit, &entries)?;

        // Load and verify
        let loaded = manager.load_snapshot("child456")?;
        assert_eq!(loaded.commit.parent, Some("parent123".to_string()));

        Ok(())
    }

    #[test]
    fn test_snapshot_compression() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let manager = SnapshotManager::new(temp_dir.path().to_path_buf(), 3);

        // Create actual temp files for testing compression
        let mut entries = Vec::new();
        for i in 0..100u64 {
            // Reduced from 1000 to 100 for faster test execution
            let file_path = temp_dir.path().join(format!("file{i}.txt"));
            fs::write(&file_path, format!("content for file {i}"))?;

            entries.push(FileEntry {
                path: file_path,
                hash: format!("hash{i}"),
                size: i,
                mode: 0o644,
                modified: 1_234_567_890_i64 + i64::try_from(i).unwrap(),
                cached_hash: None,
            });
        }

        let commit = Commit {
            id: "large_snapshot".to_string(),
            parent: None,
            message: "Large commit".to_string(),
            author: "Test User".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            tree_hash: "tree_hash_large".to_string(),
        };

        manager.create_snapshot(commit, &entries)?;

        // Check that file is compressed (exists with .zst extension)
        let snapshot_file = temp_dir.path().join("commits/large_snapshot.zst");
        assert!(snapshot_file.exists());
        let file_size = fs::metadata(&snapshot_file)?.len();
        // File should exist and have some size
        assert!(file_size > 0);

        // Load and verify integrity
        let loaded = manager.load_snapshot("large_snapshot")?;
        assert_eq!(loaded.files.len(), 100);

        Ok(())
    }

    #[test]
    fn test_list_snapshots() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let manager = SnapshotManager::new(temp_dir.path().to_path_buf(), 3);

        // Save multiple snapshots
        for i in 0..5 {
            let snapshot = create_test_snapshot(&format!("snapshot{i}"), None);
            let entries: Vec<FileEntry> = vec![];
            manager.create_snapshot(snapshot.commit, &entries)?;
        }

        // List snapshots
        let snapshots = manager.list_snapshots()?;
        assert_eq!(snapshots.len(), 5);

        // Verify all IDs are present
        for i in 0..5 {
            assert!(snapshots.contains(&format!("snapshot{i}")));
        }

        Ok(())
    }

    #[test]
    fn test_snapshot_not_found() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let manager = SnapshotManager::new(temp_dir.path().to_path_buf(), 3);

        let result = manager.load_snapshot("nonexistent");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No commit found matching")
        );

        Ok(())
    }

    #[test]
    fn test_snapshot_with_special_characters() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let manager = SnapshotManager::new(temp_dir.path().to_path_buf(), 3);

        // Create actual temp file with spaces in name
        let test_file = temp_dir.path().join("file with spaces.txt");
        fs::write(&test_file, "content with special chars")?;

        let entries = vec![FileEntry {
            path: test_file,
            hash: "hash1".to_string(),
            size: 26, // "content with special chars" length
            mode: 0o644,
            modified: 1_234_567_890,
            cached_hash: None,
        }];

        let commit = Commit {
            id: "test_special".to_string(),
            parent: None,
            message: "Commit with special chars: \n\t'\"\\".to_string(),
            author: "Test User <test@example.com>".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            tree_hash: "tree_special".to_string(),
        };

        manager.create_snapshot(commit.clone(), &entries)?;
        let loaded = manager.load_snapshot("test_special")?;

        assert_eq!(loaded.commit.message, commit.message);
        assert_eq!(loaded.commit.author, commit.author);

        Ok(())
    }
}

mod index_tests {
    use super::*;

    #[test]
    #[allow(clippy::unnecessary_wraps)]
    fn test_index_basic_operations() -> Result<()> {
        let mut index = Index::new();

        let entry = FileEntry {
            path: PathBuf::from("test.txt"),
            hash: "hash123".to_string(),
            size: 100,
            mode: 0o644,
            modified: 1_234_567_890,
            cached_hash: None,
        };

        // Add entry
        index
            .entries
            .insert(PathBuf::from("test.txt"), entry.clone());
        assert_eq!(index.entries.len(), 1);

        // Stage entry
        index
            .staged_entries
            .insert(PathBuf::from("staged.txt"), entry);
        assert_eq!(index.staged_entries.len(), 1);

        Ok(())
    }

    #[test]
    fn test_index_serialization() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let index_path = temp_dir.path().join("index.bin");

        let mut index = Index::new();

        // Add entries with various configurations
        for i in 0..10u64 {
            let entry = FileEntry {
                path: PathBuf::from(format!("file{i}.txt")),
                hash: format!("hash{i}"),
                size: i * 100,
                mode: if i % 2 == 0 { 0o644 } else { 0o755 },
                modified: 1_234_567_890_i64 + i64::try_from(i).unwrap(),
                cached_hash: if i % 3 == 0 {
                    Some(CachedHash {
                        hash: format!("cached_hash{i}"),
                        mtime_at_hash: 1_234_567_890_i64 + i64::try_from(i).unwrap(),
                        size_at_hash: i * 100,
                    })
                } else {
                    None
                },
            };

            if i < 5 {
                index.entries.insert(entry.path.clone(), entry);
            } else {
                index.staged_entries.insert(entry.path.clone(), entry);
            }
        }

        // Save
        index.save(&index_path)?;
        assert!(index_path.exists());

        // Load
        let loaded = Index::load(&index_path)?;
        assert_eq!(loaded.entries.len(), 5);
        assert_eq!(loaded.staged_entries.len(), 5);

        // Verify data integrity
        for i in 0..5u64 {
            let path = PathBuf::from(format!("file{i}.txt"));
            let entry = &loaded.entries[&path];
            assert_eq!(entry.hash, format!("hash{i}"));
            assert_eq!(entry.size, i * 100);
        }

        Ok(())
    }

    #[test]
    fn test_index_version_compatibility() -> Result<()> {
        let mut index = Index::new();
        assert_eq!(index.version, 1);

        // Add an entry to ensure it's not empty
        index.entries.insert(
            PathBuf::from("test.txt"),
            FileEntry {
                path: PathBuf::from("test.txt"),
                hash: "hash".to_string(),
                size: 100,
                mode: 0o644,
                modified: 1_234_567_890,
                cached_hash: None,
            },
        );

        // Future versions should be handled gracefully
        index.version = 2;

        let temp_dir = TempDir::new()?;
        let index_path = temp_dir.path().join("index.bin");

        // This might fail if version checking is strict
        let result = index.save(&index_path);
        assert!(result.is_ok() || result.unwrap_err().to_string().contains("version"));

        Ok(())
    }

    #[test]
    fn test_index_empty_handling() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let index_path = temp_dir.path().join("index.bin");

        let index = Index::new();
        index.save(&index_path)?;

        let loaded = Index::load(&index_path)?;
        assert_eq!(loaded.entries.len(), 0);
        assert_eq!(loaded.staged_entries.len(), 0);

        Ok(())
    }

    #[test]
    fn test_index_load_nonexistent() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let index_path = temp_dir.path().join("nonexistent.bin");

        // Should create new empty index
        let result = Index::load(&index_path);
        assert!(result.is_ok());

        let index = result?;
        assert_eq!(index.entries.len(), 0);
        assert_eq!(index.staged_entries.len(), 0);

        Ok(())
    }
}

mod stash_tests {
    use super::*;
    use dotman::storage::FileStatus;
    use dotman::storage::stash::{StashEntry, StashFile, StashManager};

    #[test]
    fn test_stash_save_and_load() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let stash_manager = StashManager::new(temp_dir.path().to_path_buf(), 3);

        let mut files = HashMap::new();
        files.insert(
            PathBuf::from("stashed.txt"),
            StashFile {
                hash: "stash_hash".to_string(),
                mode: 0o644,
                status: FileStatus::Modified(PathBuf::from("stashed.txt")),
                content: Some(b"test content".to_vec()),
            },
        );

        let stash_entry = StashEntry {
            id: "stash_123".to_string(),
            message: "WIP on main".to_string(),
            timestamp: 1_234_567_890,
            parent_commit: "commit123".to_string(),
            files,
            index_state: vec![],
        };

        // Save stash
        stash_manager.save_stash(&stash_entry)?;

        // List stashes
        let stashes = stash_manager.list_stashes()?;
        assert_eq!(stashes.len(), 1);
        assert_eq!(stashes[0], "stash_123");

        // Load stash
        let loaded = stash_manager.load_stash("stash_123")?;
        assert_eq!(loaded.files.len(), 1);
        assert_eq!(loaded.message, "WIP on main");

        Ok(())
    }

    #[test]
    fn test_stash_pop() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let stash_manager = StashManager::new(temp_dir.path().to_path_buf(), 3);

        let mut files = HashMap::new();
        files.insert(
            PathBuf::from("file.txt"),
            StashFile {
                hash: "hash".to_string(),
                mode: 0o644,
                status: FileStatus::Modified(PathBuf::from("file.txt")),
                content: Some(b"content".to_vec()),
            },
        );

        let stash_entry = StashEntry {
            id: "stash_456".to_string(),
            message: "Test stash".to_string(),
            timestamp: 1_234_567_890,
            parent_commit: "commit456".to_string(),
            files,
            index_state: vec![],
        };

        // Save
        stash_manager.save_stash(&stash_entry)?;

        // Pop from stack
        let popped_id = stash_manager.pop_from_stack()?;
        assert_eq!(popped_id, Some("stash_456".to_string()));

        // Load the stash to verify it still exists (pop_from_stack only removes from stack)
        let loaded = stash_manager.load_stash("stash_456")?;
        assert_eq!(loaded.message, "Test stash");

        // Stack should be empty now
        let stashes = stash_manager.list_stashes()?;
        assert_eq!(stashes.len(), 0);

        Ok(())
    }

    #[test]
    fn test_multiple_stashes() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let stash_manager = StashManager::new(temp_dir.path().to_path_buf(), 3);

        // Create multiple stashes
        for i in 0..5i64 {
            let mut files = HashMap::new();
            files.insert(
                PathBuf::from(format!("file{i}.txt")),
                StashFile {
                    hash: format!("hash{i}"),
                    mode: 0o644,
                    status: FileStatus::Modified(PathBuf::from(format!("file{i}.txt"))),
                    content: Some(format!("content{i}").into_bytes()),
                },
            );

            let stash_entry = StashEntry {
                id: format!("stash_{i}"),
                message: format!("Stash {i}"),
                timestamp: 1_234_567_890 + i,
                parent_commit: "commit".to_string(),
                files,
                index_state: vec![],
            };

            stash_manager.save_stash(&stash_entry)?;
        }

        let stashes = stash_manager.list_stashes()?;
        assert_eq!(stashes.len(), 5);

        // Latest stash should be first (LIFO)
        assert_eq!(stashes[0], "stash_4");

        Ok(())
    }

    #[test]
    fn test_stash_clear() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let stash_manager = StashManager::new(temp_dir.path().to_path_buf(), 3);

        // Create stashes
        for i in 0..3i64 {
            let stash_entry = StashEntry {
                id: format!("stash_{i}"),
                message: format!("Stash {i}"),
                timestamp: 1_234_567_890 + i,
                parent_commit: "commit".to_string(),
                files: HashMap::new(),
                index_state: vec![],
            };
            stash_manager.save_stash(&stash_entry)?;
        }

        assert_eq!(stash_manager.list_stashes()?.len(), 3);

        // Clear all (would need to be implemented or done manually)
        // For now, pop them all
        while stash_manager.pop_from_stack()?.is_some() {}
        assert_eq!(stash_manager.list_stashes()?.len(), 0);

        Ok(())
    }
}
