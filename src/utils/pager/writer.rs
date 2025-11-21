use anyhow::{Context, Result};
use command_group::GroupChild;
use std::io::{self, BufWriter, Stdout, Write};
use std::process::{ChildStdin, ExitStatus};
use tracing::{debug, info};

/// Trait for anything that can receive pager output
pub trait PagerWriter: Write + Send {
    /// Check if the pager is still running/accepting input
    fn is_alive(&mut self) -> bool;

    /// Finish writing and wait for pager to exit
    ///
    /// # Errors
    ///
    /// Returns an error if the pager process fails or if writing fails
    fn finish(self: Box<Self>) -> Result<ExitStatus>;
}

/// Main pager struct that wraps a writer
pub struct Pager {
    /// The underlying writer implementation (pager process or direct output).
    pub(crate) writer: Box<dyn PagerWriter>,
}

impl Pager {
    /// Creates a new pager builder for configuring the pager.
    #[must_use]
    pub const fn builder() -> super::PagerBuilder {
        super::PagerBuilder::new()
    }

    /// Get a mutable reference to the writer
    pub fn writer(&mut self) -> &mut dyn PagerWriter {
        &mut *self.writer
    }

    /// Finish writing and wait for pager to complete
    ///
    /// # Errors
    ///
    /// Returns an error if the pager process fails or if writing fails
    pub fn finish(self) -> Result<()> {
        let status = self.writer.finish()?;
        debug!(exit_code = ?status.code(), "Pager finished");
        Ok(())
    }
}

/// Pager process implementation - pipes output to actual pager process
pub struct PagerProcess {
    /// Process group handle for the pager process.
    group: GroupChild,
    /// Buffered writer to the pager's stdin (64KB buffer).
    stdin: BufWriter<ChildStdin>,
    /// Tracks whether the pager process is still running.
    alive: bool,
}

impl PagerProcess {
    /// Creates a new pager process writer with the given process handle and stdin.
    pub(crate) fn new(group: GroupChild, stdin: ChildStdin) -> Self {
        Self {
            group,
            stdin: BufWriter::with_capacity(64 * 1024, stdin), // 64KB buffer
            alive: true,
        }
    }
}

impl Write for PagerProcess {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if !self.alive {
            return Ok(0); // Silently ignore writes to dead pager
        }

        match self.stdin.write(buf) {
            Ok(n) => Ok(n),
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
                debug!("Broken pipe detected, marking pager as dead");
                self.alive = false;
                Ok(0) // Pretend write succeeded to avoid propagating error
            }
            Err(e) => Err(e),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stdin.flush()
    }
}

impl PagerWriter for PagerProcess {
    fn is_alive(&mut self) -> bool {
        if !self.alive {
            return false;
        }

        // Non-blocking check if pager process still running
        match self.group.try_wait() {
            Ok(Some(_status)) => {
                debug!("Pager process has exited");
                self.alive = false;
                false
            }
            Ok(None) => true, // Still running
            Err(e) => {
                debug!(error = %e, "Error checking pager status, assuming dead");
                self.alive = false;
                false
            }
        }
    }

    fn finish(mut self: Box<Self>) -> Result<ExitStatus> {
        // Flush any remaining output
        self.flush().context("Failed to flush pager output")?;

        // stdin will be automatically dropped here, closing the pipe

        // Wait for pager process to exit
        let status = self
            .group
            .wait()
            .context("Failed to wait for pager process")?;

        info!(exit_code = ?status.code(), "Pager process completed");
        Ok(status)
    }
}

impl Drop for PagerProcess {
    fn drop(&mut self) {
        // Best effort cleanup
        let _ = self.stdin.flush();
        let _ = self.group.kill();
    }
}

/// Direct output implementation - no paging, writes directly to stdout
pub struct DirectOutput {
    /// Buffered writer to stdout (64KB buffer).
    stdout: BufWriter<Stdout>,
}

impl DirectOutput {
    /// Creates a new direct output writer to stdout.
    pub fn new() -> Self {
        Self {
            stdout: BufWriter::with_capacity(64 * 1024, io::stdout()),
        }
    }
}

impl Write for DirectOutput {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stdout.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stdout.flush()
    }
}

impl PagerWriter for DirectOutput {
    fn is_alive(&mut self) -> bool {
        true // Stdout is always "alive"
    }

    fn finish(mut self: Box<Self>) -> Result<ExitStatus> {
        self.flush().context("Failed to flush stdout")?;

        // Create a dummy successful exit status
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
    }
}
