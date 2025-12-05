use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};
use clap_complete::{Generator, generate};
use colored::Colorize;
use dotman::cli::{BranchAction, Cli, Commands, RemoteAction, StashAction, TagAction};
use dotman::{DotmanContext, commands};
use std::io;
use std::process;
use tracing_subscriber::EnvFilter;

/// Initialize signal handlers for proper pager interaction
#[cfg(unix)]
fn init_signal_handlers() {
    unsafe {
        // Ignore SIGPIPE - handle as EPIPE error instead of crashing
        // This is critical for pager interaction (when user quits pager early)
        libc::signal(libc::SIGPIPE, libc::SIG_IGN);
    }
}

#[cfg(not(unix))]
fn init_signal_handlers() {
    // No SIGPIPE on Windows
}

/// Initialize tracing/logging system
fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .with_target(false)
        .with_level(true)
        .init();
}

fn main() {
    // Initialize signal handlers (must be done early)
    init_signal_handlers();

    // Initialize tracing
    init_tracing();

    if let Err(e) = run() {
        eprintln!("{} {}", "Error:".red().bold(), e);
        process::exit(1);
    }
}

#[allow(clippy::too_many_lines)]
fn run() -> Result<()> {
    let cli = Cli::parse();

    // Initialize output verbosity from CLI flags
    let verbosity = if cli.quiet {
        dotman::output::Verbosity::Quiet
    } else if cli.verbose {
        dotman::output::Verbosity::Verbose
    } else {
        dotman::output::Verbosity::Normal
    };
    dotman::output::set_verbosity(verbosity);

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
            dry_run,
            new_branch,
        } => {
            let ctx = context.context("Context not initialized for checkout command")?;

            if let Some(branch_name) = new_branch {
                // Create and checkout new branch (-b flag used)
                let start_point = target.as_deref();
                commands::branch::create(&ctx, &branch_name, start_point)?;
                commands::checkout::execute(&ctx, &branch_name, force, dry_run)?;
            } else {
                // Regular checkout (no -b flag)
                let target_ref =
                    target.ok_or_else(|| anyhow::anyhow!("Target branch or commit required"))?;
                commands::checkout::execute(&ctx, &target_ref, force, dry_run)?;
            }
        }
        Commands::Reset {
            commit,
            hard,
            soft,
            mixed,
            keep,
            dry_run,
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
                    dry_run,
                },
                &paths,
            )?;
        }
        Commands::Revert {
            commit,
            no_edit,
            force,
            dry_run,
        } => {
            let ctx = context.context("Context not initialized for revert command")?;
            commands::revert::execute(&ctx, &commit, no_edit, force, dry_run)?;
        }
        Commands::Restore {
            paths,
            source,
            dry_run,
        } => {
            let ctx = context.context("Context not initialized for restore command")?;
            commands::restore::execute(&ctx, &paths, Some(&source), dry_run)?;
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
            dry_run,
        } => {
            let ctx = context.context("Context not initialized for merge command")?;
            commands::merge::execute(&ctx, &branch, no_ff, squash, message.as_deref(), dry_run)?;
        }
        Commands::Rebase {
            upstream,
            branch,
            r#continue,
            abort,
            skip,
        } => {
            let ctx = context.context("Context not initialized for rebase command")?;
            commands::rebase::execute(
                &ctx,
                upstream.as_deref(),
                branch.as_deref(),
                r#continue,
                abort,
                skip,
            )?;
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
            refs,
            paths,
            limit,
            oneline,
            all,
        } => {
            let ctx = context.context("Context not initialized for log command")?;
            commands::log::execute(&ctx, &refs, &paths, limit, oneline, all)?;
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
                commands::checkout::execute(&ctx, &branch_name, false, false)?;
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
        Commands::Fsck => {
            let ctx = context.context("Context not initialized for fsck command")?;
            commands::fsck::execute(&ctx)?;
        }
    }

    Ok(())
}

fn print_completions<G: Generator>(g: G, cmd: &mut clap::Command) {
    generate(g, cmd, cmd.get_name().to_string(), &mut io::stdout());
}
