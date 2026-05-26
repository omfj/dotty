use colored::Colorize;
use dotty_parser::{Action, Chmod, Clone, Copy, Link, Step};

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
                Step::CreateDir(path) => {
                    std::fs::create_dir_all(path).map_err(DottyError::IoError)?;
                    println!("{} {}", "[CREATED]".cyan().bold(), path);
                }
                Step::Clone(clone) => run_clone(clone)?,
                Step::Copy(copy) => run_copy(copy)?,
                Step::Chmod(chmod) => run_chmod(chmod)?,
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

        utils::symlink(source.clone(), target.clone())?;

        println!(
            "{} {} -> {}",
            "[LINKED]".green().bold(),
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
                        "[IGNORED]".yellow().bold(),
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
                    "[REMOVED]".green().bold(),
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

        let dirs: Vec<&String> = self
            .steps
            .iter()
            .filter_map(|s| match s {
                Step::CreateDir(p) => Some(p),
                _ => None,
            })
            .collect();

        if !dirs.is_empty() {
            println!();
            println!("{}", "Directories:".blue().bold());
            println!();
            for path in dirs {
                let status = if std::path::Path::new(path).exists() {
                    "[EXISTS]".green().bold()
                } else {
                    "[MISSING]".yellow().bold()
                };
                println!("{} {}", status, path);
            }
        }

        let clones: Vec<&Clone> = self
            .steps
            .iter()
            .filter_map(|s| match s {
                Step::Clone(c) => Some(c),
                _ => None,
            })
            .collect();

        if !clones.is_empty() {
            println!();
            println!("{}", "Clones:".blue().bold());
            println!();
            for clone in clones {
                let status = if std::path::Path::new(&clone.destination).exists() {
                    "[EXISTS]".green().bold()
                } else {
                    "[MISSING]".yellow().bold()
                };
                println!("{} {} -> {}", status, clone.url, clone.destination);
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

fn run_clone(clone: &Clone) -> Result<(), DottyError> {
    if std::path::Path::new(&clone.destination).exists() {
        println!(
            "{} {} already exists, skipping.",
            "[IGNORED]".yellow().bold(),
            clone.destination
        );
        return Ok(());
    }

    println!(
        "{} {} -> {}",
        "[CLONING]".cyan().bold(),
        clone.url,
        clone.destination
    );

    let output = std::process::Command::new("git")
        .args(["clone", &clone.url, &clone.destination])
        .output()
        .map_err(DottyError::IoError)?;

    if output.status.success() {
        println!("{} {}", "[CLONED]".green().bold(), clone.destination);
        Ok(())
    } else {
        Err(DottyError::CommandError {
            command: format!("git clone {} {}", clone.url, clone.destination),
            message: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

fn run_copy(copy: &Copy) -> Result<(), DottyError> {
    use crate::utils::{Absolute, ExpandTilde};

    let source = copy.source.expand_tilde_path()?.absolute()?;
    let destination = copy.destination.expand_tilde_path()?.absolute()?;

    if !source.exists() {
        println!(
            "{} {} was not found, skipping.",
            "[IGNORED]".yellow().bold(),
            source.display()
        );
        return Ok(());
    }

    std::fs::copy(&source, &destination).map_err(DottyError::IoError)?;
    println!(
        "{} {} -> {}",
        "[COPIED]".green().bold(),
        source.display(),
        destination.display()
    );
    Ok(())
}

fn run_chmod(chmod: &Chmod) -> Result<(), DottyError> {
    use crate::utils::{Absolute, ExpandTilde};
    use std::os::unix::fs::PermissionsExt;

    let path = chmod.path.expand_tilde_path()?.absolute()?;
    let mode = u32::from_str_radix(&chmod.mode, 8).map_err(|_| DottyError::CommandError {
        command: format!("chmod {} {}", chmod.mode, path.display()),
        message: format!("invalid mode '{}'", chmod.mode),
    })?;

    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode))
        .map_err(DottyError::IoError)?;

    println!(
        "{} {} ({})",
        "[CHMOD]".green().bold(),
        path.display(),
        chmod.mode
    );
    Ok(())
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
        use dotty_parser::{Context, DottyConfig};

        let dir = TempDir::new().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("nested/deep/target.txt");
        fs::write(&source, "content").unwrap();

        let config = format!("link \"{}\" to \"{}\"", source.display(), target.display());
        let ctx = Context::current().unwrap();
        let steps = DottyConfig::parse_with_context(&config, ctx).unwrap().steps;

        Dotty::new(steps).install().unwrap();
        assert!(target.exists());
    }
}
