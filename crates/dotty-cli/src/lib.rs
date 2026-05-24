use colored::Colorize;
use dotty_parser::{Action, Link, Step};

use crate::{
    error::DottyError,
    utils::{Absolute, ExpandTilde},
};

pub mod error;
pub mod utils;

pub struct Dotty {
    steps: Vec<Step>,
    overwrite: bool,
    ask: bool,
}

impl Dotty {
    pub fn new(steps: Vec<Step>) -> Self {
        Self {
            steps,
            overwrite: false,
            ask: false,
        }
    }

    pub fn with_overwrite(mut self, overwrite: bool) -> Self {
        self.overwrite = overwrite;
        self
    }

    pub fn with_ask(mut self, ask: bool) -> Self {
        self.ask = ask;
        self
    }

    pub fn install(&self) -> Result<(), DottyError> {
        for step in &self.steps {
            match step {
                Step::Link(link) => self.install_link(link)?,
                Step::Action(action) => run_action(action)?,
            }
        }
        Ok(())
    }

    fn install_link(&self, link: &Link) -> Result<(), DottyError> {
        let source = link.source.expand_tilde_path()?.absolute()?;
        let target = link.destination.expand_tilde_path()?.absolute()?;

        if !source.exists() {
            println!(
                "{} {} was not found, skipping.",
                "[IGNORED]".yellow().bold(),
                source.display()
            );
            return Ok(());
        }

        if target.exists() {
            if self.overwrite {
                if target.is_dir() {
                    std::fs::remove_dir_all(&target).map_err(DottyError::IoError)?;
                } else {
                    std::fs::remove_file(&target).map_err(DottyError::IoError)?;
                }
            } else {
                println!(
                    "{} {} already exists, skipping. Use --overwrite to force.",
                    "[WARNING]".yellow().bold(),
                    target.display()
                );
                return Ok(());
            }
        }

        if self.ask {
            use std::io::{self, Write};
            print!("Link {} -> {}? [y/N] ", source.display(), target.display());
            io::stdout().flush().map_err(DottyError::IoError)?;
            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .map_err(DottyError::IoError)?;
            if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
                println!("{} Skipping.", "Skipped:".yellow().bold());
                return Ok(());
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

        Ok(())
    }

    pub fn remove(&self) -> Result<(), DottyError> {
        for step in &self.steps {
            if let Step::Link(link) = step {
                let target = link.destination.expand_tilde_path()?.absolute()?;

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
        }
        Ok(())
    }

    pub fn status(&self) -> Result<(), DottyError> {
        let actions: Vec<&Action> = self
            .steps
            .iter()
            .filter_map(|s| match s {
                Step::Action(a) => Some(a),
                _ => None,
            })
            .collect();

        for step in &self.steps {
            if let Step::Link(link) = step {
                let source = link.source.expand_tilde_path()?.absolute()?;
                let target = link.destination.expand_tilde_path()?.absolute()?;

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
        }

        if !actions.is_empty() {
            println!();
            println!("{}", "Actions:".blue().bold());
            println!();
            for action in actions {
                println!("{} {}", "[READY]".green().bold(), action.command);
            }
        }

        Ok(())
    }
}

fn run_action(action: &Action) -> Result<(), DottyError> {
    println!("{} {}", "Action:".blue().bold(), action.command);

    let shell_bin = if action.shell.is_empty() {
        "sh"
    } else {
        &action.shell
    };

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
            println!("{} {}", "Success:".green().bold(), action.command);
        }
        Ok(())
    } else {
        Err(DottyError::CommandError {
            command: action.command.clone(),
            message: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn link_step(source: &str, destination: &str) -> Step {
        Step::Link(Link {
            source: source.to_string(),
            destination: destination.to_string(),
        })
    }

    #[test]
    fn test_install_basic_link() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");
        fs::write(&source, "content").unwrap();

        let dotty = Dotty::new(vec![link_step(
            &source.to_string_lossy(),
            &target.to_string_lossy(),
        )]);

        dotty.install().unwrap();
        assert!(target.exists());
        assert_eq!(fs::read_to_string(&target).unwrap(), "content");
    }

    #[test]
    fn test_install_missing_source_skips() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.txt");

        let dotty = Dotty::new(vec![link_step(
            "/nonexistent/source.txt",
            &target.to_string_lossy(),
        )]);

        dotty.install().unwrap();
        assert!(!target.exists());
    }

    #[test]
    fn test_remove_existing_file() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.txt");
        fs::write(&target, "content").unwrap();

        let dotty = Dotty::new(vec![link_step("source.txt", &target.to_string_lossy())]);

        dotty.remove().unwrap();
        assert!(!target.exists());
    }

    #[test]
    fn test_install_creates_parent_dirs() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("nested/deep/target.txt");
        fs::write(&source, "content").unwrap();

        let dotty = Dotty::new(vec![link_step(
            &source.to_string_lossy(),
            &target.to_string_lossy(),
        )]);

        dotty.install().unwrap();
        assert!(target.exists());
    }
}
