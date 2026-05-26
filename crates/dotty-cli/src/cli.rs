use clap::Parser;
use colored::Colorize;
use dotty_parser::{Context, DottyConfig};

use dotty_cli::Dotty;

#[derive(Parser, Debug)]
pub struct Cli {
    /// Path to the dottyfile (walks up directories if not specified)
    #[clap(short, long)]
    pub config: Option<std::path::PathBuf>,
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
        let config_path = match self.config {
            Some(p) => p,
            None => find_dottyfile()?,
        };

        // Change to the dottyfile's directory so relative paths in the file resolve correctly
        if let Some(dir) = config_path.parent()
            && !dir.as_os_str().is_empty()
        {
            std::env::set_current_dir(dir)
                .map_err(|e| anyhow::anyhow!("Failed to chdir to '{}': {}", dir.display(), e))?;
        }

        let config_path = config_path
            .file_name()
            .map(std::path::PathBuf::from)
            .unwrap_or(config_path);

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

fn find_dottyfile() -> anyhow::Result<std::path::PathBuf> {
    let mut dir = std::env::current_dir()
        .map_err(|e| anyhow::anyhow!("Failed to get current directory: {}", e))?;

    loop {
        let dottyfile = dir.join("dottyfile");
        if dottyfile.exists() {
            return Ok(dottyfile);
        }
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => {
                return Err(anyhow::anyhow!(
                    "No dottyfile found in current or parent directories"
                ));
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
