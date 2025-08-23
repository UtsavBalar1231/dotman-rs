use anyhow::Result;
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
}

#[derive(Subcommand)]
enum Commands {
    /// Add files to be tracked
    Add {
        /// Paths to add
        paths: Vec<String>,

        #[arg(short, long)]
        force: bool,
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
        message: String,

        #[arg(short, long)]
        all: bool,
    },

    /// Switch branches or restore working tree files
    Checkout {
        /// Branch or commit to checkout
        target: String,

        #[arg(short, long)]
        force: bool,
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
    },

    /// Update remote refs along with associated objects
    Push {
        /// Remote name
        #[arg(default_value = "origin")]
        remote: String,

        /// Branch to push
        #[arg(default_value = "main")]
        branch: String,
    },

    /// Fetch from and integrate with another repository
    Pull {
        /// Remote name
        #[arg(default_value = "origin")]
        remote: String,

        /// Branch to pull
        #[arg(default_value = "main")]
        branch: String,
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
    },

    /// Show changes between commits
    Diff {
        /// First commit
        from: Option<String>,

        /// Second commit
        to: Option<String>,
    },

    /// Remove files from tracking
    Rm {
        /// Paths to remove
        paths: Vec<String>,

        #[arg(short, long)]
        cached: bool,

        #[arg(short, long)]
        force: bool,

        #[arg(short, long)]
        interactive: bool,
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
    },

    /// Generate shell completion scripts
    Completion {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
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

fn run() -> Result<()> {
    let cli = Cli::parse();

    // Initialize context
    let context = match &cli.command {
        Commands::Init { .. } | Commands::Completion { .. } => None,
        Commands::Remote { .. } | Commands::Branch { .. } => {
            // Remote and Branch commands need mutable context
            Some(DotmanContext::new()?)
        }
        _ => Some(DotmanContext::new()?),
    };

    // Execute command
    match cli.command {
        Commands::Add { paths, force } => {
            let ctx = context.unwrap();
            commands::add::execute(&ctx, &paths, force)?;
        }
        Commands::Status { short, untracked } => {
            let ctx = context.unwrap();
            commands::status::execute(&ctx, short, untracked)?;
        }
        Commands::Commit { message, all } => {
            let ctx = context.unwrap();
            commands::commit::execute(&ctx, &message, all)?;
        }
        Commands::Checkout { target, force } => {
            let ctx = context.unwrap();
            commands::checkout::execute(&ctx, &target, force)?;
        }
        Commands::Reset { commit, hard, soft } => {
            let ctx = context.unwrap();
            commands::reset::execute(&ctx, &commit, hard, soft)?;
        }
        Commands::Push { remote, branch } => {
            let ctx = context.unwrap();
            commands::push::execute(&ctx, &remote, &branch)?;
        }
        Commands::Pull { remote, branch } => {
            let ctx = context.unwrap();
            commands::pull::execute(&ctx, &remote, &branch)?;
        }
        Commands::Init { bare } => {
            commands::init::execute(bare)?;
        }
        Commands::Show { object } => {
            let ctx = context.unwrap();
            commands::show::execute(&ctx, &object)?;
        }
        Commands::Log {
            target,
            limit,
            oneline,
        } => {
            let ctx = context.unwrap();
            commands::log::execute(&ctx, target.as_deref(), limit, oneline)?;
        }
        Commands::Diff { from, to } => {
            let ctx = context.unwrap();
            commands::diff::execute(&ctx, from.as_deref(), to.as_deref())?;
        }
        Commands::Rm {
            paths,
            cached,
            force,
            interactive,
        } => {
            let ctx = context.unwrap();
            commands::rm::execute(&ctx, &paths, cached, force, interactive)?;
        }
        Commands::Remote { action } => {
            let mut ctx = context.unwrap();
            match action {
                RemoteAction::List => commands::remote::list(&ctx)?,
                RemoteAction::Add { name, url } => commands::remote::add(&mut ctx, &name, &url)?,
                RemoteAction::Remove { name } => commands::remote::remove(&mut ctx, &name)?,
                RemoteAction::SetUrl { name, url } => {
                    commands::remote::set_url(&mut ctx, &name, &url)?
                }
                RemoteAction::Show { name } => commands::remote::show(&ctx, &name)?,
                RemoteAction::Rename { old_name, new_name } => {
                    commands::remote::rename(&mut ctx, &old_name, &new_name)?
                }
            }
        }
        Commands::Branch { action } => {
            let mut ctx = context.unwrap();
            match action {
                None | Some(BranchAction::List) => commands::branch::list(&ctx)?,
                Some(BranchAction::Create { name, from }) => {
                    commands::branch::create(&ctx, &name, from.as_deref())?
                }
                Some(BranchAction::Delete { name, force }) => {
                    commands::branch::delete(&ctx, &name, force)?
                }
                Some(BranchAction::Rename { old_name, new_name }) => {
                    commands::branch::rename(&ctx, old_name.as_deref(), &new_name)?
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
                    commands::branch::unset_upstream(&mut ctx, branch.as_deref())?
                }
            }
        }
        Commands::Completion { shell } => {
            print_completions(shell, &mut Cli::command());
        }
    }

    Ok(())
}

fn print_completions<G: Generator>(g: G, cmd: &mut clap::Command) {
    generate(g, cmd, cmd.get_name().to_string(), &mut io::stdout());
}
