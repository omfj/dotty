use colored::Colorize;

use crate::{
    config::Action,
    error::DottyError,
    utils::{Absolute, ExpandTilde},
};

pub mod config;
pub mod error;
pub mod lua;
pub mod utils;

pub use crate::config::DottyConfig;

pub struct Dotty {
    pub config: DottyConfig,
}

impl Dotty {
    pub fn new(config: DottyConfig) -> Self {
        Dotty { config }
    }

    pub fn install(&self) -> Result<(), DottyError> {
        for link in &self.config.links {
            let source = link.source.expand_tilde_path()?.absolute()?;
            let target = link.target.expand_tilde_path()?.absolute()?;

            if !source.exists() {
                println!(
                    "{} {} was not found, skipping.",
                    "Ignored:".yellow().bold(),
                    source.display()
                );
                continue;
            }

            if target.exists() {
                if self.config.overwrite {
                    if target.is_dir() {
                        std::fs::remove_dir_all(&target).map_err(DottyError::IoError)?;
                    } else {
                        std::fs::remove_file(&target).map_err(DottyError::IoError)?;
                    }
                } else {
                    println!(
                        "{} {} already exists, skipping. Use --overwrite to force.",
                        "Warning:".yellow().bold(),
                        target.display()
                    );
                    continue;
                }
            }

            if self.config.ask {
                use std::io::{self, Write};
                print!("Link {} -> {}? [y/N] ", source.display(), target.display());
                io::stdout().flush().map_err(DottyError::IoError)?;
                let mut input = String::new();
                io::stdin()
                    .read_line(&mut input)
                    .map_err(DottyError::IoError)?;
                if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
                    println!("{} Skipping.", "Skipped:".yellow().bold());
                    continue;
                }
            }

            if let Some(parent) = target.parent()
                && !parent.exists()
            {
                std::fs::create_dir_all(parent).map_err(DottyError::IoError)?;
            }

            utils::symlink(source.clone(), target.clone())?;

            println!(
                "{} {} -> {}",
                "Linked:".green().bold(),
                source.display(),
                target.display()
            );
        }

        for action in &self.config.actions {
            run_action(action)?;
        }

        Ok(())
    }

    pub fn remove(&self) -> Result<(), DottyError> {
        for link in &self.config.links {
            let target = link.target.expand_tilde_path()?.absolute()?;

            if !target.exists() {
                println!(
                    "{} {} does not exist, skipping.",
                    "Ignored:".yellow().bold(),
                    target.display()
                );
                continue;
            }

            if target.is_dir() {
                std::fs::remove_dir_all(&target).map_err(DottyError::IoError)?;
            } else {
                std::fs::remove_file(&target).map_err(DottyError::IoError)?;
            }

            println!(
                "{} {} removed.",
                "Removed:".green().bold(),
                target.display()
            );
        }

        Ok(())
    }

    pub fn status(&self) -> Result<(), DottyError> {
        for link in &self.config.links {
            let source = link.source.expand_tilde_path()?.absolute()?;
            let target = link.target.expand_tilde_path()?.absolute()?;

            if !source.exists() {
                print!("{}", "[SOURCE MISSING]".red().bold());
            } else if !target.exists() {
                print!("{}", "[NOT LINKED]".yellow().bold());
            } else if target.is_symlink() {
                match target.read_link() {
                    Ok(actual) if actual == source => print!("{}", "[OK]".green().bold()),
                    Ok(actual) => print!(
                        "{} (points to {})",
                        "[WRONG TARGET]".red().bold(),
                        actual.display()
                    ),
                    Err(_) => print!("{}", "[SYMLINK ERROR]".red().bold()),
                }
            } else {
                print!("{}", "[EXISTS BUT NOT SYMLINK]".yellow().bold());
            }

            println!(" {} -> {}", source.display(), target.display());
        }

        if !self.config.actions.is_empty() {
            println!();
            println!("{}", "Actions:".blue().bold());
            println!();
            for action in &self.config.actions {
                println!("{} {}", "[READY]".green().bold(), action.name);
            }
        }

        Ok(())
    }
}

fn run_action(action: &Action) -> Result<(), DottyError> {
    println!("{} Running: {}", "Action:".blue().bold(), action.name);

    let shell = action.shell.as_str();
    let shell_bin = if shell.is_empty() { "sh" } else { shell };

    let output = std::process::Command::new(shell_bin)
        .arg("-c")
        .arg(&action.command)
        .output()
        .map_err(DottyError::IoError)?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.trim().is_empty() {
            println!("{} {}", "Success:".green().bold(), stdout.trim());
        } else {
            println!("{} {}", "Success:".green().bold(), action.name);
        }
        Ok(())
    } else {
        Err(DottyError::CommandError {
            command: action.name.clone(),
            message: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DottyConfig, Link};
    use std::fs;
    use tempfile::TempDir;

    fn config_with_links(links: Vec<Link>) -> DottyConfig {
        DottyConfig {
            links,
            ..Default::default()
        }
    }

    #[test]
    fn test_install_basic_link() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");
        fs::write(&source, "content").unwrap();

        let dotty = Dotty::new(config_with_links(vec![Link {
            source: source.to_string_lossy().to_string(),
            target: target.to_string_lossy().to_string(),
        }]));

        dotty.install().unwrap();
        assert!(target.exists());
        assert_eq!(fs::read_to_string(&target).unwrap(), "content");
    }

    #[test]
    fn test_install_missing_source_skips() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.txt");

        let dotty = Dotty::new(config_with_links(vec![Link {
            source: "/nonexistent/source.txt".to_string(),
            target: target.to_string_lossy().to_string(),
        }]));

        dotty.install().unwrap();
        assert!(!target.exists());
    }

    #[test]
    fn test_remove_existing_file() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.txt");
        fs::write(&target, "content").unwrap();

        let dotty = Dotty::new(config_with_links(vec![Link {
            source: "source.txt".to_string(),
            target: target.to_string_lossy().to_string(),
        }]));

        dotty.remove().unwrap();
        assert!(!target.exists());
    }

    #[test]
    fn test_install_creates_parent_dirs() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("nested/deep/target.txt");
        fs::write(&source, "content").unwrap();

        let dotty = Dotty::new(config_with_links(vec![Link {
            source: source.to_string_lossy().to_string(),
            target: target.to_string_lossy().to_string(),
        }]));

        dotty.install().unwrap();
        assert!(target.exists());
    }
}
