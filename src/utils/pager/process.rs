use super::{PagerConfig, parser::parse_pager_command, writer::PagerProcess};
use anyhow::{Context, Result};
use command_group::CommandGroup;
use crossterm::tty::IsTty;
use std::env;
use std::io;
use std::process::{Command, Stdio};
use tracing::{Level, debug, info, span};

/// Spawn a pager process and return a `PagerProcess` writer
pub fn spawn_pager(cmd: &str) -> Result<PagerProcess> {
    let span = span!(Level::DEBUG, "spawn_pager", cmd);
    let _guard = span.enter();

    // Parse command with proper shell-like parsing
    let (program, args) = parse_pager_command(cmd)?;

    debug!(program = %program, args = ?args, "Spawning pager process");

    // Set LESS environment variable if using less and not already set
    if program.contains("less") && env::var("LESS").is_err() {
        // SAFETY: We're only setting this before spawning the pager process,
        // and we check that it's not already set to avoid interfering with user config
        unsafe {
            env::set_var("LESS", "FRX");
        }
        debug!("Set LESS=FRX for pager");
    }

    // Spawn as process group for proper cleanup
    let mut group = Command::new(&program)
        .args(&args)
        .stdin(Stdio::piped())
        .group_spawn()
        .with_context(|| format!("Failed to spawn pager: {program}"))?;

    // Take ownership of stdin - command_group 5.0 uses inner() instead of inner_mut()
    let stdin = group
        .inner()
        .stdin
        .take()
        .context("Failed to open pager stdin")?;

    info!(program = %program, "Pager process spawned successfully");

    Ok(PagerProcess::new(group, stdin))
}

/// Determine if pager should be used based on environment and config
pub fn should_use_pager(config: &PagerConfig) -> bool {
    let span = span!(Level::DEBUG, "should_use_pager");
    let _guard = span.enter();

    // Check NO_PAGER environment variable (standard Unix convention)
    if env::var("NO_PAGER").is_ok() {
        debug!("Pager disabled by NO_PAGER environment variable");
        return false;
    }

    // Check if output is to a terminal (not piped or redirected)
    if !io::stdout().is_tty() {
        debug!("Pager disabled: stdout is not a TTY");
        return false;
    }

    // Check if explicitly disabled in config
    if config.disabled {
        debug!("Pager disabled by configuration");
        return false;
    }

    // Auto-detection is enabled by config
    if !config.auto_detect {
        debug!("Pager enabled (auto-detect disabled, using unconditionally)");
        return true;
    }

    // If we get here, paging should be used
    debug!("Pager should be used");
    true
}

/// Get terminal dimensions for smart paging decisions
pub fn get_terminal_size() -> Option<(u16, u16)> {
    use crossterm::terminal;

    terminal::size().ok()
}

/// Check if output should be paged based on content size
///
/// This is used for the adaptive buffering strategy:
/// - Buffer first N lines
/// - If < terminal height: skip pager
/// - If >= terminal height: use pager
pub fn should_page_content(line_count: usize, min_lines: usize) -> bool {
    // If content is smaller than minimum threshold, skip pager
    if line_count < min_lines {
        debug!(line_count, min_lines, "Content too small, skipping pager");
        return false;
    }

    // If we can detect terminal size, compare against that
    if let Some((_width, height)) = get_terminal_size() {
        let terminal_lines = height as usize;

        if line_count < terminal_lines {
            debug!(
                line_count,
                terminal_lines, "Content fits on screen, skipping pager"
            );
            return false;
        }
    }

    // Content is large enough to page
    debug!(line_count, "Content should be paged");
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_pager_env() {
        // SAFETY: This is a test function that runs in isolation with #[serial]
        // We're only setting/removing the variable for this test
        unsafe {
            env::set_var("NO_PAGER", "1");
        }
        let config = PagerConfig::default();
        assert!(!should_use_pager(&config));
        unsafe {
            env::remove_var("NO_PAGER");
        }
    }

    #[test]
    fn test_disabled_config() {
        let config = PagerConfig {
            disabled: true,
            ..Default::default()
        };
        assert!(!should_use_pager(&config));
    }

    #[test]
    fn test_terminal_size() {
        // This might fail in CI without a TTY, but should work in dev
        let size = get_terminal_size();
        // Just verify it doesn't panic
        debug!("Terminal size: {:?}", size);
    }
}
