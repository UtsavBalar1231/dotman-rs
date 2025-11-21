//! Adaptive pager writer that buffers output and dynamically chooses backend.
//!
//! The adaptive writer implements intelligent paging by:
//! 1. Buffering initial output (configurable threshold)
//! 2. Counting lines as they're written
//! 3. At the threshold, checking terminal size and content size
//! 4. Deciding whether to spawn a pager or use direct output
//! 5. Flushing the buffer to the chosen backend

use super::PagerConfig;
use super::process::{should_page_content, spawn_pager};
use super::writer::{DirectOutput, PagerWriter};
use anyhow::Result;
use std::io::{self, Write};
use std::process::ExitStatus;
use tracing::{debug, info};

/// Buffered lines collected before making paging decision
struct LineBuffer {
    /// Buffered lines collected before paging decision.
    lines: Vec<Vec<u8>>,
    /// Total bytes written to the buffer.
    total_bytes: usize,
}

impl LineBuffer {
    /// Creates a new line buffer with initial capacity for 100 lines.
    fn new() -> Self {
        Self {
            lines: Vec::with_capacity(100),
            total_bytes: 0,
        }
    }

    /// Adds a line to the buffer and updates the total byte count.
    fn push(&mut self, line: Vec<u8>) {
        self.total_bytes += line.len();
        self.lines.push(line);
    }

    /// Returns the number of lines currently buffered.
    const fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Writes all buffered lines to the specified writer.
    fn flush_to(&self, writer: &mut dyn Write) -> io::Result<()> {
        for line in &self.lines {
            writer.write_all(line)?;
        }
        Ok(())
    }
}

/// Adaptive writer that delays pager decision until content size is known
pub struct AdaptiveWriter {
    /// Configuration for pager behavior.
    config: PagerConfig,
    /// Line buffer for collecting output before making paging decision.
    buffer: LineBuffer,
    /// The chosen backend (pager or direct output) once decision is made.
    backend: Option<Box<dyn PagerWriter>>,
    /// Current incomplete line being assembled from write calls.
    current_line: Vec<u8>,
    /// Number of lines to buffer before deciding whether to page.
    decision_threshold: usize,
}

impl AdaptiveWriter {
    /// Create a new adaptive writer with the given configuration
    pub fn new(config: PagerConfig) -> Self {
        let decision_threshold = config.min_lines;

        Self {
            config,
            buffer: LineBuffer::new(),
            backend: None,
            current_line: Vec::with_capacity(256),
            decision_threshold,
        }
    }

    /// Make the paging decision based on buffered content
    fn make_decision(&mut self) -> Result<()> {
        let line_count = self.buffer.line_count();

        debug!(
            line_count,
            threshold = self.decision_threshold,
            "Making paging decision"
        );

        // Check if we should page based on content size and terminal size
        let use_pager = should_page_content(line_count, self.config.min_lines);

        let backend: Box<dyn PagerWriter> = if use_pager {
            info!("Content requires paging, spawning pager process");
            Box::new(spawn_pager(&self.config.command)?)
        } else {
            debug!("Content fits on screen, using direct output");
            Box::new(DirectOutput::new())
        };

        self.backend = Some(backend);
        Ok(())
    }

    /// Ensure we have a backend selected, making decision if needed
    fn ensure_backend(&mut self) -> Result<&mut Box<dyn PagerWriter>> {
        if self.backend.is_none() {
            // Haven't reached threshold yet, but backend needed
            // Make decision with current buffer
            self.make_decision()?;

            // Flush buffered content to chosen backend
            if let Some(ref mut backend) = self.backend {
                self.buffer
                    .flush_to(&mut **backend)
                    .map_err(|e| anyhow::anyhow!("Failed to flush buffer: {e}"))?;
            }
        }

        self.backend
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Backend not initialized"))
    }

    /// Process a complete line (called when newline detected)
    fn process_line(&mut self, line: Vec<u8>) -> io::Result<()> {
        if self.backend.is_some() {
            // Decision already made, write directly to backend
            if let Some(ref mut backend) = self.backend {
                backend.write_all(&line)?;
            }
        } else {
            // Still buffering - add to buffer
            self.buffer.push(line);

            // Check if we've hit the decision threshold
            if self.buffer.line_count() >= self.decision_threshold {
                debug!(
                    lines = self.buffer.line_count(),
                    "Reached decision threshold"
                );

                // Make paging decision
                self.make_decision()
                    .map_err(|e| io::Error::other(e.to_string()))?;

                // Flush buffer to chosen backend
                if let Some(ref mut backend) = self.backend {
                    self.buffer.flush_to(&mut **backend)?;
                }
            }
        }

        Ok(())
    }
}

impl Write for AdaptiveWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut bytes_written = 0;

        for &byte in buf {
            self.current_line.push(byte);
            bytes_written += 1;

            // Check for newline
            if byte == b'\n' {
                let line = std::mem::replace(&mut self.current_line, Vec::with_capacity(256));
                self.process_line(line)?;
            }
        }

        Ok(bytes_written)
    }

    fn flush(&mut self) -> io::Result<()> {
        // Flush any remaining partial line
        if !self.current_line.is_empty() {
            let line = std::mem::take(&mut self.current_line);
            self.process_line(line)?;
        }

        // Flush backend if it exists
        if let Some(ref mut backend) = self.backend {
            backend.flush()?;
        }

        Ok(())
    }
}

impl PagerWriter for AdaptiveWriter {
    fn is_alive(&mut self) -> bool {
        self.backend
            .as_mut()
            .is_none_or(|backend| backend.is_alive())
    }

    fn finish(mut self: Box<Self>) -> Result<ExitStatus> {
        // Ensure backend is initialized
        self.ensure_backend()?;

        // Flush any remaining content
        self.flush()
            .map_err(|e| anyhow::anyhow!("Failed to flush before finish: {e}"))?;

        // Finish the backend
        self.backend.map_or_else(
            || {
                // This shouldn't happen, but return success if it does
                #[cfg(unix)]
                {
                    use std::os::unix::process::ExitStatusExt;
                    Ok(ExitStatus::from_raw(0))
                }

                #[cfg(windows)]
                {
                    use std::os::windows::process::ExitStatusExt;
                    Ok(ExitStatus::from_raw(0))
                }
            },
            super::writer::PagerWriter::finish,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_output_no_pager() -> Result<()> {
        let config = PagerConfig {
            command: "less -R".to_string(),
            disabled: false,
            min_lines: 20,
            auto_detect: true,
        };

        let writer = AdaptiveWriter::new(config);

        // Box it to match PagerWriter trait
        let mut boxed_writer: Box<dyn PagerWriter> = Box::new(writer);

        // Write just a few lines
        writeln!(boxed_writer, "Line 1")?;
        writeln!(boxed_writer, "Line 2")?;
        writeln!(boxed_writer, "Line 3")?;

        // Finish triggers the decision
        let status = boxed_writer.finish()?;

        // Should succeed
        assert!(status.success() || status.code() == Some(0));

        Ok(())
    }

    #[test]
    fn test_large_output_triggers_decision() -> Result<()> {
        let config = PagerConfig {
            command: "cat".to_string(), // Use cat for testing
            disabled: false,
            min_lines: 5,
            auto_detect: true,
        };

        let writer = AdaptiveWriter::new(config);
        let mut boxed_writer: Box<dyn PagerWriter> = Box::new(writer);

        // Write more than threshold lines
        for i in 0..10 {
            writeln!(boxed_writer, "Line {i}")?;
        }

        // Finish and check success
        let status = boxed_writer.finish()?;
        assert!(status.success() || status.code() == Some(0));

        Ok(())
    }
}
