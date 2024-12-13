use clap::{Args, Parser, Subcommand};
use std::fmt;

#[derive(Parser)]
#[command(name = "dotman")]
#[command(author = "Utsav Balar")]
#[command(version, about, long_about)]
pub struct DotmanArgs {
    /// Provide custom path to the config file (default: ${pwd}/config.ron)
    #[clap(short, long)]
    pub config_path: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Force push configs from dotconfigs directory into your local system
    #[clap(short_flag = 'p')]
    LocalPull,

    /// Force pull configs from your local system into the dotconfigs directory
    #[clap(short_flag = 'u')]
    LocalPush,

    /// Update your dotconfigs directory with the latest configs
    #[clap(short_flag = 'P')]
    ForcePull,

    /// Update your local system configs with the configs from the dotconfigs directory
    #[clap(short_flag = 'U')]
    ForcePush,

    /// Clear the metadata of config entries in the sync-dotfiles config
    #[clap(short_flag = 'x')]
    ClearMetadata,

    /// Prints a new sync-dotfiles configuration
    #[clap(name = "new", short_flag = 'n')]
    PrintNew,

    /// Prints the currently used sync-dotfiles config file
    #[clap(name = "printconf", short_flag = 'r')]
    PrintConfig,

    /// Fix your sync-dotfiles config file for any errors
    #[clap(short_flag = 'z')]
    FixConfig,

    /// Adds a new config entry to your exisiting sync-dotfiles config
    #[clap(short_flag = 'a')]
    #[command(arg_required_else_help = true)]
    Add(AddArgs),

    /// Edit the sync-dotfiles config file
    #[clap(short_flag = 'e')]
    Edit,
}

#[derive(Args)]
pub struct AddArgs {
    /// The name of the config entry
    #[arg(short = 'n', long)]
    pub name: String,
    /// The path to the config entry
    #[arg(short = 'p', long)]
    pub path: String,
}

pub fn get_env_args() -> DotmanArgs {
    DotmanArgs::parse()
}

impl fmt::Display for Commands {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Commands::LocalPull => write!(f, "local pull"),
            Commands::LocalPush => write!(f, "local push"),
            Commands::ForcePull => write!(f, "force pull"),
            Commands::ForcePush => write!(f, "force push"),
            Commands::ClearMetadata => write!(f, "clear metadata"),
            Commands::PrintNew => write!(f, "print new"),
            Commands::PrintConfig => write!(f, "print config"),
            Commands::FixConfig => write!(f, "fix config"),
            Commands::Add(_) => write!(f, "add"),
            Commands::Edit => write!(f, "edit"),
        }
    }
}
