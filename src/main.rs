use dotman::errors::ConfigError;
use args::Commands;
use dotman::config::*;
use std::path::PathBuf;
mod args;


fn main() -> Result<(), ConfigError> {
    let env_args = args::get_env_args();
    let config_path: PathBuf = if let Some(custom_path) = env_args.config_path {
        PathBuf::from(custom_path)
    } else {
        let config_path_name = format!("{}/config.ron", env!("CARGO_PKG_NAME"));
        dirs::config_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap())
            .join(config_path_name)
    };

    let args = env_args.command;

    if matches!(args, Commands::PrintNew) {
        Config::print_config(None)?;
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
        Commands::PrintNew => Config::print_config(None),
        Commands::PrintConfig => Config::print_config(Some(&config)),
        Commands::FixConfig => config.fix_config(),
        Commands::Add(args::AddArgs { name, path }) => config.add_config(&name, &path),
        Commands::Edit => config.edit_config(),
        Commands::Clean => config.clean_configs(),
    }?;

    println!(
        "{} {} completed successfully.",
        env!("CARGO_PKG_NAME"),
        &args
    );
    Ok(())
}
