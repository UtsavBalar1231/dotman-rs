//! xtask for dotman - build automation and tooling
//!
//! This binary provides development tasks like man page generation.

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "xtask", about = "Build automation for dotman")]
enum Task {
    /// Generate man pages from clap definitions
    GenerateManPages {
        /// Output directory for man pages (default: ./man)
        #[arg(short, long, default_value = "man")]
        output: PathBuf,
    },
}

fn main() -> Result<()> {
    let task = Task::parse();

    match task {
        Task::GenerateManPages { output } => generate_man_pages(&output)?,
    }

    Ok(())
}

fn generate_man_pages(output_dir: &PathBuf) -> Result<()> {
    println!("Generating man pages...");

    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir)
        .with_context(|| format!("Failed to create directory: {}", output_dir.display()))?;

    // Get the clap Command from dotman's CLI
    let mut cmd = dotman::cli::Cli::command();

    // Generate the main man page for dot(1)
    let man_path = output_dir.join("dot.1");
    let man_file = fs::File::create(&man_path)
        .with_context(|| format!("Failed to create man page: {}", man_path.display()))?;

    clap_mangen::Man::new(cmd.clone()).render(&mut std::io::BufWriter::new(man_file))?;

    println!("✓ Generated: {}", man_path.display());

    // Optionally generate man pages for subcommands
    // For now, we'll include everything in the main man page
    // Future enhancement: Generate separate pages for complex subcommands

    // Example: Generate man pages for major subcommands
    let subcommands = ["add", "commit", "status", "push", "pull", "merge"];
    for subcmd_name in &subcommands {
        if let Some(subcmd) = cmd.find_subcommand_mut(subcmd_name) {
            let subcmd_man_path = output_dir.join(format!("dot-{}.1", subcmd_name));
            let subcmd_man_file = fs::File::create(&subcmd_man_path).with_context(|| {
                format!(
                    "Failed to create subcommand man page: {}",
                    subcmd_man_path.display()
                )
            })?;

            clap_mangen::Man::new(subcmd.clone())
                .render(&mut std::io::BufWriter::new(subcmd_man_file))?;

            println!("✓ Generated: {}", subcmd_man_path.display());
        }
    }

    println!(
        "\nMan pages successfully generated in: {}",
        output_dir.display()
    );
    println!("\nTo view the man pages:");
    println!("  man {}/dot.1", output_dir.display());
    println!("\nTo install system-wide (requires root):");
    println!(
        "  sudo cp {}/*.1 /usr/share/man/man1/",
        output_dir.display()
    );
    println!("  sudo mandb");

    Ok(())
}
