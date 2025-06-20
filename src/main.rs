use dotman_rs::cli::{parse_args, CommandHandler};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = parse_args();

    // Initialize logging based on verbosity
    let log_level = match args.verbose {
        0 => "warn",
        1 => "info", 
        2 => "debug",
        _ => "trace",
    };

    tracing_subscriber::fmt()
        .with_env_filter(log_level)
        .with_target(false)
        .init();

    // Initialize command handler
    let mut handler = CommandHandler::new(&args).await?;

    // Execute the command
    handler.execute(&args).await?;

    Ok(())
}
