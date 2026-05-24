use clap::Parser;
use colored::Colorize;

use dotty_cli::{Dotty, DottyConfig};

#[derive(Parser, Debug)]
pub struct Cli {
    /// Path to the Lua configuration file
    #[clap(short, long, default_value = "dotty.lua")]
    pub config: std::path::PathBuf,
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Parser, Debug)]
pub enum Command {
    /// Link dotfiles to their respective locations
    Install {
        /// Override existing links if they already exist
        #[clap(short, long, default_value = "false")]
        overwrite: bool,
        /// Ask before creating each symlink
        #[clap(short, long, default_value = "false")]
        ask: bool,
        /// Profile to pass to the Lua config
        #[clap(short, long)]
        profile: Option<String>,
    },
    /// Remove all links registered by the Lua config
    Remove {
        /// Profile to pass to the Lua config
        #[clap(short, long)]
        profile: Option<String>,
    },
    /// Show the status of all configured links
    Status {
        /// Profile to pass to the Lua config
        #[clap(short, long)]
        profile: Option<String>,
    },
}

impl Cli {
    pub fn run(self) -> anyhow::Result<()> {
        let config_path = self.config.clone();

        match self.command {
            Command::Install {
                overwrite,
                ask,
                profile,
            } => {
                let config = load_config(&config_path, profile)?
                    .with_overwrite(overwrite)
                    .with_ask(ask);
                handle_install(Dotty::new(config))
            }
            Command::Remove { profile } => {
                let config = load_config(&config_path, profile)?;
                handle_remove(Dotty::new(config))
            }
            Command::Status { profile } => {
                let config = load_config(&config_path, profile)?;
                handle_status(Dotty::new(config))
            }
        }
    }
}

fn load_config(path: &std::path::Path, profile: Option<String>) -> anyhow::Result<DottyConfig> {
    let base = DottyConfig::default().with_profile(profile);
    dotty_cli::lua::load(path, base).map_err(|e| {
        eprintln!("{} {}", "Error:".red().bold(), e);
        e
    })
}

fn handle_install(dotty: Dotty) -> anyhow::Result<()> {
    dotty.install().map_err(|e| {
        eprintln!("{} {}", "Error:".red().bold(), e);
        anyhow::anyhow!("{}", e)
    })?;
    println!("{}", "Installation completed successfully.".green());
    Ok(())
}

fn handle_remove(dotty: Dotty) -> anyhow::Result<()> {
    dotty.remove().map_err(|e| {
        eprintln!("{} {}", "Error:".red().bold(), e);
        anyhow::anyhow!("{}", e)
    })?;
    println!("{}", "Removal completed successfully.".green());
    Ok(())
}

fn handle_status(dotty: Dotty) -> anyhow::Result<()> {
    dotty.status().map_err(|e| {
        eprintln!("{} {}", "Error:".red().bold(), e);
        anyhow::anyhow!("{}", e)
    })?;
    Ok(())
}
