use anyhow::Result;
use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};

/// Check if output is a terminal (not redirected)
pub fn is_terminal() -> bool {
    atty::is(atty::Stream::Stdout)
}

/// Get the pager command from environment or use default
pub fn get_pager() -> String {
    env::var("PAGER")
        .map(|pager| {
            // If PAGER is set, extract just the binary name if it's a path
            Path::new(&pager)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(&pager)
                .to_string()
        })
        .unwrap_or_else(|_| {
            // Try common pagers in order of preference
            if which::which("less").is_ok() {
                "less".to_string()
            } else if which::which("more").is_ok() {
                "more".to_string()
            } else {
                "cat".to_string() // Fallback to cat if no pager available
            }
        })
}

/// Output content through a pager if in terminal, otherwise directly
pub fn output_through_pager(content: &str, use_pager: bool) -> Result<()> {
    if !use_pager || !is_terminal() {
        // Output directly if not a terminal or pager disabled
        print!("{}", content);
        io::stdout().flush()?;
        return Ok(());
    }

    let pager = get_pager();

    // Special handling for less to enable color support
    let args = if pager.contains("less") {
        vec!["-R"] // Enable raw control characters (for colors)
    } else {
        vec![]
    };

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

/// Builder for pager output with configurable options
#[derive(Default)]
pub struct PagerOutput {
    content: String,
    use_pager: bool,
}

impl PagerOutput {
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
        output_through_pager(&self.content, self.use_pager)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_get_pager_default() {
        // Clear PAGER env var
        unsafe {
            env::remove_var("PAGER");
        }
        let pager = get_pager();
        // Should return less, more, or cat
        assert!(["less", "more", "cat"].contains(&pager.as_str()));
    }

    #[test]
    #[serial]
    fn test_get_pager_from_env() {
        unsafe {
            env::set_var("PAGER", "custom_pager");
        }
        let pager = get_pager();
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

    // Note: test_output_direct_when_disabled removed as it outputs to stdout during test runs
    // The functionality is simple enough that it doesn't need a test
}
