use anyhow::{Context, Result};
use content_inspector::{ContentType, inspect};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use tracing::{Level, debug, span};

/// Check if a file is binary by inspecting its content.
///
/// Reads the first 8KB of the file and uses content inspection
/// to determine if it's binary or text. This is much faster and
/// more accurate than checking for null bytes manually.
///
/// # Arguments
///
/// * `path` - Path to the file to check
///
/// # Returns
///
/// # Errors
///
/// * `Ok(true)` if the file is binary
/// * `Ok(false)` if the file is text/UTF-8
/// * `Err` if the file cannot be read
pub fn is_binary_file(path: &Path) -> Result<bool> {
    let span = span!(Level::DEBUG, "binary_check", path = %path.display());
    let _guard = span.enter();

    let mut file = File::open(path)
        .with_context(|| format!("Failed to open file for binary check: {}", path.display()))?;

    let mut buffer = [0u8; 8192]; // Check first 8KB
    let n = file
        .read(&mut buffer)
        .with_context(|| format!("Failed to read file for binary check: {}", path.display()))?;

    if n == 0 {
        // Empty file is considered text
        debug!("File is empty, treating as text");
        return Ok(false);
    }

    let is_binary = matches!(inspect(&buffer[..n]), ContentType::BINARY);

    debug!(is_binary, bytes_checked = n, "Binary detection complete");

    Ok(is_binary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_text_file() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(file, "This is a text file")?;
        writeln!(file, "With multiple lines")?;

        let is_binary = is_binary_file(file.path())?;
        assert!(!is_binary, "Text file should not be detected as binary");

        Ok(())
    }

    #[test]
    fn test_binary_file() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        // Write some binary data
        file.write_all(&[0xFF, 0x00, 0xAA, 0xBB, 0xCC, 0xDD])?;

        let is_binary = is_binary_file(file.path())?;
        assert!(is_binary, "Binary file should be detected as binary");

        Ok(())
    }

    #[test]
    fn test_empty_file() -> Result<()> {
        let file = NamedTempFile::new()?;
        // Don't write anything

        let is_binary = is_binary_file(file.path())?;
        assert!(!is_binary, "Empty file should be treated as text");

        Ok(())
    }

    #[test]
    fn test_utf8_file() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(file, "Hello 世界 🌍")?; // Unicode content

        let is_binary = is_binary_file(file.path())?;
        assert!(!is_binary, "UTF-8 file should not be detected as binary");

        Ok(())
    }
}
