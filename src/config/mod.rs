#![allow(clippy::module_inception)]
pub mod config;
pub mod profile;
pub mod entry;
pub mod migration;

pub use config::Config;
pub use profile::{Profile, ProfileManager};
pub use entry::ConfigEntry;
pub use migration::{ConfigMigration, MigrationInfo};
