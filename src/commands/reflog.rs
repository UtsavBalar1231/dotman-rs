use crate::DotmanContext;
use crate::reflog::ReflogManager;
use crate::utils::pager::PagerOutput;
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
                format!("HEAD@{{{index}}}").cyan(),
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
                format!("HEAD@{{{index}}}").cyan(),
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
