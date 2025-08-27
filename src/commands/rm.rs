use crate::storage::index::Index;
use crate::{DotmanContext, INDEX_FILE};
use anyhow::Result;
use colored::Colorize;
use std::path::{Path, PathBuf};

pub fn execute(
    ctx: &DotmanContext,
    paths: &[String],
    cached: bool,
    force: bool,
    interactive: bool,
) -> Result<()> {
    ctx.check_repo_initialized()?;

    let index_path = ctx.repo_path.join(INDEX_FILE);
    let mut index = Index::load(&index_path)?;

    let mut removed_count = 0;
    let mut not_found_count = 0;

    // Get home directory for making paths relative
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;

    for path_str in paths {
        let path = PathBuf::from(path_str);

        // Convert to relative path from home if it's absolute
        let index_path = if path.is_absolute() {
            path.strip_prefix(&home).unwrap_or(&path).to_path_buf()
        } else {
            path.clone()
        };

        // Check if file is in index
        if index.get_entry(&index_path).is_none() && !force {
            super::print_warning(&format!("File not tracked: {}", path.display()));
            not_found_count += 1;
            continue;
        }

        // Remove from index
        if index.remove_entry(&index_path).is_some() {
            println!("  {} {}", "removed:".red(), path.display());
            removed_count += 1;
        }

        // Remove from filesystem if not --cached
        if !cached && path.exists() {
            // Only prompt if interactive mode is on and not forcing
            if interactive && !force {
                if confirm_removal(&path)? {
                    std::fs::remove_file(&path)?;
                    println!("  {} {}", "deleted:".red().bold(), path.display());
                }
            } else if force || !interactive {
                // Force mode or non-interactive mode removes without asking
                std::fs::remove_file(&path)?;
                println!("  {} {}", "deleted:".red().bold(), path.display());
            }
        }
    }

    // Save updated index
    if removed_count > 0 {
        index.save(&index_path)?;
        super::print_success(&format!("Removed {} file(s) from tracking", removed_count));
    }

    if not_found_count > 0 {
        super::print_info(&format!("{} file(s) were not tracked", not_found_count));
    }

    Ok(())
}

fn confirm_removal(path: &Path) -> Result<bool> {
    use std::io::{self, Write};

    print!("Remove file {} from filesystem? [y/N]: ", path.display());
    io::stdout().flush()?;

    let mut response = String::new();
    io::stdin().read_line(&mut response)?;

    Ok(response.trim().eq_ignore_ascii_case("y"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::storage::FileEntry;
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

        // Create HEAD file (required for repo initialization check)
        fs::write(repo_path.join("HEAD"), "")?;

        let mut config = Config::default();
        config.core.repo_path = repo_path.clone();
        config.save(&config_path)?;

        let ctx = DotmanContext {
            repo_path,
            config_path,
            config,
        };

        Ok((temp, ctx))
    }

    #[test]
    fn test_execute_remove_tracked_file_cached() -> Result<()> {
        let (temp, ctx) = setup_test_context()?;

        let index_path = ctx.repo_path.join(INDEX_FILE);
        let mut index = Index::load(&index_path)?;

        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, "test content")?;

        index.add_entry(FileEntry {
            path: file_path.clone(),
            hash: "test_hash".to_string(),
            size: 12,
            modified: 0,
            mode: 0o644,
        });
        index.save(&index_path)?;

        let result = execute(
            &ctx,
            &[file_path.to_string_lossy().to_string()],
            true,
            false,
            false,
        );
        assert!(result.is_ok());

        assert!(file_path.exists());
        let index = Index::load(&index_path)?;
        assert!(index.get_entry(&file_path).is_none());

        Ok(())
    }

    #[test]
    fn test_execute_remove_untracked_file_no_force() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        let result = execute(&ctx, &["untracked.txt".to_string()], false, false, false);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_execute_remove_untracked_file_with_force() -> Result<()> {
        let (temp, ctx) = setup_test_context()?;

        let file_path = temp.path().join("untracked.txt");
        fs::write(&file_path, "content")?;

        let result = execute(
            &ctx,
            &[file_path.to_string_lossy().to_string()],
            false,
            true,
            false,
        );
        assert!(result.is_ok());

        assert!(!file_path.exists());

        Ok(())
    }

    #[test]
    fn test_execute_remove_multiple_files() -> Result<()> {
        let (temp, ctx) = setup_test_context()?;

        let index_path = ctx.repo_path.join(INDEX_FILE);
        let mut index = Index::load(&index_path)?;

        let file1 = temp.path().join("file1.txt");
        let file2 = temp.path().join("file2.txt");

        fs::write(&file1, "content1")?;
        fs::write(&file2, "content2")?;

        index.add_entry(FileEntry {
            path: file1.clone(),
            hash: "hash1".to_string(),
            size: 8,
            modified: 0,
            mode: 0o644,
        });

        index.add_entry(FileEntry {
            path: file2.clone(),
            hash: "hash2".to_string(),
            size: 8,
            modified: 0,
            mode: 0o644,
        });

        index.save(&index_path)?;

        let paths = vec![
            file1.to_string_lossy().to_string(),
            file2.to_string_lossy().to_string(),
        ];
        let result = execute(&ctx, &paths, true, false, false);
        assert!(result.is_ok());

        assert!(file1.exists());
        assert!(file2.exists());

        let index = Index::load(&index_path)?;
        assert!(index.get_entry(&file1).is_none());
        assert!(index.get_entry(&file2).is_none());

        Ok(())
    }

    #[test]
    fn test_execute_empty_paths() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        let result = execute(&ctx, &[], false, false, false);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_execute_mixed_tracked_untracked() -> Result<()> {
        let (temp, ctx) = setup_test_context()?;

        // Add one file to index
        let index_path = ctx.repo_path.join(INDEX_FILE);
        let mut index = Index::load(&index_path)?;

        let tracked_file = temp.path().join("tracked.txt");
        fs::write(&tracked_file, "tracked content")?;

        index.add_entry(FileEntry {
            path: tracked_file.clone(),
            hash: "tracked_hash".to_string(),
            size: 15,
            modified: 0,
            mode: 0o644,
        });
        index.save(&index_path)?;

        // Try to remove both tracked and untracked
        let paths = vec![
            tracked_file.to_string_lossy().to_string(),
            "untracked.txt".to_string(),
        ];

        let result = execute(&ctx, &paths, true, false, false);
        assert!(result.is_ok());

        // Tracked file should be removed from index
        let index = Index::load(&index_path)?;
        assert!(index.get_entry(&tracked_file).is_none());

        Ok(())
    }

    #[test]
    fn test_confirm_removal_parsing() {
        // Test the response parsing logic
        assert!("y".trim().eq_ignore_ascii_case("y"));
        assert!("Y".trim().eq_ignore_ascii_case("y"));
        assert!(!"n".trim().eq_ignore_ascii_case("y"));
        assert!(!"".trim().eq_ignore_ascii_case("y"));
        assert!(!"yes".trim().eq_ignore_ascii_case("y"));
    }

    #[test]
    fn test_execute_nonexistent_file() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        let result = execute(
            &ctx,
            &["/nonexistent/path/file.txt".to_string()],
            false,
            false,
            false,
        );
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_execute_with_absolute_and_relative_paths() -> Result<()> {
        let (temp, ctx) = setup_test_context()?;

        // Add files with different path types
        let index_path = ctx.repo_path.join(INDEX_FILE);
        let mut index = Index::load(&index_path)?;

        let abs_file = temp.path().join("abs.txt");
        let rel_file = PathBuf::from("rel.txt");

        fs::write(&abs_file, "absolute")?;

        index.add_entry(FileEntry {
            path: abs_file.clone(),
            hash: "abs_hash".to_string(),
            size: 8,
            modified: 0,
            mode: 0o644,
        });

        index.add_entry(FileEntry {
            path: rel_file.clone(),
            hash: "rel_hash".to_string(),
            size: 8,
            modified: 0,
            mode: 0o644,
        });

        index.save(&index_path)?;

        // Remove both
        let paths = vec![
            abs_file.to_string_lossy().to_string(),
            rel_file.to_string_lossy().to_string(),
        ];

        let result = execute(&ctx, &paths, true, false, false);
        assert!(result.is_ok());

        Ok(())
    }
}
