use anyhow::Result;
use std::env;
use std::io::{self, IsTerminal, Write};
use std::process::{Command, Stdio};

/// Get the pager command using Git's priority order
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
pub fn output_through_pager(
    content: &str,
    use_pager: bool,
    ctx: Option<&crate::DotmanContext>,
) -> Result<()> {
    // Check NO_PAGER environment variable
    if env::var("NO_PAGER").is_ok() {
        print!("{}", content);
        io::stdout().flush()?;
        return Ok(());
    }

    // Skip pager if disabled or not a terminal
    if !use_pager || !io::stdout().is_terminal() {
        print!("{}", content);
        io::stdout().flush()?;
        return Ok(());
    }

    // Count lines to determine if pager is needed
    let line_count = content.lines().count();
    let terminal_height = get_terminal_height();

    // Skip pager if content fits on one screen (like Git's -F flag)
    if line_count < terminal_height.saturating_sub(1) {
        print!("{}", content);
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
    match Command::new(&pager)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(mut child) => {
            // Write content to pager's stdin
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(content.as_bytes())?;
                stdin.flush()?;
            }

            // Wait for pager to finish
            child.wait()?;
        }
        Err(_) => {
            // Fallback to direct output if pager fails
            print!("{}", content);
            io::stdout().flush()?;
        }
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
    let mut args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

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
pub struct PagerOutput {
    content: String,
    use_pager: bool,
    ctx: Option<*const crate::DotmanContext>,
}

impl Default for PagerOutput {
    fn default() -> Self {
        Self {
            content: String::new(),
            use_pager: true, // Default to enabled like Git
            ctx: None,
        }
    }
}

impl PagerOutput {
    pub fn new(ctx: &crate::DotmanContext, no_pager: bool) -> Self {
        Self {
            content: String::new(),
            use_pager: !no_pager,
            ctx: Some(ctx as *const crate::DotmanContext),
        }
    }

    pub fn with_content(mut self, content: String) -> Self {
        self.content = content;
        self
    }

    pub fn append(&mut self, text: &str) {
        self.content.push_str(text);
    }

    pub fn appendln(&mut self, text: &str) {
        self.content.push_str(text);
        self.content.push('\n');
    }

    pub fn disable_pager(mut self) -> Self {
        self.use_pager = false;
        self
    }

    pub fn show(self) -> Result<()> {
        let ctx = self.ctx.map(|ctx_ptr| unsafe { &*ctx_ptr });
        output_through_pager(&self.content, self.use_pager, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_get_pager_default() {
        // Clear environment variables
        unsafe {
            env::remove_var("DOT_PAGER");
            env::remove_var("PAGER");
        }
        let pager = get_pager(None);
        // Should return less, more, or cat
        assert!(["less", "more", "cat"].contains(&pager.as_str()));
    }

    #[test]
    #[serial]
    fn test_get_pager_from_dot_pager() {
        unsafe {
            env::set_var("DOT_PAGER", "dot_custom_pager");
            env::set_var("PAGER", "pager_env");
        }
        let pager = get_pager(None);
        assert_eq!(pager, "dot_custom_pager");
        unsafe {
            env::remove_var("DOT_PAGER");
            env::remove_var("PAGER");
        }
    }

    #[test]
    #[serial]
    fn test_get_pager_from_env() {
        unsafe {
            env::remove_var("DOT_PAGER");
            env::set_var("PAGER", "custom_pager");
        }
        let pager = get_pager(None);
        assert_eq!(pager, "custom_pager");
        unsafe {
            env::remove_var("PAGER");
        }
    }

    #[test]
    fn test_pager_output_builder() {
        let mut output = PagerOutput::default();
        output.append("Line 1");
        output.appendln(" continued");
        output.append("Line 2");

        assert!(output.content.contains("Line 1 continued\n"));
        assert!(output.content.contains("Line 2"));
    }

    #[test]
    fn test_parse_pager_command() {
        // Test simple command
        let (cmd, args) = parse_pager_command("less");
        assert_eq!(cmd, "less");
        assert_eq!(args, vec!["-FRX"]);

        // Test command with args
        let (cmd, args) = parse_pager_command("less -R");
        assert_eq!(cmd, "less");
        assert_eq!(args, vec!["-R"]);

        // Test other pager
        let (cmd, args) = parse_pager_command("more");
        assert_eq!(cmd, "more");
        assert!(args.is_empty());

        // Test complex command
        let (cmd, args) = parse_pager_command("less -F -R -X");
        assert_eq!(cmd, "less");
        assert_eq!(args, vec!["-F", "-R", "-X"]);

        // Test empty command
        let (cmd, args) = parse_pager_command("");
        assert_eq!(cmd, "less");
        assert_eq!(args, vec!["-FRX"]);
    }

    // Note: test_output_direct_when_disabled removed as it outputs to stdout during test runs
    // The functionality is simple enough that it doesn't need a test
}
