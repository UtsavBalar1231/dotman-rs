/// File tracking and staging operations.
pub mod add;
/// Branch management operations (create, delete, rename, list).
pub mod branch;
/// Checkout operations to restore files from commits.
pub mod checkout;
/// Clean untracked files and directories.
pub mod clean;
/// Commit operations and snapshot creation.
pub mod commit;
/// Configuration viewing and management.
pub mod config;
/// Shared command context and utilities.
pub mod context;
/// Show differences between commits and working tree.
pub mod diff;
/// Fetch changes from remote repositories.
pub mod fetch;
/// File system consistency check.
pub mod fsck;
/// Import configurations from other systems.
pub mod import;
/// Repository initialization.
pub mod init;
/// View commit history.
pub mod log;
/// Merge branches and resolve conflicts.
pub mod merge;
/// Fetch and merge from remote.
pub mod pull;
/// Push changes to remote repository.
pub mod push;
/// Reference log operations.
pub mod reflog;
/// Remote repository management.
pub mod remote;
/// Remote operation utilities.
pub mod remote_ops;
/// Reset repository state to specific commits.
pub mod reset;
/// Restore files from commits.
pub mod restore;
/// Revert specific commits.
pub mod revert;
/// Remove files from tracking.
pub mod rm;
/// Show commit and file information.
pub mod show;
/// Stash and unstash changes.
pub mod stash;
/// Show working tree status.
pub mod status;
/// Tag management.
pub mod tag;
