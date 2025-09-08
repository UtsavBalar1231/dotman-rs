use crate::DotmanContext;
use crate::refs::resolver::RefResolver;
use crate::storage::snapshots::SnapshotManager;
use anyhow::{Context, Result};
use colored::Colorize;
use std::path::PathBuf;

/// Restore files from a specific commit
///
/// # Errors
///
/// Returns an error if:
/// - The repository is not initialized
/// - No files are specified
/// - The source reference cannot be resolved
/// - The specified commit does not exist
/// - Failed to restore files
pub fn execute(ctx: &DotmanContext, paths: &[String], source: Option<&str>) -> Result<()> {
    ctx.check_repo_initialized()?;

    if paths.is_empty() {
        return Err(anyhow::anyhow!("No files specified to restore"));
    }

    // Default to HEAD if no source is provided
    let source_ref = source.unwrap_or("HEAD");

    // Use the reference resolver to handle HEAD, HEAD~n, branches, and short hashes
    let resolver = RefResolver::new(ctx.repo_path.clone());
    let commit_id = resolver
        .resolve(source_ref)
        .with_context(|| format!("Failed to resolve reference: {source_ref}"))?;

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    let snapshot = snapshot_manager
        .load_snapshot(&commit_id)
        .with_context(|| format!("Failed to load commit: {commit_id}"))?;

    let display_commit = if commit_id.len() >= 8 {
        &commit_id[..8]
    } else {
        &commit_id
    };

    super::print_info(&format!(
        "Restoring files from commit {}",
        display_commit.yellow()
    ));

    // Get home directory as base for relative paths
    let home = dirs::home_dir().context("Could not find home directory")?;

    let mut restored_count = 0;
    let mut not_found = Vec::new();

    for path_str in paths {
        let path = PathBuf::from(path_str);

        // Normalize the path - convert absolute to relative from home
        let relative_path = if path.is_absolute() {
            path.strip_prefix(&home).unwrap_or(&path).to_path_buf()
        } else {
            path.clone()
        };

        if let Some(snapshot_file) = snapshot.files.get(&relative_path) {
            // Determine the target path for restoration
            let target_path = if path.is_absolute() {
                path.clone()
            } else {
                home.join(&path)
            };

            // Create parent directories if needed
            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            // Restore the file content
            snapshot_manager.restore_file_content(&snapshot_file.content_hash, &target_path)?;

            // Restore file permissions on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let permissions = std::fs::Permissions::from_mode(snapshot_file.mode);
                std::fs::set_permissions(&target_path, permissions)?;
            }

            println!("  {} {}", "✓".green(), target_path.display());
            restored_count += 1;
        } else {
            not_found.push(path_str.clone());
        }
    }

    // Report results
    if restored_count > 0 {
        super::print_success(&format!(
            "Restored {} file{} from commit {}",
            restored_count,
            if restored_count == 1 { "" } else { "s" },
            display_commit.yellow()
        ));
    }

    if !not_found.is_empty() {
        super::print_warning(&format!(
            "The following files were not found in commit {}: {}",
            display_commit.yellow(),
            not_found.join(", ")
        ));
    }

    if restored_count == 0 && !not_found.is_empty() {
        return Err(anyhow::anyhow!("No files were restored"));
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::used_underscore_binding)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::storage::Commit;
    use crate::storage::index::Index;
    use crate::storage::snapshots::{Snapshot, SnapshotFile};
    use std::collections::HashMap;
    use std::fs;
    use tempfile::tempdir;

    fn setup_test_context() -> Result<(tempfile::TempDir, DotmanContext)> {
        let temp = tempdir()?;
        let repo_path = temp.path().join(".dotman");
        let config_path = temp.path().join("config.toml");

        // Create repo structure
        fs::create_dir_all(&repo_path)?;
        fs::create_dir_all(repo_path.join("commits"))?;
        fs::create_dir_all(repo_path.join("objects"))?;

        // Create empty index
        let index = Index::new();
        let index_path = repo_path.join("index.bin");
        index.save(&index_path)?;

        let mut config = Config::default();
        config.core.repo_path = repo_path.clone();
        config.save(&config_path)?;

        let ctx = DotmanContext {
            repo_path,
            config_path,
            config,
            no_pager: true,
        };

        Ok((temp, ctx))
    }

    fn create_test_snapshot(
        ctx: &DotmanContext,
        id: &str,
        message: &str,
        files: Vec<(&str, &str)>,
    ) -> Result<()> {
        let mut file_map = HashMap::new();

        for (path, content) in files {
            let hash = crate::utils::hash::hash_bytes(content.as_bytes());
            let path_buf = PathBuf::from(path);
            file_map.insert(
                path_buf,
                SnapshotFile {
                    hash: hash.clone(),
                    mode: 0o644,
                    content_hash: hash.clone(),
                },
            );

            // Save object
            let object_path = ctx.repo_path.join("objects").join(format!("{}.zst", &hash));
            let compressed = zstd::stream::encode_all(content.as_bytes(), 3)?;
            fs::write(&object_path, compressed)?;
        }

        let commit = Commit {
            id: id.to_string(),
            parent: None,
            message: message.to_string(),
            author: "Test User".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            tree_hash: "test_tree".to_string(),
        };

        let snapshot = Snapshot {
            commit,
            files: file_map,
        };

        // Serialize and compress snapshot
        let serialized = crate::utils::serialization::serialize(&snapshot)?;
        let compressed = zstd::stream::encode_all(&serialized[..], 3)?;

        let snapshot_path = ctx.repo_path.join("commits").join(format!("{id}.zst"));
        fs::write(&snapshot_path, compressed)?;

        // Also set HEAD to point to this commit
        fs::write(ctx.repo_path.join("HEAD"), id)?;

        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_restore_single_file() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        // Set HOME for the test
        unsafe {
            std::env::set_var("HOME", _temp.path());
        }

        // Use a valid 32-character hex commit ID
        let commit_id = "00000000000000000000000000001234";
        create_test_snapshot(
            &ctx,
            commit_id,
            "Test commit",
            vec![
                ("file1.txt", "content1"),
                ("file2.txt", "content2"),
                (".config/file3.conf", "content3"),
            ],
        )?;

        // Restore a single file
        let result = execute(&ctx, &["file1.txt".to_string()], Some(commit_id));
        if let Err(e) = &result {
            eprintln!("Error during restore: {e:?}");
        }
        assert!(result.is_ok());

        // Verify the file was restored
        let restored_path = _temp.path().join("file1.txt");
        assert!(restored_path.exists());
        let content = fs::read_to_string(restored_path)?;
        assert_eq!(content, "content1");

        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_restore_multiple_files() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        unsafe {
            std::env::set_var("HOME", _temp.path());
        }

        let commit_id = "00000000000000000000000000005678";
        create_test_snapshot(
            &ctx,
            commit_id,
            "Test commit",
            vec![
                ("file1.txt", "content1"),
                ("file2.txt", "content2"),
                ("dir/file3.txt", "content3"),
            ],
        )?;

        // Restore multiple files
        let result = execute(
            &ctx,
            &["file1.txt".to_string(), "dir/file3.txt".to_string()],
            Some(commit_id),
        );
        assert!(result.is_ok());

        // Verify both files were restored
        assert!(_temp.path().join("file1.txt").exists());
        assert!(_temp.path().join("dir/file3.txt").exists());

        let content1 = fs::read_to_string(_temp.path().join("file1.txt"))?;
        assert_eq!(content1, "content1");

        let content3 = fs::read_to_string(_temp.path().join("dir/file3.txt"))?;
        assert_eq!(content3, "content3");

        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_restore_from_head() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        unsafe {
            std::env::set_var("HOME", _temp.path());
        }

        let commit_id = "00000000000000000000000000009abc";
        create_test_snapshot(
            &ctx,
            commit_id,
            "HEAD commit",
            vec![("test.txt", "head content")],
        )?;

        // Restore from HEAD (default)
        let result = execute(&ctx, &["test.txt".to_string()], None);
        assert!(result.is_ok());

        let content = fs::read_to_string(_temp.path().join("test.txt"))?;
        assert_eq!(content, "head content");

        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_restore_nonexistent_file() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        unsafe {
            std::env::set_var("HOME", _temp.path());
        }

        let commit_id = "0000000000000000000000000000def0";
        create_test_snapshot(
            &ctx,
            commit_id,
            "Test commit",
            vec![("exists.txt", "content")],
        )?;

        // Try to restore a file that doesn't exist in the snapshot
        let result = execute(&ctx, &["nonexistent.txt".to_string()], Some(commit_id));
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No files were restored")
        );

        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_restore_no_files_specified() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        unsafe {
            std::env::set_var("HOME", _temp.path());
        }

        let result = execute(&ctx, &[], Some("HEAD"));
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_restore_invalid_commit() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        unsafe {
            std::env::set_var("HOME", _temp.path());
        }

        let result = execute(&ctx, &["file.txt".to_string()], Some("invalid"));
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_restore_with_subdirectories() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        unsafe {
            std::env::set_var("HOME", _temp.path());
        }

        let commit_id = "00000000000000000000000000012345";
        create_test_snapshot(
            &ctx,
            commit_id,
            "Subdirectory test",
            vec![(".config/deep/nested/file.conf", "nested content")],
        )?;

        // Restore a deeply nested file
        let result = execute(
            &ctx,
            &[".config/deep/nested/file.conf".to_string()],
            Some(commit_id),
        );
        assert!(result.is_ok());

        // Verify the nested directories were created and file restored
        let nested_path = _temp.path().join(".config/deep/nested/file.conf");
        assert!(nested_path.exists());
        let content = fs::read_to_string(nested_path)?;
        assert_eq!(content, "nested content");

        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_restore_mixed_found_not_found() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        unsafe {
            std::env::set_var("HOME", _temp.path());
        }

        let commit_id = "00000000000000000000000000067890";
        create_test_snapshot(
            &ctx,
            commit_id,
            "Mixed test",
            vec![("found1.txt", "content1"), ("found2.txt", "content2")],
        )?;

        // Try to restore a mix of existing and non-existing files
        let result = execute(
            &ctx,
            &[
                "found1.txt".to_string(),
                "notfound.txt".to_string(),
                "found2.txt".to_string(),
            ],
            Some(commit_id),
        );

        // Should succeed but with warnings
        assert!(result.is_ok());

        // Verify the found files were restored
        assert!(_temp.path().join("found1.txt").exists());
        assert!(_temp.path().join("found2.txt").exists());
        assert!(!_temp.path().join("notfound.txt").exists());

        Ok(())
    }
}
