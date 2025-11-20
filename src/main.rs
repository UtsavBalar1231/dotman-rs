use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{Generator, Shell, generate};
use colored::Colorize;
use dotman::{DotmanContext, commands};
use std::io;
use std::process;

#[derive(Parser)]
#[command(
    name = "dot",
    version = dotman::VERSION,
    about = "Extremely fast dotfiles manager",
    long_about = "A git-like dotfiles manager optimized for maximum performance"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, global = true)]
    verbose: bool,

    #[arg(long, global = true, help = "Disable pager output")]
    no_pager: bool,
}

#[derive(Subcommand)]
enum Commands {
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

        #[arg(short, long)]
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
    },

    /// Restore specific files from a commit
    Restore {
        /// Files to restore
        paths: Vec<String>,

        /// Source commit to restore from
        #[arg(short, long, default_value = "HEAD")]
        source: String,
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
    Log {
        /// Commit to start from (defaults to showing all commits)
        target: Option<String>,

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
        #[arg(short = 'b', long = "branch", conflicts_with = "action")]
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
}

#[derive(Subcommand)]
enum StashAction {
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

#[derive(Subcommand)]
enum TagAction {
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

#[derive(Subcommand)]
enum RemoteAction {
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

#[derive(Subcommand)]
enum BranchAction {
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
        old_name: Option<String>,
        /// New branch name
        new_name: String,
    },

    /// Set upstream tracking for a branch
    SetUpstream {
        /// Branch name (current branch if not specified)
        #[arg(short, long)]
        branch: Option<String>,
        /// Remote name
        remote: String,
        /// Remote branch name (same as local branch if not specified)
        #[arg(short = 'b', long)]
        remote_branch: Option<String>,
    },

    /// Remove upstream tracking for a branch
    UnsetUpstream {
        /// Branch name (current branch if not specified)
        branch: Option<String>,
    },
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{} {}", "Error:".red().bold(), e);
        process::exit(1);
    }
}

#[allow(clippy::too_many_lines)]
fn run() -> Result<()> {
    let cli = Cli::parse();

    let context = match &cli.command {
        Commands::Init { .. } | Commands::Completion { .. } => None,
        Commands::Remote { .. } | Commands::Branch { .. } | Commands::Config { .. } => {
            // Remote, Branch and Config commands need mutable context
            Some(DotmanContext::new_with_pager(cli.no_pager)?)
        }
        Commands::Stash { .. } => {
            // Stash command needs context
            Some(DotmanContext::new_with_pager(cli.no_pager)?)
        }
        _ => Some(DotmanContext::new_with_pager(cli.no_pager)?),
    };

    match cli.command {
        Commands::Add { paths, force, all } => {
            let ctx = context.context("Context not initialized for add command")?;
            commands::add::execute(&ctx, &paths, force, all)?;
        }
        Commands::Status { short, untracked } => {
            let ctx = context.context("Context not initialized for status command")?;
            commands::status::execute_verbose(&ctx, short, untracked, cli.verbose)?;
        }
        Commands::Commit {
            message,
            all,
            amend,
        } => {
            let ctx = context.context("Context not initialized for commit command")?;
            if amend {
                commands::commit::execute_amend(&ctx, message.as_deref(), all)?;
            } else {
                let msg = message
                    .ok_or_else(|| anyhow::anyhow!("Commit message is required (use -m)"))?;
                commands::commit::execute(&ctx, &msg, all)?;
            }
        }
        Commands::Checkout {
            target,
            force,
            new_branch,
        } => {
            let ctx = context.context("Context not initialized for checkout command")?;

            if let Some(branch_name) = new_branch {
                // Create and checkout new branch (-b flag used)
                let start_point = target.as_deref();
                commands::branch::create(&ctx, &branch_name, start_point)?;
                commands::checkout::execute(&ctx, &branch_name, force)?;
            } else {
                // Regular checkout (no -b flag)
                let target_ref =
                    target.ok_or_else(|| anyhow::anyhow!("Target branch or commit required"))?;
                commands::checkout::execute(&ctx, &target_ref, force)?;
            }
        }
        Commands::Reset {
            commit,
            hard,
            soft,
            mixed,
            keep,
            paths,
        } => {
            let ctx = context.context("Context not initialized for reset command")?;
            commands::reset::execute(
                &ctx,
                &commit,
                &commands::reset::ResetOptions {
                    hard,
                    soft,
                    mixed,
                    keep,
                },
                &paths,
            )?;
        }
        Commands::Revert {
            commit,
            no_edit,
            force,
        } => {
            let ctx = context.context("Context not initialized for revert command")?;
            commands::revert::execute(&ctx, &commit, no_edit, force)?;
        }
        Commands::Restore { paths, source } => {
            let ctx = context.context("Context not initialized for restore command")?;
            commands::restore::execute(&ctx, &paths, Some(&source))?;
        }
        Commands::Fetch {
            remote,
            branch,
            all,
            tags,
        } => {
            let ctx = context.context("Context not initialized for fetch command")?;
            commands::fetch::execute(&ctx, &remote, branch.as_deref(), all, tags)?;
        }
        Commands::Merge {
            branch,
            no_ff,
            squash,
            message,
        } => {
            let ctx = context.context("Context not initialized for merge command")?;
            commands::merge::execute(&ctx, &branch, no_ff, squash, message.as_deref())?;
        }
        Commands::Push {
            remote,
            branch,
            force,
            force_with_lease,
            dry_run,
            tags,
            set_upstream,
        } => {
            let mut ctx = context.context("Context not initialized for push command")?;
            commands::push::execute(
                &mut ctx,
                &commands::push::PushArgs {
                    remote,
                    branch,
                    force,
                    force_with_lease,
                    dry_run,
                    tags,
                    set_upstream,
                },
            )?;
        }
        Commands::Pull {
            remote,
            branch,
            rebase,
            no_ff,
            squash,
        } => {
            let ctx = context.context("Context not initialized for pull command")?;
            commands::pull::execute(
                &ctx,
                remote.as_deref(),
                branch.as_deref(),
                rebase,
                no_ff,
                squash,
            )?;
        }
        Commands::Init { bare } => {
            commands::init::execute(bare)?;
        }
        Commands::Show { object } => {
            let ctx = context.context("Context not initialized for show command")?;
            commands::show::execute(&ctx, &object)?;
        }
        Commands::Log {
            target,
            limit,
            oneline,
            all,
        } => {
            let ctx = context.context("Context not initialized for log command")?;
            commands::log::execute(&ctx, target.as_deref(), limit, oneline, all)?;
        }
        Commands::Diff { from, to } => {
            let ctx = context.context("Context not initialized for diff command")?;
            commands::diff::execute(&ctx, from.as_deref(), to.as_deref())?;
        }
        Commands::Rm {
            paths,
            cached,
            force,
            recursive,
            dry_run,
        } => {
            let ctx = context.context("Context not initialized for rm command")?;
            commands::rm::execute(
                &ctx,
                &paths,
                &commands::rm::RmOptions {
                    cached,
                    force,
                    recursive,
                    dry_run,
                },
            )?;
        }
        Commands::Clean { dry_run, force } => {
            let ctx = context.context("Context not initialized for clean command")?;
            commands::clean::execute(&ctx, dry_run, force)?;
        }
        Commands::Remote { action } => {
            let mut ctx = context.context("Context not initialized for remote command")?;
            match action {
                RemoteAction::List => commands::remote::list(&ctx)?,
                RemoteAction::Add { name, url } => commands::remote::add(&mut ctx, &name, &url)?,
                RemoteAction::Remove { name } => commands::remote::remove(&mut ctx, &name)?,
                RemoteAction::SetUrl { name, url } => {
                    commands::remote::set_url(&mut ctx, &name, &url)?;
                }
                RemoteAction::Show { name } => commands::remote::show(&ctx, &name)?,
                RemoteAction::Rename { old_name, new_name } => {
                    commands::remote::rename(&mut ctx, &old_name, &new_name)?;
                }
            }
        }
        Commands::Config {
            key,
            value,
            unset,
            list,
        } => {
            let mut ctx = context.context("Context not initialized for config command")?;
            commands::config::execute(&mut ctx, key.as_deref(), value, unset, list)?;
        }
        Commands::Branch {
            action,
            new_branch,
            start_point,
        } => {
            let mut ctx = context.context("Context not initialized for branch command")?;

            // Handle -b flag (shorthand for create + checkout)
            if let Some(branch_name) = new_branch {
                commands::branch::create(&ctx, &branch_name, start_point.as_deref())?;
                commands::checkout::execute(&ctx, &branch_name, false)?;
            } else {
                // Regular branch subcommands
                match action {
                    None | Some(BranchAction::List) => commands::branch::list(&ctx)?,
                    Some(BranchAction::Create { name, from }) => {
                        commands::branch::create(&ctx, &name, from.as_deref())?;
                    }
                    Some(BranchAction::Delete { name, force }) => {
                        commands::branch::delete(&ctx, &name, force)?;
                    }
                    Some(BranchAction::Checkout { name, force }) => {
                        commands::branch::checkout(&ctx, &name, force)?;
                    }
                    Some(BranchAction::Rename { old_name, new_name }) => {
                        commands::branch::rename(&ctx, old_name.as_deref(), &new_name)?;
                    }
                    Some(BranchAction::SetUpstream {
                        branch,
                        remote,
                        remote_branch,
                    }) => commands::branch::set_upstream(
                        &mut ctx,
                        branch.as_deref(),
                        &remote,
                        remote_branch.as_deref(),
                    )?,
                    Some(BranchAction::UnsetUpstream { branch }) => {
                        commands::branch::unset_upstream(&mut ctx, branch.as_deref())?;
                    }
                }
            }
        }
        Commands::Completion { shell } => {
            print_completions(shell, &mut Cli::command());
        }
        Commands::Tag { action } => {
            let ctx = context.context("Context not initialized for tag command")?;
            match action {
                None | Some(TagAction::List) => commands::tag::list(&ctx)?,
                Some(TagAction::Create { name, commit }) => {
                    commands::tag::create(&ctx, &name, commit.as_deref())?;
                }
                Some(TagAction::Delete { name, force }) => {
                    commands::tag::delete(&ctx, &name, force)?;
                }
                Some(TagAction::Show { name }) => commands::tag::show(&ctx, &name)?,
            }
        }
        Commands::Stash { action } => {
            let ctx = context.context("Context not initialized for stash command")?;
            let stash_cmd = match action {
                None | Some(StashAction::Push { .. }) => {
                    // Default to push when no subcommand or explicit push
                    if let Some(StashAction::Push {
                        message,
                        include_untracked,
                        keep_index,
                    }) = action
                    {
                        commands::stash::StashCommand::Push {
                            message,
                            include_untracked,
                            keep_index,
                        }
                    } else {
                        // Default push with no options
                        commands::stash::StashCommand::Push {
                            message: None,
                            include_untracked: false,
                            keep_index: false,
                        }
                    }
                }
                Some(StashAction::Pop) => commands::stash::StashCommand::Pop,
                Some(StashAction::Apply { stash }) => {
                    commands::stash::StashCommand::Apply { stash_id: stash }
                }
                Some(StashAction::List) => commands::stash::StashCommand::List,
                Some(StashAction::Show { stash }) => {
                    commands::stash::StashCommand::Show { stash_id: stash }
                }
                Some(StashAction::Drop { stash }) => {
                    commands::stash::StashCommand::Drop { stash_id: stash }
                }
                Some(StashAction::Clear) => commands::stash::StashCommand::Clear,
            };
            commands::stash::execute(&ctx, stash_cmd)?;
        }
        Commands::Reflog {
            limit,
            oneline,
            all,
        } => {
            let ctx = context.context("Context not initialized for reflog command")?;
            commands::reflog::execute(&ctx, limit, oneline, all)?;
        }
        Commands::Import {
            source,
            track,
            force,
            dry_run,
            yes,
        } => {
            let ctx = context.context("Context not initialized for import command")?;
            let options = commands::import::ImportOptions {
                track,
                force,
                dry_run,
                yes,
            };
            commands::import::execute(&ctx, &source, &options)?;
        }
    }

    Ok(())
}

fn print_completions<G: Generator>(g: G, cmd: &mut clap::Command) {
    generate(g, cmd, cmd.get_name().to_string(), &mut io::stdout());
}
