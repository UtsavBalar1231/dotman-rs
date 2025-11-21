/// Adaptive pager writer that buffers output and dynamically chooses backend.
mod adaptive;
/// Pager configuration structures and builders.
mod config;
/// Pager command parsing utilities.
mod parser;
/// Pager process spawning and management.
mod process;
/// Pager writer implementations and trait definitions.
mod writer;

pub use config::PagerConfig;
pub use writer::{Pager, PagerWriter};

use anyhow::Result;
use tracing::{Level, debug, span};

/// Builder for creating a Pager with custom configuration
pub struct PagerBuilder {
    /// Configuration for the pager, if specified.
    config: Option<PagerConfig>,
    /// Force pager to be disabled regardless of other settings.
    force_disabled: bool,
}

impl PagerBuilder {
    /// Creates a new pager builder with default configuration.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            config: None,
            force_disabled: false,
        }
    }

    /// Sets the pager configuration.
    #[must_use]
    pub fn config(mut self, config: PagerConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Force the pager to be disabled regardless of configuration.
    #[must_use]
    pub const fn force_disabled(mut self, disabled: bool) -> Self {
        self.force_disabled = disabled;
        self
    }

    /// Builds the pager, either spawning a process or using direct output.
    ///
    /// # Errors
    ///
    /// Returns an error if the pager process cannot be spawned or direct output initialization fails.
    pub fn build(self) -> Result<Pager> {
        let config = self.config.unwrap_or_default();
        let disabled = self.force_disabled || config.disabled;

        let span = span!(Level::DEBUG, "pager_init", disabled, command = %config.command, auto_detect = config.auto_detect);
        let _guard = span.enter();

        let writer: Box<dyn PagerWriter> = if disabled || !process::should_use_pager(&config) {
            debug!("Using direct output (no pager)");
            Box::new(writer::DirectOutput::new())
        } else if config.auto_detect {
            debug!("Using adaptive pager (will decide based on output size)");
            Box::new(adaptive::AdaptiveWriter::new(config))
        } else {
            debug!("Spawning pager process immediately");
            Box::new(process::spawn_pager(&config.command)?)
        };

        Ok(Pager { writer })
    }
}

impl Default for PagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}
