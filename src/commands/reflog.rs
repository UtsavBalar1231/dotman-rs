use crate::DotmanContext;
use crate::reflog::ReflogManager;
use anyhow::Result;
use chrono::{Local, TimeZone};
use colored::Colorize;

/// Execute the reflog command to show HEAD update history
pub fn execute(ctx: &DotmanContext, limit: usize, oneline: bool, all: bool) -> Result<()> {
    ctx.ensure_repo_exists()?;

    let reflog_manager = ReflogManager::new(ctx.repo_path.clone());
    let mut entries = reflog_manager.read_head_log()?;

    if entries.is_empty() {
        super::print_info("No reflog entries found");
        return Ok(());
    }

    // Reverse to show most recent first (like git reflog)
    entries.reverse();

    // Apply limit unless showing all
    let display_limit = if all {
        entries.len()
    } else {
        limit.min(entries.len())
    };
    let entries_to_show = &entries[..display_limit];

    // Display entries
    for (index, entry) in entries_to_show.iter().enumerate() {
        if oneline {
            // Compact one-line format: <short_hash> HEAD@{n}: <operation>: <message>
            println!(
                "{} {}: {}: {}",
                entry.short_hash().yellow(),
                format!("HEAD@{{{}}}", index).cyan(),
                entry.operation.green(),
                entry.message
            );
        } else {
            // Full format with timestamp
            let datetime = Local
                .timestamp_opt(entry.timestamp, 0)
                .single()
                .unwrap_or_else(Local::now);

            println!(
                "{} {} ({})",
                entry.short_hash().yellow(),
                format!("HEAD@{{{}}}", index).cyan(),
                datetime.format("%Y-%m-%d %H:%M:%S").to_string().dimmed()
            );

            println!(
                "{}  {}: {}",
                "    ".dimmed(),
                entry.operation.green(),
                entry.message
            );

            if !oneline && index < entries_to_show.len() - 1 {
                println!(); // Add spacing between entries
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reflog::ReflogManager;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_context() -> Result<(TempDir, DotmanContext)> {
        let temp_dir = TempDir::new()?;
        let home_dir = temp_dir.path();
        let repo_path = home_dir.join(".dotman");

        fs::create_dir_all(&repo_path)?;

        let ctx = DotmanContext {
            repo_path: repo_path.clone(),
            config_path: home_dir.join(".config/dotman/config"),
            config: Default::default(),
        };

        Ok((temp_dir, ctx))
    }

    #[test]
    fn test_execute_empty_reflog() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        // Should not fail on empty reflog
        execute(&ctx, 10, false, false)?;

        Ok(())
    }

    #[test]
    fn test_execute_with_entries() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        let reflog_manager = ReflogManager::new(ctx.repo_path.clone());

        // Add some test entries
        reflog_manager.log_head_update(
            "0000000000000000000000000000000000000000",
            "abc123def456789012345678901234567890abcd",
            "commit",
            "Initial commit",
        )?;

        reflog_manager.log_head_update(
            "abc123def456789012345678901234567890abcd",
            "def456abc123789012345678901234567890abcd",
            "commit",
            "Second commit",
        )?;

        reflog_manager.log_head_update(
            "def456abc123789012345678901234567890abcd",
            "ghi789def456123789012345678901234567890ab",
            "checkout",
            "checkout: moving from main to feature-branch",
        )?;

        // Test normal execution
        execute(&ctx, 10, false, false)?;

        // Test oneline format
        execute(&ctx, 10, true, false)?;

        // Test with limit
        execute(&ctx, 1, false, false)?;

        // Test show all
        execute(&ctx, 1, false, true)?;

        Ok(())
    }

    #[test]
    fn test_execute_with_limit() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        let reflog_manager = ReflogManager::new(ctx.repo_path.clone());

        // Add multiple entries
        for i in 0..5 {
            reflog_manager.log_head_update(
                &format!("old{:02}", i),
                &format!("new{:02}", i),
                "commit",
                &format!("Commit {}", i),
            )?;
        }

        // Test with different limits
        execute(&ctx, 2, false, false)?;
        execute(&ctx, 10, false, false)?; // Should show all 5

        Ok(())
    }

    #[test]
    fn test_execute_oneline_format() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        let reflog_manager = ReflogManager::new(ctx.repo_path.clone());

        reflog_manager.log_head_update(
            "abc123def456",
            "def456abc123",
            "checkout",
            "checkout: moving from main to develop",
        )?;

        // Should execute without errors in oneline mode
        execute(&ctx, 10, true, false)?;

        Ok(())
    }

    #[test]
    fn test_execute_nonexistent_repo() {
        let temp_dir = TempDir::new().unwrap();
        let home_dir = temp_dir.path();

        let ctx = DotmanContext {
            repo_path: home_dir.join("nonexistent"),
            config_path: home_dir.join(".config/dotman/config"),
            config: Default::default(),
        };

        // Should succeed but show no reflog entries for empty repo
        let result = execute(&ctx, 10, false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_show_all_flag() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        let reflog_manager = ReflogManager::new(ctx.repo_path.clone());

        // Add more entries than the default limit
        for i in 0..25 {
            reflog_manager.log_head_update(
                &format!("old{:02}", i),
                &format!("new{:02}", i),
                "commit",
                &format!("Commit {}", i),
            )?;
        }

        // With all=false and limit=10, should show only 10
        execute(&ctx, 10, false, false)?;

        // With all=true, should show all 25
        execute(&ctx, 10, false, true)?;

        Ok(())
    }
}
