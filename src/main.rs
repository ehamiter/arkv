mod config;
mod setup;
mod transfer;

use anyhow::Result;
use clap::Parser;
use config::Config;
use dialoguer::Select;
use transfer::{Transferer, TransferStats};

#[derive(Parser)]
#[command(name = "arkv")]
#[command(about = "Archive files to remote servers via SFTP", long_about = None)]
struct Cli {
    #[arg(help = "File or folder to archive")]
    path: Option<String>,

    #[arg(long, help = "Re-run the setup wizard")]
    setup: bool,

    #[arg(short, long, help = "Select destination interactively")]
    interactive: bool,

    #[arg(short, long, help = "Enable verbose logging")]
    verbose: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.setup {
        setup::run_setup()?;
        return Ok(());
    }

    let config = match Config::load()? {
        Some(cfg) => cfg,
        None => {
            println!("No configuration found. Running setup...\n");
            setup::run_setup()?
        }
    };

    if config.destinations.is_empty() {
        eprintln!("Error: No destinations configured. Run 'arkv --setup' to add one.");
        std::process::exit(1);
    }

    match cli.path {
        Some(path) => {
            let destinations = if cli.interactive {
                let names: Vec<String> = config.destinations.iter()
                    .map(|d| format!("{} ({})", d.name, d.host))
                    .collect();

                let selection = Select::new()
                    .with_prompt("Select destination")
                    .items(&names)
                    .default(0)
                    .interact()?;

                vec![&config.destinations[selection]]
            } else {
                config.destinations.iter().collect()
            };

            if destinations.len() > 1 {
                println!("\n📦 Archiving to {} destinations\n", destinations.len());
            } else {
                println!("\n📦 Archiving to {} ({})\n", destinations[0].name, destinations[0].host);
            }

            use std::thread;
            let handles: Vec<_> = destinations.into_iter().map(|destination| {
                let dest = destination.clone();
                let path_clone = path.clone();
                let ssh_key_path = config.ssh_key_path.clone();
                let verbose = cli.verbose;
                
                thread::spawn(move || {
                    let transferer = Transferer::new(dest.clone(), verbose);
                    transferer.transfer(&path_clone, &ssh_key_path)
                        .map(|stats| (dest.name.clone(), stats))
                })
            }).collect();

            let mut errors = Vec::new();
            let mut all_stats: Vec<(String, TransferStats)> = Vec::new();
            
            for handle in handles {
                match handle.join() {
                    Ok(Ok((name, stats))) => {
                        println!("✓ Completed upload to {}", name);
                        all_stats.push((name, stats));
                    }
                    Ok(Err(e)) => errors.push(e),
                    Err(_) => errors.push(anyhow::anyhow!("Thread panicked")),
                }
            }

            if !errors.is_empty() {
                eprintln!("\n❌ Errors occurred:");
                for error in errors {
                    eprintln!("  {}", error);
                }
                std::process::exit(1);
            }

            println!();
            for (name, stats) in &all_stats {
                let mb = stats.bytes_transferred as f64 / 1_048_576.0;
                let speed = mb / stats.duration_secs;
                println!("📊 {}: {:.2} MB in {:.1}s ({:.2} MB/s)", 
                    name, mb, stats.duration_secs, speed);
            }

            println!("\n✨ Done!\n");
        }
        None => {
            print_usage();
        }
    }

    Ok(())
}

fn print_usage() {
    println!(r#"
arkv - Archive files to remote servers

USAGE:
    arkv <FILE_OR_FOLDER>    Upload a file or folder
    arkv --setup             Run setup wizard
    arkv --help              Show detailed help

EXAMPLES:
    arkv cool-picture.png              Upload a single file
    arkv my_files/tuesday/             Upload a folder and its contents
    arkv document.pdf --interactive    Choose destination interactively

Get started by running: arkv --setup
"#);
}
