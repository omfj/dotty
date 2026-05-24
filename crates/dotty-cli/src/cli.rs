use clap::Parser;
use colored::Colorize;
use dotty_parser::{Context, DottyConfig};

use dotty_cli::Dotty;

#[derive(Parser, Debug)]
pub struct Cli {
    /// Path to the dottyfile
    #[clap(short, long, default_value = "dottyfile")]
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
        /// Profile to use
        #[clap(short, long)]
        profile: Option<String>,
    },
    /// Remove all symlinks registered in the dottyfile
    Remove {
        /// Profile to use
        #[clap(short, long)]
        profile: Option<String>,
    },
    /// Show the status of all configured links
    Status {
        /// Profile to use
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
                let steps = load_steps(&config_path, profile)?;
                let dotty = Dotty::new(steps).with_overwrite(overwrite).with_ask(ask);
                dotty.install().map_err(|e| {
                    eprintln!("{} {}", "Error:".red().bold(), e);
                    anyhow::anyhow!("{}", e)
                })?;
                println!("{}", "Installation completed successfully.".green());
                Ok(())
            }
            Command::Remove { profile } => {
                let steps = load_steps(&config_path, profile)?;
                Dotty::new(steps).remove().map_err(|e| {
                    eprintln!("{} {}", "Error:".red().bold(), e);
                    anyhow::anyhow!("{}", e)
                })?;
                println!("{}", "Removal completed successfully.".green());
                Ok(())
            }
            Command::Status { profile } => {
                let steps = load_steps(&config_path, profile)?;
                Dotty::new(steps).status().map_err(|e| {
                    eprintln!("{} {}", "Error:".red().bold(), e);
                    anyhow::anyhow!("{}", e)
                })?;
                Ok(())
            }
        }
    }
}

fn load_steps(
    path: &std::path::Path,
    profile: Option<String>,
) -> anyhow::Result<Vec<dotty_parser::Step>> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read '{}': {}", path.display(), e))?;

    let mut ctx = Context::current().map_err(|e| {
        eprintln!("{} {}", "Error:".red().bold(), e);
        e
    })?;

    if let Some(profile) = profile {
        ctx = ctx.with_profile(profile);
    }

    DottyConfig::parse_with_context(&source, ctx)
        .map(|c| c.steps)
        .map_err(|e| {
            eprintln!("{} {}", "Error:".red().bold(), e);
            e
        })
}
