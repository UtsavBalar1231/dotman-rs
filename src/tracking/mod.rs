//! Tracking system for managing what directories and files dotman monitors.
//!
//! This module provides the infrastructure for directory-level tracking,
//! which is essential for proper dotfiles management. When a user adds
//! a directory like `~/.config/nvim`, dotman remembers this intent and
//! automatically detects changes to any files within that directory.
//!
//! # Architecture
//!
//! The tracking system consists of two main components:
//!
//! - [`crate::tracking::TrackingManifest`] - Stores what the user wants tracked
//! - [`crate::tracking::DirectoryScanner`] - Scans tracked locations to find files
//!
//! # Usage
//!
//! ```no_run
//! use dotman::tracking::manifest::TrackingManifest;
//! use dotman::tracking::scanner::DirectoryScanner;
//! use std::path::PathBuf;
//!
//! # fn main() -> anyhow::Result<()> {
//! // Create and save a manifest
//! let mut manifest = TrackingManifest::new();
//! manifest.add_directory(PathBuf::from("/home/user/.config/nvim"));
//! manifest.save(PathBuf::from("/home/user/.dotman").as_path())?;
//!
//! // Load and scan
//! let manifest = TrackingManifest::load(PathBuf::from("/home/user/.dotman").as_path())?;
//! let scanner = DirectoryScanner::new(
//!     manifest,
//!     Vec::new(),
//!     false,
//!     PathBuf::from("/home/user")
//! );
//! let files = scanner.scan_all_files()?;
//! # Ok(())
//! # }
//! ```

pub mod manifest;
pub mod scanner;

pub use manifest::TrackingManifest;
pub use scanner::DirectoryScanner;
