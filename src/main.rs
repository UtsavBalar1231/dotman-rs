use args::Commands;
use dotman::config::*;
use std::io::{self};
use std::path::PathBuf;
mod args;

fn main() -> io::Result<()> {
    let env_args = args::get_env_args();
    let config_path: PathBuf = if let Some(custom_path) = env_args.config_path {
        PathBuf::from(custom_path)
    } else {
        dirs::config_dir()
            .expect("Cannot find config directory")
            .join("config.ron")
    };

    let args = env_args.command;

    if matches!(args, Commands::PrintNew) {
        Config::print_config(None, PrintConfigOpts::new_required())?;
        return Ok(());
    }

    if !config_path.exists() {
        eprintln!("Config file not found: {}", config_path.display());
        return Ok(());
    }

    let mut config = Config::load_config(&config_path)?;

    match &args {
        Commands::LocalPull => config.pull_config(false),
        Commands::LocalPush => config.push_config(false),
        Commands::ForcePull => config.pull_config(true),
        Commands::ForcePush => config.push_config(true),
        Commands::ClearMetadata => config.clear_config(),
        Commands::PrintNew => Config::print_config(None, PrintConfigOpts::new_required()),
        Commands::PrintConfig => Config::print_config(Some(&config), PrintConfigOpts::default()),
        Commands::FixConfig => config.fix_config(),
        Commands::Add(args::AddArgs { name, path }) => config.add_config(&name, &path),
        Commands::Edit => config.edit_config(),
    }?;

    println!("{} {} completed successfully.", env!("CARGO_PKG_NAME"), &args);
    Ok(())
}
