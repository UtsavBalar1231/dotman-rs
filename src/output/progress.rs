//! Progress bar implementation for long-running operations.

use colored::Colorize;
use std::io::{self, IsTerminal, Write};

/// A progress bar that updates in place on TTY terminals.
///
/// Shows completion percentage and current/total counts in git style:
/// "Processing commits: 100% (6/6), done."
pub struct Progress {
    /// Title displayed before the progress bar
    title: String,
    /// Total number of items to process
    total: usize,
    /// Current number of items processed
    current: usize,
    /// Whether stderr is a TTY (enables inline updating)
    is_tty: bool,
    /// Last displayed percentage (to avoid redundant updates)
    last_percent: u8,
    /// Whether progress display has started
    started: bool,
}

impl Progress {
    /// Creates a new progress bar with the given title and total items.
    ///
    /// If stderr is a TTY, progress will update inline. Otherwise, it's silent.
    #[must_use]
    pub fn new(title: &str, total: usize) -> Self {
        let is_tty = io::stderr().is_terminal();

        let mut progress = Self {
            title: title.to_string(),
            total,
            current: 0,
            is_tty,
            last_percent: 0,
            started: false,
        };

        if is_tty && total > 0 {
            progress.display();
            progress.started = true;
        }

        progress
    }

    /// Updates the progress bar to the given current position.
    ///
    /// Only redraws if the percentage has changed (to avoid excessive writes).
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    pub fn update(&mut self, current: usize) {
        self.current = current.min(self.total);

        let percent = if self.total > 0 {
            ((self.current as f64 / self.total as f64) * 100.0) as u8
        } else {
            0
        };

        // Always update percentage for testing, but only display on TTY
        if percent != self.last_percent {
            self.last_percent = percent;
            if self.is_tty {
                self.display();
            }
        }
    }

    /// Completes the progress bar and displays final "done" message.
    ///
    /// Consumes self to prevent further updates.
    #[allow(clippy::mem_forget)]
    pub fn finish(mut self) {
        self.current = self.total;
        self.last_percent = 100;

        if self.is_tty && self.started {
            self.display_final();
        }

        std::mem::forget(self);
    }

    /// Displays the current progress state (percentage and count).
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    fn display(&self) {
        if !self.is_tty || self.total == 0 {
            return;
        }

        let percent = (self.current as f64 / self.total as f64) * 100.0;
        eprint!(
            "\r{}: {}% ({}/{})",
            self.title.dimmed(),
            (percent as u8).to_string().dimmed(),
            self.current,
            self.total
        );
        let _ = io::stderr().flush();
    }

    /// Displays the final completion message with "done" suffix.
    fn display_final(&self) {
        eprintln!(
            "\r{}: 100% ({}/{}), done.",
            self.title.dimmed(),
            self.total,
            self.total
        );
    }
}

impl Drop for Progress {
    fn drop(&mut self) {
        if self.is_tty && self.started && self.current < self.total {
            eprintln!();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_initial_state() {
        let progress = Progress::new("Test", 100);
        assert_eq!(progress.current, 0);
        assert_eq!(progress.total, 100);
        assert_eq!(progress.last_percent, 0);
    }

    #[test]
    fn test_progress_update() {
        let mut progress = Progress::new("Test", 100);

        progress.update(50);
        assert_eq!(progress.current, 50);
        assert_eq!(progress.last_percent, 50);

        progress.update(75);
        assert_eq!(progress.current, 75);
        assert_eq!(progress.last_percent, 75);
    }

    #[test]
    fn test_progress_bounds_clamping() {
        let mut progress = Progress::new("Test", 10);
        progress.update(20);
        assert_eq!(progress.current, 10);
    }

    #[test]
    fn test_progress_zero_total() {
        let mut progress = Progress::new("Test", 0);
        progress.update(5);
        assert_eq!(progress.current, 0);
        assert_eq!(progress.last_percent, 0);
    }

    #[test]
    fn test_progress_percentage_calculation() {
        let mut progress = Progress::new("Test", 100);

        progress.update(25);
        assert_eq!(progress.last_percent, 25);

        progress.update(50);
        assert_eq!(progress.last_percent, 50);

        progress.update(100);
        assert_eq!(progress.last_percent, 100);
    }
}
