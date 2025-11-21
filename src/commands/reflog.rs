use crate::DotmanContext;
use crate::output;
use crate::reflog::ReflogManager;
use crate::utils::pager::{Pager, PagerConfig};
use anyhow::Result;
use chrono::{Local, TimeZone};
use colored::Colorize;

/// Execute the reflog command to show HEAD update history
///
/// # Errors
///
/// Returns an error if:
/// - The repository is not initialized
/// - Cannot read the reflog entries
/// - Pager output fails
pub fn execute(ctx: &DotmanContext, limit: usize, oneline: bool, all: bool) -> Result<()> {
    ctx.check_repo_initialized()?;

    let reflog_manager = ReflogManager::new(ctx.repo_path.clone());
    let mut entries = reflog_manager.read_head_log()?;

    if entries.is_empty() {
        output::info("No reflog entries found");
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

    // Create pager with context
    let pager_config = PagerConfig::from_context(ctx, "reflog");
    let mut pager = Pager::builder().config(pager_config).build()?;
    let writer = pager.writer();

    // Display entries
    for (index, entry) in entries_to_show.iter().enumerate() {
        if oneline {
            // Compact one-line format: <short_hash> HEAD@{n}: <operation>: <message>
            writeln!(
                writer,
                "{} {}: {}: {}",
                entry.short_hash().yellow(),
                format!("HEAD@{{{index}}}").cyan(),
                entry.operation.green(),
                entry.message
            )?;
        } else {
            // Full format with timestamp
            let datetime = Local
                .timestamp_opt(entry.timestamp, 0)
                .single()
                .unwrap_or_else(Local::now);

            writeln!(
                writer,
                "{} {} ({})",
                entry.short_hash().yellow(),
                format!("HEAD@{{{index}}}").cyan(),
                datetime.format("%Y-%m-%d %H:%M:%S").to_string().dimmed()
            )?;

            writeln!(
                writer,
                "{}  {}: {}",
                "    ".dimmed(),
                entry.operation.green(),
                entry.message
            )?;

            if !oneline && index < entries_to_show.len() - 1 {
                writeln!(writer)?; // Add spacing between entries
            }
        }
    }

    if display_limit > 0 {
        pager.finish()?;
    }

    Ok(())
}
