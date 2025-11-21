use anyhow::{Context, Result, bail};
use shell_words;
use tracing::{Level, debug, span};
use which::which;

/// Parse a pager command string into program and arguments
///
/// Handles shell-like quoting:
/// - Single quotes: 'value with spaces'
/// - Double quotes: "value with spaces"
/// - Escaping: value\ with\ spaces
/// - Nested quotes: "value with 'nested' quotes"
///
/// Examples:
/// - `"less -FRX"` → `("less", ["-FRX"])`
/// - `"bat --theme='Monokai Extended'"` → `("bat", ["--theme=Monokai Extended"])`
/// - `"less -R '+Gg'"` → `("less", ["-R", "+Gg"])`
pub fn parse_pager_command(cmd: &str) -> Result<(String, Vec<String>)> {
    let span = span!(Level::DEBUG, "parse_pager_command", cmd);
    let _guard = span.enter();

    // Use shell-words crate for proper shell-like parsing
    let parts = shell_words::split(cmd)
        .with_context(|| format!("Invalid pager command syntax: '{cmd}'"))?;

    if parts.is_empty() {
        bail!("Empty pager command");
    }

    let program = parts[0].clone();
    let args = parts[1..].to_vec();

    debug!(program = %program, args = ?args, "Pager command parsed");

    // Validate that the program exists in PATH
    validate_pager_program(&program)?;

    Ok((program, args))
}

/// Validate that a pager program exists in PATH
fn validate_pager_program(program: &str) -> Result<()> {
    which(program).with_context(|| format!("Pager program '{program}' not found in PATH"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_command() {
        let (prog, args) = parse_pager_command("less").unwrap();
        assert_eq!(prog, "less");
        assert_eq!(args, Vec::<String>::new());
    }

    #[test]
    fn test_parse_command_with_args() {
        let (prog, args) = parse_pager_command("less -FRX").unwrap();
        assert_eq!(prog, "less");
        assert_eq!(args, vec!["-FRX"]);
    }

    #[test]
    fn test_parse_command_with_quoted_args() {
        // Note: This test will only pass if 'bat' is in PATH
        // In a real test environment, we'd mock the 'which' check
        if which::which("bat").is_ok() {
            let (prog, args) = parse_pager_command("bat --theme='Monokai Extended'").unwrap();
            assert_eq!(prog, "bat");
            assert_eq!(args, vec!["--theme=Monokai Extended"]);
        }
    }

    #[test]
    fn test_parse_empty_command() {
        let result = parse_pager_command("");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Empty pager command")
        );
    }

    #[test]
    fn test_parse_invalid_syntax() {
        let result = parse_pager_command("less 'unclosed quote");
        assert!(result.is_err());
    }
}
