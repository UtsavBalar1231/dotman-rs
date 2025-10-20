use anyhow::Result;
use std::env;
use std::io::{self, IsTerminal, Write};
use std::process::{Command, Stdio};

/// Get the pager command using Git's priority order
#[must_use]
pub fn get_pager(ctx: Option<&crate::DotmanContext>) -> String {
    // 1. Check DOT_PAGER environment variable
    if let Ok(pager) = env::var("DOT_PAGER") {
        return pager;
    }

    // 2. Check core.pager config (if context available)
    if let Some(ctx) = ctx
        && let Some(pager) = ctx.config.core.pager.as_ref()
    {
        return pager.clone();
    }

    // 3. Check PAGER environment variable
    if let Ok(pager) = env::var("PAGER") {
        return pager;
    }

    // 4. Default to less with Git-style flags
    if which::which("less").is_ok() {
        "less".to_string()
    } else if which::which("more").is_ok() {
        "more".to_string()
    } else {
        "cat".to_string() // Fallback to cat if no pager available
    }
}

/// Output content through a pager if appropriate, with Git-style behavior
///
/// # Errors
///
/// Returns an error if:
/// - Failed to spawn the pager process
/// - Failed to write to the pager's stdin
/// - Failed to flush stdout
pub fn output_through_pager(
    content: &str,
    use_pager: bool,
    ctx: Option<&crate::DotmanContext>,
) -> Result<()> {
    // Check NO_PAGER environment variable
    if env::var("NO_PAGER").is_ok() {
        print!("{content}");
        io::stdout().flush()?;
        return Ok(());
    }

    // Skip pager if disabled or not a terminal
    if !use_pager || !io::stdout().is_terminal() {
        print!("{content}");
        io::stdout().flush()?;
        return Ok(());
    }

    // Count lines to determine if pager is needed
    let line_count = content.lines().count();
    let terminal_height = get_terminal_height();

    // Skip pager if content fits on one screen (like Git's -F flag)
    if line_count < terminal_height.saturating_sub(1) {
        print!("{content}");
        io::stdout().flush()?;
        return Ok(());
    }

    let pager_cmd = get_pager(ctx);
    let (pager, args) = parse_pager_command(&pager_cmd);

    // Set LESS environment variable if not set and using less
    if pager == "less" && env::var("LESS").is_err() {
        unsafe {
            env::set_var("LESS", "FRX");
        }
    }

    // Try to spawn the pager
    if let Ok(mut child) = Command::new(&pager)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
    {
        // Write content to pager's stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(content.as_bytes())?;
            stdin.flush()?;
        }

        // Wait for pager to finish
        child.wait()?;
    } else {
        // Fallback to direct output if pager fails
        print!("{content}");
        io::stdout().flush()?;
    }

    Ok(())
}

/// Parse pager command string into command and args
fn parse_pager_command(pager_cmd: &str) -> (String, Vec<String>) {
    let parts: Vec<&str> = pager_cmd.split_whitespace().collect();
    if parts.is_empty() {
        return ("less".to_string(), vec!["-FRX".to_string()]);
    }

    let pager = parts[0].to_string();
    let mut args: Vec<String> = parts[1..].iter().map(|s| (*s).to_string()).collect();

    // Add default flags for less if no args provided
    if pager == "less" && args.is_empty() {
        args.push("-FRX".to_string());
    }

    (pager, args)
}

/// Get terminal height, defaulting to 24 if unknown
fn get_terminal_height() -> usize {
    if let Some((_, height)) = terminal_size::terminal_size() {
        height.0 as usize
    } else {
        24 // Default terminal height
    }
}

/// Builder for pager output with configurable options
pub struct PagerOutput<'a> {
    /// Accumulated content to be displayed
    content: String,
    /// Whether to use a pager for output
    use_pager: bool,
    /// Optional context for accessing configuration
    ctx: Option<&'a crate::DotmanContext>,
}

impl Default for PagerOutput<'_> {
    fn default() -> Self {
        Self {
            content: String::new(),
            use_pager: true, // Default to enabled like Git
            ctx: None,
        }
    }
}

impl<'a> PagerOutput<'a> {
    /// Create a new pager output builder with the given context
    ///
    /// # Arguments
    ///
    /// * `ctx` - Dotman context for accessing configuration
    /// * `no_pager` - If true, disables pager output
    #[must_use]
    pub const fn new(ctx: &'a crate::DotmanContext, no_pager: bool) -> Self {
        Self {
            content: String::new(),
            use_pager: !no_pager,
            ctx: Some(ctx),
        }
    }

    /// Set the content to be displayed
    ///
    /// # Arguments
    ///
    /// * `content` - The content string to display
    #[must_use]
    pub fn with_content(mut self, content: String) -> Self {
        self.content = content;
        self
    }

    /// Append text to the accumulated content
    ///
    /// # Arguments
    ///
    /// * `text` - Text to append (no newline added)
    pub fn append(&mut self, text: &str) {
        self.content.push_str(text);
    }

    /// Append text with a newline to the accumulated content
    ///
    /// # Arguments
    ///
    /// * `text` - Text to append (newline will be added)
    pub fn appendln(&mut self, text: &str) {
        self.content.push_str(text);
        self.content.push('\n');
    }

    /// Disable pager output, forcing direct output to stdout
    #[must_use]
    pub const fn disable_pager(mut self) -> Self {
        self.use_pager = false;
        self
    }

    /// Display the accumulated content through the configured pager
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to output content through the pager
    pub fn show(self) -> Result<()> {
        output_through_pager(&self.content, self.use_pager, self.ctx)
    }
}
