use anyhow::Result;
use colored::Colorize;
use similar::{Algorithm, ChangeTag, TextDiff};
use std::io::Write;
use std::path::Path;
use tracing::{Level, info, span};

/// Configuration for unified diff generation
pub struct UnifiedDiffConfig {
    /// Number of context lines around changes (typically 3)
    pub context_lines: usize,
    /// Diff algorithm to use (Myers, Patience, Histogram)
    pub algorithm: Algorithm,
    /// Whether to colorize the output
    pub colorize: bool,
}

/// Generate a unified diff between two file contents.
///
/// Produces Git-style unified diff output with:
/// - File headers (`--- a/path` and `+++ b/path`)
/// - Hunk headers (`@@ -10,7 +10,9 @@`)
/// - Context lines (unchanged, prefixed with space)
/// - Deletion lines (prefixed with `-`, colored red)
/// - Addition lines (prefixed with `+`, colored green)
///
/// # Arguments
///
/// * `old_content` - Content of the old/original file
/// * `new_content` - Content of the new/modified file
/// * `old_path` - Path to display for old file
/// * `new_path` - Path to display for new file
/// * `config` - Diff configuration (context lines, algorithm, colorization)
/// * `writer` - Output writer to write diff to
///
/// # Errors
///
/// Returns an error if writing to the output writer fails.
pub fn generate_unified_diff(
    old_content: &str,
    new_content: &str,
    old_path: &Path,
    new_path: &Path,
    config: &UnifiedDiffConfig,
    writer: &mut dyn Write,
) -> Result<()> {
    let span = span!(
        Level::DEBUG,
        "diff_generation",
        path = %new_path.display(),
        algorithm = ?config.algorithm,
        context = config.context_lines
    );
    let _guard = span.enter();

    // Create diff with specified algorithm
    let diff = TextDiff::configure()
        .algorithm(config.algorithm)
        .diff_lines(old_content, new_content);

    // Git-style file headers
    let old_header = format!("--- a/{}", old_path.display());
    let new_header = format!("+++ b/{}", new_path.display());

    if config.colorize {
        writeln!(writer, "{}", old_header.red())?;
        writeln!(writer, "{}", new_header.green())?;
    } else {
        writeln!(writer, "{old_header}")?;
        writeln!(writer, "{new_header}")?;
    }

    let mut total_changes = 0;

    // Generate hunks with context
    for hunk in diff
        .unified_diff()
        .context_radius(config.context_lines)
        .iter_hunks()
    {
        // Hunk header (e.g., "@@ -10,7 +10,9 @@")
        let hunk_header = hunk.header().to_string();

        if config.colorize {
            writeln!(writer, "{}", hunk_header.cyan())?;
        } else {
            writeln!(writer, "{hunk_header}")?;
        }

        // Process changes in this hunk
        for change in hunk.iter_changes() {
            total_changes += 1;

            let (_prefix, content) = match change.tag() {
                ChangeTag::Delete => {
                    let line = format!("-{change}");
                    if config.colorize {
                        ("-".to_string(), line.red().to_string())
                    } else {
                        ("-".to_string(), line)
                    }
                }
                ChangeTag::Insert => {
                    let line = format!("+{change}");
                    if config.colorize {
                        ("+".to_string(), line.green().to_string())
                    } else {
                        ("+".to_string(), line)
                    }
                }
                ChangeTag::Equal => {
                    let line = format!(" {change}");
                    (" ".to_string(), line)
                }
            };

            // Write without prefix since we already included it
            write!(writer, "{content}")?;

            // Add newline if the change doesn't end with one
            if !content.ends_with('\n') {
                writeln!(writer)?;
            }
        }
    }

    info!(
        path = %new_path.display(),
        changes = total_changes,
        "Diff generation complete"
    );

    Ok(())
}

/// Generate a simple "Binary files differ" message for binary files.
///
/// # Errors
///
/// Returns an error if writing to the output fails.
pub fn generate_binary_diff_message(
    old_path: &Path,
    new_path: &Path,
    writer: &mut dyn Write,
) -> Result<()> {
    writeln!(
        writer,
        "Binary files {} and {} differ",
        old_path.display(),
        new_path.display()
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_unified_diff_simple() -> Result<()> {
        let old = "line1\nline2\nline3\n";
        let new = "line1\nmodified\nline3\n";

        let mut output = Vec::new();
        let config = UnifiedDiffConfig {
            context_lines: 3,
            algorithm: Algorithm::Myers,
            colorize: false,
        };

        generate_unified_diff(
            old,
            new,
            &PathBuf::from("old.txt"),
            &PathBuf::from("new.txt"),
            &config,
            &mut output,
        )?;

        let result = String::from_utf8(output)?;

        assert!(result.contains("--- a/old.txt"));
        assert!(result.contains("+++ b/new.txt"));
        assert!(result.contains("@@"));
        assert!(result.contains("-line2"));
        assert!(result.contains("+modified"));

        Ok(())
    }

    #[test]
    fn test_unified_diff_no_changes() -> Result<()> {
        let content = "line1\nline2\nline3\n";

        let mut output = Vec::new();
        let config = UnifiedDiffConfig {
            context_lines: 3,
            algorithm: Algorithm::Myers,
            colorize: false,
        };

        generate_unified_diff(
            content,
            content,
            &PathBuf::from("file.txt"),
            &PathBuf::from("file.txt"),
            &config,
            &mut output,
        )?;

        let result = String::from_utf8(output)?;

        // Should have headers but no hunks
        assert!(result.contains("--- a/file.txt"));
        assert!(result.contains("+++ b/file.txt"));
        assert!(!result.contains("@@")); // No hunks

        Ok(())
    }

    #[test]
    fn test_binary_diff_message() -> Result<()> {
        let mut output = Vec::new();

        generate_binary_diff_message(
            &PathBuf::from("old.bin"),
            &PathBuf::from("new.bin"),
            &mut output,
        )?;

        let result = String::from_utf8(output)?;

        assert!(result.contains("Binary files"));
        assert!(result.contains("old.bin"));
        assert!(result.contains("new.bin"));
        assert!(result.contains("differ"));

        Ok(())
    }
}
