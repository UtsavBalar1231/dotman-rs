use crate::DotmanContext;
use crate::reflog::ReflogManager;
use crate::utils::pager::PagerOutput;
use anyhow::Result;
use chrono::{Local, TimeZone};
use colored::Colorize;

/// Execute the reflog command to show HEAD update history
pub fn execute(ctx: &DotmanContext, limit: usize, oneline: bool, all: bool) -> Result<()> {
    ctx.check_repo_initialized()?;

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

    let mut output = PagerOutput::new(ctx, ctx.no_pager);

    // Display entries
    for (index, entry) in entries_to_show.iter().enumerate() {
        if oneline {
            // Compact one-line format: <short_hash> HEAD@{n}: <operation>: <message>
            output.appendln(&format!(
                "{} {}: {}: {}",
                entry.short_hash().yellow(),
                format!("HEAD@{{{}}}", index).cyan(),
                entry.operation.green(),
                entry.message
            ));
        } else {
            // Full format with timestamp
            let datetime = Local
                .timestamp_opt(entry.timestamp, 0)
                .single()
                .unwrap_or_else(Local::now);

            output.appendln(&format!(
                "{} {} ({})",
                entry.short_hash().yellow(),
                format!("HEAD@{{{}}}", index).cyan(),
                datetime.format("%Y-%m-%d %H:%M:%S").to_string().dimmed()
            ));

            output.appendln(&format!(
                "{}  {}: {}",
                "    ".dimmed(),
                entry.operation.green(),
                entry.message
            ));

            if !oneline && index < entries_to_show.len() - 1 {
                output.append("\n"); // Add spacing between entries
            }
        }
    }

    if display_limit > 0 {
        output.show()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reflog::ReflogManager;
    use crate::test_utils::fixtures::{create_test_context, test_commit_id};
    use tempfile::TempDir;

    fn setup_test_context() -> Result<(TempDir, DotmanContext)> {
        create_test_context()
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

        // Add some test entries with valid commit IDs
        let commit1 = test_commit_id("01");
        let commit2 = test_commit_id("02");
        let commit3 = test_commit_id("03");

        reflog_manager.log_head_update(
            "0000000000000000000000000000000000000000",
            &commit1,
            "commit",
            "Initial commit",
        )?;

        reflog_manager.log_head_update(&commit1, &commit2, "commit", "Second commit")?;

        reflog_manager.log_head_update(
            &commit2,
            &commit3,
            "checkout",
            "checkout: moving from main to feature-branch",
        )?;

        // Test normal execution
        execute(&ctx, 10, false, false)?;

        // Test oneline format
        execute(&ctx, 10, true, false)?;

        execute(&ctx, 1, false, false)?;

        // Test show all
        execute(&ctx, 1, false, true)?;

        Ok(())
    }

    #[test]
    fn test_execute_with_limit() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        let reflog_manager = ReflogManager::new(ctx.repo_path.clone());

        // Add multiple entries with valid commit IDs
        for i in 0..5 {
            let old_commit = test_commit_id(&format!("old{:02}", i));
            let new_commit = test_commit_id(&format!("new{:02}", i));

            reflog_manager.log_head_update(
                &old_commit,
                &new_commit,
                "commit",
                &format!("Commit {}", i),
            )?;
        }

        execute(&ctx, 2, false, false)?;
        execute(&ctx, 10, false, false)?; // Should show all 5

        Ok(())
    }

    #[test]
    fn test_execute_oneline_format() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        let reflog_manager = ReflogManager::new(ctx.repo_path.clone());

        let old_commit = test_commit_id("abc123");
        let new_commit = test_commit_id("def456");

        reflog_manager.log_head_update(
            &old_commit,
            &new_commit,
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
            no_pager: true,
        };

        let result = execute(&ctx, 10, false, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_show_all_flag() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        let reflog_manager = ReflogManager::new(ctx.repo_path.clone());

        // Add more entries than the default limit with valid commit IDs
        for i in 0..25 {
            let old_commit = test_commit_id(&format!("old{:02}", i));
            let new_commit = test_commit_id(&format!("new{:02}", i));

            reflog_manager.log_head_update(
                &old_commit,
                &new_commit,
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
