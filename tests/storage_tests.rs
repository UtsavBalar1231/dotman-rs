use anyhow::Result;
use dotman::storage::FileEntry;
use dotman::storage::concurrent_index::ConcurrentIndex;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use tempfile::TempDir;

#[test]
fn test_concurrent_index_basic_operations() -> Result<()> {
    let index = ConcurrentIndex::new();

    let entry = FileEntry {
        path: PathBuf::from("test.txt"),
        hash: "abc123".to_string(),
        size: 100,
        mode: 0o644,
        modified: chrono::Utc::now().timestamp(),
        cached_hash: None,
    };

    // Test staging
    index.stage_entry(entry.clone());
    assert!(index.has_staged_changes());

    let staged = index.get_staged_entry(&PathBuf::from("test.txt"));
    assert!(staged.is_some());
    assert_eq!(staged.unwrap().hash, "abc123");

    // Test commit
    index.commit_staged();
    assert!(!index.has_staged_changes());

    let committed = index.get_entry(&PathBuf::from("test.txt"));
    assert!(committed.is_some());
    assert_eq!(committed.unwrap().hash, "abc123");

    Ok(())
}

#[test]
fn test_concurrent_index_thread_safety() -> Result<()> {
    let index = Arc::new(ConcurrentIndex::new());
    let mut handles = vec![];

    // Spawn multiple threads that write to the index
    for i in 0..10 {
        let index_clone = Arc::clone(&index);
        let handle = thread::spawn(move || {
            for j in 0..100 {
                let entry = FileEntry {
                    path: PathBuf::from(format!("file_{}_{}.txt", i, j)),
                    hash: format!("hash_{}_{}", i, j),
                    size: (i * 100 + j) as u64,
                    mode: 0o644,
                    modified: chrono::Utc::now().timestamp(),
                    cached_hash: None,
                };
                index_clone.stage_entry(entry);
            }
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all entries were added
    let staged = index.staged_entries();
    assert_eq!(staged.len(), 1000);

    Ok(())
}

#[test]
fn test_index_persistence() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let index_path = temp_dir.path().join("index.bin");

    let index1 = ConcurrentIndex::new();

    // Add some entries
    for i in 0..5 {
        let entry = FileEntry {
            path: PathBuf::from(format!("file_{}.txt", i)),
            hash: format!("hash_{}", i),
            size: i * 100,
            mode: 0o644,
            modified: chrono::Utc::now().timestamp(),
            cached_hash: None,
        };
        index1.stage_entry(entry);
    }
    index1.commit_staged();

    // Save to disk
    index1.save(&index_path)?;

    // Load from disk
    let index2 = ConcurrentIndex::load(&index_path)?;

    // Verify entries were persisted
    assert_eq!(index2.len(), 5);
    for i in 0..5 {
        let entry = index2.get_entry(&PathBuf::from(format!("file_{}.txt", i)));
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().hash, format!("hash_{}", i));
    }

    Ok(())
}

#[test]
fn test_index_remove_entry() -> Result<()> {
    let index = ConcurrentIndex::new();

    let entry = FileEntry {
        path: PathBuf::from("test.txt"),
        hash: "hash123".to_string(),
        size: 100,
        mode: 0o644,
        modified: chrono::Utc::now().timestamp(),
        cached_hash: None,
    };

    index.stage_entry(entry.clone());
    index.commit_staged();

    assert!(index.get_entry(&PathBuf::from("test.txt")).is_some());

    // Remove entry
    let _ = index.remove_entry(&PathBuf::from("test.txt"));
    assert!(index.get_entry(&PathBuf::from("test.txt")).is_none());

    Ok(())
}

#[test]
fn test_index_clear_staged() -> Result<()> {
    let index = ConcurrentIndex::new();

    // Stage multiple entries
    for i in 0..5 {
        let entry = FileEntry {
            path: PathBuf::from(format!("file_{}.txt", i)),
            hash: format!("hash_{}", i),
            size: i * 100,
            mode: 0o644,
            modified: chrono::Utc::now().timestamp(),
            cached_hash: None,
        };
        index.stage_entry(entry);
    }

    assert!(index.has_staged_changes());
    assert_eq!(index.staged_entries().len(), 5);

    // Clear staged
    index.clear_staged();
    assert!(!index.has_staged_changes());
    assert_eq!(index.staged_entries().len(), 0);

    Ok(())
}

#[test]
fn test_index_get_all_paths() -> Result<()> {
    let index = ConcurrentIndex::new();

    let paths: HashSet<PathBuf> = (0..10)
        .map(|i| {
            let path = PathBuf::from(format!("file_{}.txt", i));
            let entry = FileEntry {
                path: path.clone(),
                hash: format!("hash_{}", i),
                size: i * 100,
                mode: 0o644,
                modified: chrono::Utc::now().timestamp(),
                cached_hash: None,
            };
            index.stage_entry(entry);
            path
        })
        .collect();

    index.commit_staged();

    let all_paths: HashSet<_> = index.entries().into_iter().map(|(p, _)| p).collect();
    assert_eq!(all_paths, paths);

    Ok(())
}

#[test]
fn test_concurrent_stage_and_commit() -> Result<()> {
    let index = Arc::new(ConcurrentIndex::new());
    let mut handles = vec![];

    // Thread 1: Stage entries
    let index1 = Arc::clone(&index);
    let h1 = thread::spawn(move || {
        for i in 0..100 {
            let entry = FileEntry {
                path: PathBuf::from(format!("staged_{}.txt", i)),
                hash: format!("hash_{}", i),
                size: i,
                mode: 0o644,
                modified: chrono::Utc::now().timestamp(),
                cached_hash: None,
            };
            index1.stage_entry(entry);
            if i % 10 == 0 {
                thread::yield_now();
            }
        }
    });
    handles.push(h1);

    // Thread 2: Periodically commit staged entries
    let index2 = Arc::clone(&index);
    let h2 = thread::spawn(move || {
        thread::sleep(std::time::Duration::from_millis(5));
        for _ in 0..5 {
            index2.commit_staged();
            thread::sleep(std::time::Duration::from_millis(10));
        }
    });
    handles.push(h2);

    for handle in handles {
        handle.join().unwrap();
    }

    // Final commit to ensure all staged are committed
    index.commit_staged();

    // Verify all entries are in committed state
    let total_entries = index.len();
    assert!(total_entries > 0);
    assert!(!index.has_staged_changes());

    Ok(())
}
