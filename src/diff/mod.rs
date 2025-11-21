//! Diff generation for file comparisons.
//!
//! This module provides unified diff generation with support for:
//! - Line-by-line diffs with context
//! - Binary file detection
//! - Colorized output
//! - Multiple diff algorithms (Myers, Patience)

/// Binary file detection utilities
pub mod binary;
/// Unified diff generation for text files
pub mod unified;

pub use binary::is_binary_file;
pub use unified::{UnifiedDiffConfig, generate_unified_diff};

use similar::Algorithm;

/// Convert `DiffAlgorithm` config enum to `similar::Algorithm`
#[must_use]
pub const fn config_to_algorithm(algo: &crate::config::DiffAlgorithm) -> Algorithm {
    match algo {
        crate::config::DiffAlgorithm::Myers => Algorithm::Myers,
        crate::config::DiffAlgorithm::Patience => Algorithm::Patience,
    }
}
