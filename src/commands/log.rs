use crate::DotmanContext;
use crate::refs::resolver::RefResolver;
use crate::storage::snapshots::SnapshotManager;
use anyhow::Result;
use chrono::{Local, TimeZone};
use colored::Colorize;
use std::collections::HashSet;

pub fn execute(
    ctx: &DotmanContext,
    target: Option<&str>,
    limit: usize,
    oneline: bool,
) -> Result<()> {
    ctx.ensure_repo_exists()?;

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    let snapshots = snapshot_manager.list_snapshots()?;

    if snapshots.is_empty() {
        super::print_info("No commits yet");
        return Ok(());
    }

    // Load and display commits
    let mut commits_displayed = 0;

    // If a target is specified, start from that commit and follow parent chain
    if let Some(target_ref) = target {
        // Use the reference resolver to handle HEAD, HEAD~n, branches, and short hashes
        let resolver = RefResolver::new(ctx.repo_path.clone());
        let start_commit_id = resolver.resolve(target_ref)?;

        // Follow parent chain from the starting commit
        let mut current_commit_id = Some(start_commit_id);
        let mut visited = HashSet::new();

        while let Some(commit_id) = current_commit_id {
            if commits_displayed >= limit {
                break;
            }

            // Prevent infinite loops
            if visited.contains(&commit_id) {
                break;
            }
            visited.insert(commit_id.clone());

            // Load the snapshot
            let snapshot = match snapshot_manager.load_snapshot(&commit_id) {
                Ok(s) => s,
                Err(_) => break, // Stop if we can't load a commit
            };

            let commit = &snapshot.commit;

            if oneline {
                // One-line format - show first 8 chars like git
                let display_id = if commit.id.len() >= 8 {
                    &commit.id[..8]
                } else {
                    &commit.id
                };
                println!("{} {}", display_id.yellow(), commit.message);
            } else {
                // Full format
                println!("{} {}", "commit".yellow(), commit.id);

                if let Some(parent) = &commit.parent {
                    println!("{}: {}", "Parent".bold(), &parent[..8.min(parent.len())]);
                }

                println!("{}: {}", "Author".bold(), commit.author);

                let datetime = Local
                    .timestamp_opt(commit.timestamp, 0)
                    .single()
                    .unwrap_or_else(Local::now);
                println!(
                    "{}: {}",
                    "Date".bold(),
                    datetime.format("%Y-%m-%d %H:%M:%S")
                );

                println!("\n    {}\n", commit.message);
            }

            commits_displayed += 1;

            // Move to parent commit
            current_commit_id = commit.parent.clone();
        }
    } else {
        // Original behavior: show all commits in reverse chronological order
        for snapshot_id in snapshots.iter().rev().take(limit) {
            let snapshot = snapshot_manager.load_snapshot(snapshot_id)?;
            let commit = &snapshot.commit;

            if oneline {
                // One-line format - show first 8 chars like git
                let display_id = if commit.id.len() >= 8 {
                    &commit.id[..8]
                } else {
                    &commit.id
                };
                println!("{} {}", display_id.yellow(), commit.message);
            } else {
                // Full format
                println!("{} {}", "commit".yellow(), commit.id);

                if let Some(parent) = &commit.parent {
                    println!("{}: {}", "Parent".bold(), &parent[..8.min(parent.len())]);
                }

                println!("{}: {}", "Author".bold(), commit.author);

                let datetime = Local
                    .timestamp_opt(commit.timestamp, 0)
                    .single()
                    .unwrap_or_else(Local::now);
                println!(
                    "{}: {}",
                    "Date".bold(),
                    datetime.format("%Y-%m-%d %H:%M:%S")
                );

                println!("\n    {}\n", commit.message);
            }

            commits_displayed += 1;
        }
    }

    if commits_displayed == 0 {
        super::print_info("No commits to display");
    } else if commits_displayed < snapshots.len() {
        println!(
            "\n{} (showing {} of {} commits)",
            "...".dimmed(),
            commits_displayed,
            snapshots.len()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::storage::Commit;
    use crate::storage::snapshots::Snapshot;
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

    fn create_test_snapshot(
        ctx: &DotmanContext,
        commit_id: &str,
        message: &str,
        parent: Option<String>,
    ) -> Result<()> {
        let snapshot = Snapshot {
            commit: Commit {
                id: commit_id.to_string(),
                message: message.to_string(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_secs() as i64,
                parent,
                author: "Test Author".to_string(),
                tree_hash: "test_tree_hash".to_string(),
            },
            files: HashMap::new(),
        };

        // Save snapshot directly using bincode and zstd
        use crate::utils::compress::compress_bytes;
        use crate::utils::serialization::serialize;
        let serialized = serialize(&snapshot)?;
        let compressed = compress_bytes(&serialized, ctx.config.core.compression_level)?;
        let snapshot_path = ctx
            .repo_path
            .join("commits")
            .join(format!("{}.zst", commit_id));
        fs::write(&snapshot_path, compressed)?;

        Ok(())
    }

    #[test]
    fn test_execute_no_commits() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        let result = execute(&ctx, None, 10, false);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_execute_with_commits() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        // Create test commits
        create_test_snapshot(&ctx, "commit1", "First commit", None)?;
        create_test_snapshot(
            &ctx,
            "commit2",
            "Second commit",
            Some("commit1".to_string()),
        )?;

        let result = execute(&ctx, None, 10, false);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_execute_oneline_format() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        // Create test commits
        create_test_snapshot(&ctx, "20241201120000000000abc123", "Test commit", None)?;

        let result = execute(&ctx, None, 10, true);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_execute_with_limit() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        // Create multiple commits
        for i in 1..=5 {
            let commit_id = format!("commit{}", i);
            let message = format!("Commit #{}", i);
            let parent = if i > 1 {
                Some(format!("commit{}", i - 1))
            } else {
                None
            };
            create_test_snapshot(&ctx, &commit_id, &message, parent)?;
        }

        // Test with limit of 2
        let result = execute(&ctx, None, 2, false);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_execute_limit_zero() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        create_test_snapshot(&ctx, "commit1", "Test", None)?;

        let result = execute(&ctx, None, 0, false);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_execute_with_short_commit_id() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        // Create commit with short ID
        create_test_snapshot(&ctx, "abc", "Short ID commit", None)?;

        let result = execute(&ctx, None, 10, true);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_execute_with_parent_links() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        // Create chain of commits
        create_test_snapshot(&ctx, "root", "Root commit", None)?;
        create_test_snapshot(&ctx, "child1", "Child 1", Some("root".to_string()))?;
        create_test_snapshot(&ctx, "child2", "Child 2", Some("child1".to_string()))?;

        let result = execute(&ctx, None, 10, false);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_execute_empty_repo() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        // Don't create any commits, just ensure repo exists
        ctx.ensure_repo_exists()?;

        let result = execute(&ctx, None, 10, false);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_execute_large_limit() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        // Create one commit
        create_test_snapshot(&ctx, "single", "Single commit", None)?;

        // Use limit larger than number of commits
        let result = execute(&ctx, None, 100, false);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_execute_oneline_with_multiple() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        // Create multiple commits for oneline display
        for i in 1..=3 {
            let commit_id = format!("2024120112000000000000commit{}", i);
            let message = format!("Message {}", i);
            create_test_snapshot(&ctx, &commit_id, &message, None)?;
        }

        let result = execute(&ctx, None, 10, true);
        assert!(result.is_ok());

        Ok(())
    }
}
