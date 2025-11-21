use crate::DotmanContext;
use std::env;
use tracing::{Level, debug, span};

/// Configuration for pager behavior.
#[derive(Debug, Clone)]
pub struct PagerConfig {
    /// Command to use for paging (e.g., "less -R")
    pub command: String,
    /// Whether the pager is explicitly disabled
    pub disabled: bool,
    /// Minimum number of lines before paging is used
    pub min_lines: usize,
    /// Whether to auto-detect if paging should be used based on output size
    pub auto_detect: bool,
}

impl Default for PagerConfig {
    fn default() -> Self {
        Self {
            command: detect_best_pager(),
            disabled: false,
            min_lines: 20,
            auto_detect: true,
        }
    }
}

impl PagerConfig {
    /// Create pager configuration from context for a specific command
    pub fn from_context(ctx: &DotmanContext, command_name: &str) -> Self {
        let span = span!(Level::DEBUG, "resolve_pager_config", command = command_name);
        let _guard = span.enter();

        let command = resolve_pager_command(ctx, command_name);
        let disabled = ctx.no_pager || is_command_disabled(ctx, command_name);

        debug!(command = %command, disabled, "Pager config resolved");

        Self {
            command,
            disabled,
            min_lines: ctx
                .config
                .pager
                .as_ref()
                .and_then(|p| p.min_lines)
                .unwrap_or(20),
            auto_detect: ctx
                .config
                .pager
                .as_ref()
                .and_then(|p| p.auto)
                .unwrap_or(true),
        }
    }
}

/// Resolve pager command using 9-level precedence hierarchy
fn resolve_pager_command(ctx: &DotmanContext, cmd_name: &str) -> String {
    // 1. Command-specific environment variable (e.g., DOT_DIFF_PAGER)
    let env_key = format!("DOT_{}_PAGER", cmd_name.to_uppercase());
    if let Ok(pager) = env::var(&env_key) {
        debug!(source = "env_specific", pager = %pager, "Pager command resolved");
        return pager;
    }

    // 2. DOT_PAGER environment variable
    if let Ok(pager) = env::var("DOT_PAGER") {
        debug!(source = "DOT_PAGER", pager = %pager, "Pager command resolved");
        return pager;
    }

    // 3. GIT_PAGER environment variable (Git interoperability!)
    if let Ok(pager) = env::var("GIT_PAGER") {
        debug!(source = "GIT_PAGER", pager = %pager, "Pager command resolved");
        return pager;
    }

    // 4. PAGER environment variable (standard Unix)
    if let Ok(pager) = env::var("PAGER") {
        debug!(source = "PAGER", pager = %pager, "Pager command resolved");
        return pager;
    }

    // 5. Config: command-specific pager (e.g., pager.diff_pager)
    if let Some(ref pager_config) = ctx.config.pager {
        let command_pager = match cmd_name {
            "diff" => &pager_config.diff_pager,
            "log" => &pager_config.log_pager,
            _ => &None,
        };

        if let Some(pager) = command_pager {
            debug!(source = "config_specific", pager = %pager, "Pager command resolved");
            return pager.clone();
        }
    }

    // 6. Config: core.pager
    if let Some(ref pager) = ctx.config.core.pager {
        debug!(source = "core.pager", pager = %pager, "Pager command resolved");
        return pager.clone();
    }

    // 7. Smart detection (modern → traditional)
    let pager = detect_best_pager();
    debug!(source = "auto_detect", pager = %pager, "Pager command resolved");
    pager
}

/// Check if paging is disabled for a specific command
fn is_command_disabled(ctx: &DotmanContext, cmd_name: &str) -> bool {
    if let Some(ref pager_config) = ctx.config.pager {
        let enabled = match cmd_name {
            "diff" => pager_config.diff,
            "log" => pager_config.log,
            "show" => pager_config.show,
            "branch" => pager_config.branch,
            "status" => pager_config.status,
            _ => Some(true), // Default: enabled
        };

        // If explicitly set to false, paging is disabled
        if enabled == Some(false) {
            debug!(command = cmd_name, "Paging disabled by config");
            return true;
        }
    }

    false
}

/// Auto-detect best available pager (modern → traditional fallback)
fn detect_best_pager() -> String {
    use which::which;

    // Try modern pagers first (better UX)
    if which("delta").is_ok() {
        return "delta".to_string();
    }

    if which("bat").is_ok() {
        return "bat --paging=always --style=plain".to_string();
    }

    if which("moar").is_ok() {
        return "moar".to_string();
    }

    // Fall back to traditional pagers
    if which("less").is_ok() {
        return "less -FRX".to_string();
    }

    if which("more").is_ok() {
        return "more".to_string();
    }

    // Last resort: just cat
    "cat".to_string()
}
