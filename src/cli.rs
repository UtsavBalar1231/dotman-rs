//! Command-line interface definitions for dotman.
//!
//! This module contains all CLI argument parsing structures using clap's derive macros.
//! The CLI definitions are shared between the main binary and build tools (like xtask)
//! for man page generation.
//!
//! Note: Field-level documentation is provided via clap attributes (#[arg(help = "...")]),
//! so we allow missing_docs for this module to avoid redundant documentation.

#![allow(missing_docs)]
#![allow(clippy::missing_docs_in_private_items)]

use clap::{Parser, Subcommand};
use clap_complete::Shell;

/// Main CLI structure for dotman.
#[derive(Parser)]
#[command(
    name = "dot",
    version = crate::VERSION,
    about = "Git-like dotfiles manager with content deduplication",
    long_about = "A dotfiles manager with git-like commands, using xxHash3 and parallel processing"
)]
pub struct Cli {
    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Commands,

    /// Show verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress informational messages
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Disable pager output
    #[arg(long, global = true, help = "Disable pager output")]
    pub no_pager: bool,
}

/// All available commands.
#[derive(Subcommand)]
pub enum Commands {
    /// Add files to be tracked
    Add {
        /// Paths to add
        paths: Vec<String>,

        #[arg(short, long)]
        force: bool,

        /// Stage all changes (modified, deleted, and new files)
        #[arg(short = 'A', long)]
        all: bool,
    },

    /// Show the working tree status
    Status {
        #[arg(short, long)]
        short: bool,

        /// Show untracked files (default: true, use --no-untracked to disable)
        #[arg(short, long, default_value_t = true, action = clap::ArgAction::Set)]
        untracked: bool,
    },

    /// Record changes to the repository
    Commit {
        #[arg(short, long)]
        message: Option<String>,

        #[arg(short, long)]
        all: bool,

        /// Amend the previous commit
        #[arg(long)]
        amend: bool,
    },

    /// Switch branches or restore working tree files
    Checkout {
        /// Branch or commit to checkout (or start point when using -b)
        target: Option<String>,

        #[arg(short, long)]
        force: bool,

        /// Show what would happen without making changes
        #[arg(long)]
        dry_run: bool,

        /// Create a new branch and check it out
        #[arg(short = 'b', long = "branch")]
        new_branch: Option<String>,
    },

    /// Reset current HEAD to the specified state
    Reset {
        /// Commit to reset to
        #[arg(default_value = "HEAD")]
        commit: String,

        #[arg(long)]
        hard: bool,

        #[arg(long)]
        soft: bool,

        #[arg(long)]
        mixed: bool,

        #[arg(long)]
        keep: bool,

        /// Show what would happen without making changes
        #[arg(long)]
        dry_run: bool,

        /// Files to reset
        #[arg(last = true)]
        paths: Vec<String>,
    },

    /// Create a new commit that undoes changes from a specified commit
    Revert {
        /// Commit to revert
        commit: String,

        /// Skip the commit confirmation and immediately create the revert commit
        #[arg(short, long)]
        no_edit: bool,

        /// Allow reverting when there are uncommitted changes
        #[arg(short, long)]
        force: bool,

        /// Show what would happen without making changes
        #[arg(long)]
        dry_run: bool,
    },

    /// Restore specific files from a commit
    Restore {
        /// Files to restore
        paths: Vec<String>,

        /// Source commit to restore from
        #[arg(short, long, default_value = "HEAD")]
        source: String,

        /// Show what would happen without making changes
        #[arg(long)]
        dry_run: bool,
    },

    /// Update remote refs along with associated objects
    Push {
        /// Remote name (uses tracking if not specified)
        remote: Option<String>,

        /// Branch to push (uses current branch if not specified)
        branch: Option<String>,

        #[arg(short, long)]
        force: bool,

        #[arg(long)]
        force_with_lease: bool,

        #[arg(long)]
        dry_run: bool,

        #[arg(long)]
        tags: bool,

        /// Set upstream tracking for the branch
        #[arg(short = 'u', long)]
        set_upstream: bool,
    },

    /// Download objects and refs from another repository
    Fetch {
        /// Remote name
        #[arg(default_value = "origin")]
        remote: String,

        /// Branch to fetch
        branch: Option<String>,

        #[arg(long)]
        all: bool,

        #[arg(long)]
        tags: bool,
    },

    /// Join two or more development histories together
    Merge {
        /// Branch or commit to merge
        branch: String,

        #[arg(long)]
        no_ff: bool,

        #[arg(long)]
        squash: bool,

        #[arg(short, long)]
        message: Option<String>,

        /// Show what would happen without making changes
        #[arg(long)]
        dry_run: bool,
    },

    /// Reapply commits on top of another base
    Rebase {
        /// Upstream branch or commit to rebase onto
        upstream: Option<String>,

        /// Branch to rebase (defaults to current branch)
        #[arg(value_name = "BRANCH")]
        branch: Option<String>,

        /// Continue rebase after resolving conflicts
        #[arg(long, conflicts_with_all = ["abort", "skip", "upstream"])]
        r#continue: bool,

        /// Abort rebase and restore original state
        #[arg(long, conflicts_with_all = ["continue", "skip", "upstream"])]
        abort: bool,

        /// Skip current commit and continue
        #[arg(long, conflicts_with_all = ["continue", "abort", "upstream"])]
        skip: bool,
    },

    /// Fetch from and integrate with another repository
    Pull {
        /// Remote name (uses tracking if not specified)
        remote: Option<String>,

        /// Branch to pull (uses current branch if not specified)
        branch: Option<String>,

        #[arg(long)]
        rebase: bool,

        #[arg(long)]
        no_ff: bool,

        #[arg(long)]
        squash: bool,
    },

    /// Initialize a new dotman repository
    Init {
        #[arg(short, long)]
        bare: bool,
    },

    /// Show various types of objects
    Show {
        /// Object to show
        object: String,
    },

    /// Show commit logs
    ///
    /// Arguments: \[refs...\] \[--\] \[paths...\]
    ///
    /// Without `--`: Uses heuristic (first arg as ref if valid, rest as paths).
    /// With `--`: Everything before is refs, everything after is paths.
    ///
    /// Examples:
    ///   dot log                    # Show recent commits from HEAD
    ///   dot log HEAD~5             # Show from 5 commits back
    ///   dot log .bashrc            # Show commits that modified .bashrc
    ///   dot log HEAD .bashrc       # Show .bashrc changes from HEAD
    ///   dot log file1 file2        # Show commits touching either file
    ///   dot log -- main            # Force 'main' as path (not branch)
    ///   dot log HEAD -- config     # Explicit: ref=HEAD, path=config
    ///   dot log main feature -- f  # Union: commits from main OR feature
    Log {
        /// Commit references to start from (before --, default: HEAD)
        #[arg(value_terminator = "--")]
        refs: Vec<String>,

        /// File paths to filter by (after --)
        #[arg(last = true)]
        paths: Vec<String>,

        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,

        #[arg(long)]
        oneline: bool,

        /// Show all commits including orphaned ones
        #[arg(long)]
        all: bool,
    },

    /// Show changes between commits
    Diff {
        /// First commit
        from: Option<String>,

        /// Second commit
        to: Option<String>,
    },

    /// Remove files from tracking (files remain on disk)
    Rm {
        /// Paths to remove from tracking
        paths: Vec<String>,

        /// Only remove from index, keep in repository storage
        #[arg(short, long)]
        cached: bool,

        /// Force removal even if file doesn't exist
        #[arg(short, long)]
        force: bool,

        /// Remove directories recursively
        #[arg(short = 'r', long)]
        recursive: bool,

        /// Show what would be removed without making changes
        #[arg(long)]
        dry_run: bool,
    },

    /// Remove untracked files from working directory
    Clean {
        /// Dry run - only show what would be removed
        #[arg(short = 'n', long)]
        dry_run: bool,

        /// Force removal of untracked files
        #[arg(short, long)]
        force: bool,
    },

    /// Manage remote repositories
    Remote {
        #[command(subcommand)]
        action: RemoteAction,
    },

    /// Manage branches
    Branch {
        #[command(subcommand)]
        action: Option<BranchAction>,

        /// Create a new branch and check it out (shorthand for checkout -b)
        #[arg(short = 'b', long = "branch")]
        new_branch: Option<String>,

        /// Start point for new branch (defaults to HEAD)
        #[arg(requires = "new_branch")]
        start_point: Option<String>,
    },

    /// Get and set repository or user options
    Config {
        /// Configuration key
        key: Option<String>,

        /// Configuration value to set
        value: Option<String>,

        /// Unset the configuration key
        #[arg(long)]
        unset: bool,

        /// List all configuration values
        #[arg(short, long)]
        list: bool,
    },

    /// Generate shell completion scripts
    Completion {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },

    /// Create, list, or delete tags for important commits
    Tag {
        #[command(subcommand)]
        action: Option<TagAction>,
    },

    /// Temporarily save changes to a dirty working directory
    Stash {
        #[command(subcommand)]
        action: Option<StashAction>,
    },

    /// Show reference update history for recovery
    Reflog {
        /// Number of entries to show
        #[arg(short = 'n', long, default_value = "20")]
        limit: usize,

        /// Show one line per entry
        #[arg(long)]
        oneline: bool,

        /// Show all entries
        #[arg(long)]
        all: bool,
    },

    /// Import dotfiles from a git repository
    Import {
        /// Repository path or URL to import from
        source: String,

        /// Automatically track imported files with dotman
        #[arg(short, long)]
        track: bool,

        /// Force overwrite existing files
        #[arg(short, long)]
        force: bool,

        /// Only show what would be imported without making changes
        #[arg(long)]
        dry_run: bool,

        /// Skip confirmation prompts
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Verify repository integrity and consistency
    Fsck,
}

/// Stash subcommands.
#[derive(Subcommand)]
pub enum StashAction {
    /// Save your local modifications to a new stash (default)
    #[command(alias = "save")]
    Push {
        /// Stash message
        #[arg(short, long)]
        message: Option<String>,

        /// Include untracked files
        #[arg(short = 'u', long)]
        include_untracked: bool,

        /// Keep changes in index
        #[arg(short = 'k', long)]
        keep_index: bool,
    },

    /// Remove a stash and apply it on top of the current working tree
    Pop,

    /// Apply a stash on top of the current working tree
    Apply {
        /// Stash to apply (defaults to latest)
        stash: Option<String>,
    },

    /// List all stashes
    List,

    /// Show the changes recorded in a stash
    Show {
        /// Stash to show (defaults to latest)
        stash: Option<String>,
    },

    /// Remove a stash from the list
    Drop {
        /// Stash to drop
        stash: String,
    },

    /// Remove all stashes
    Clear,
}

/// Tag subcommands.
#[derive(Subcommand)]
pub enum TagAction {
    /// Create a new tag
    Create {
        /// Tag name
        name: String,
        /// Commit to tag (defaults to HEAD)
        commit: Option<String>,
    },

    /// List all tags
    List,

    /// Delete a tag
    Delete {
        /// Tag name
        name: String,
        /// Force deletion
        #[arg(short, long)]
        force: bool,
    },

    /// Show details about a tag
    Show {
        /// Tag name
        name: String,
    },
}

/// Remote subcommands.
#[derive(Subcommand)]
pub enum RemoteAction {
    /// List all remotes
    List,

    /// Add a new remote
    Add {
        /// Remote name
        name: String,
        /// Remote URL
        url: String,
    },

    /// Remove a remote
    Remove {
        /// Remote name
        name: String,
    },

    /// Set the URL for a remote
    SetUrl {
        /// Remote name
        name: String,
        /// New URL
        url: String,
    },

    /// Show information about a remote
    Show {
        /// Remote name
        name: String,
    },

    /// Rename a remote
    Rename {
        /// Old remote name
        old_name: String,
        /// New remote name
        new_name: String,
    },
}

/// Branch subcommands.
#[derive(Subcommand)]
pub enum BranchAction {
    /// List all branches
    List,

    /// Create a new branch
    Create {
        /// Branch name
        name: String,
        /// Starting point (commit or branch)
        #[arg(short, long)]
        from: Option<String>,
    },

    /// Delete a branch
    Delete {
        /// Branch name
        name: String,
        /// Force deletion
        #[arg(short, long)]
        force: bool,
    },

    /// Checkout a branch
    Checkout {
        /// Branch name
        name: String,
        /// Force checkout even with uncommitted changes
        #[arg(short, long)]
        force: bool,
    },

    /// Rename a branch
    Rename {
        /// Old branch name (current branch if not specified)
        #[arg(short = 'o', long)]
        old_name: Option<String>,
        /// New branch name
        #[arg(short = 'n', long)]
        new_name: String,
    },

    /// Set upstream tracking for a branch
    SetUpstream {
        /// Branch name (current branch if not specified)
        #[arg(short = 'l', long)]
        branch: Option<String>,
        /// Remote name
        remote: String,
        /// Remote branch name (same as local branch if not specified)
        #[arg(short = 'r', long)]
        remote_branch: Option<String>,
    },

    /// Remove upstream tracking for a branch
    UnsetUpstream {
        /// Branch name (current branch if not specified)
        branch: Option<String>,
    },
}
