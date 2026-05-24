use colored::Colorize;

pub trait ExpandTilde {
    fn expand_tilde_path(&self) -> Result<std::path::PathBuf, String>;
}

impl<P: AsRef<std::path::Path>> ExpandTilde for P {
    fn expand_tilde_path(&self) -> Result<std::path::PathBuf, String> {
        let path_str = self.as_ref().to_string_lossy().to_string();
        if path_str.starts_with('~') {
            if let Some(home_dir) = dirs::home_dir() {
                let relative_path = path_str.strip_prefix('~').unwrap_or(&path_str);
                Ok(home_dir.join(relative_path.trim_start_matches('/')))
            } else {
                Err("Home directory not found".to_string())
            }
        } else {
            Ok(self.as_ref().to_path_buf())
        }
    }
}

pub trait Absolute {
    fn absolute(&self) -> Result<std::path::PathBuf, String>;
}

impl<P: AsRef<std::path::Path>> Absolute for P {
    fn absolute(&self) -> Result<std::path::PathBuf, String> {
        let path = self.as_ref();
        if path.is_absolute() {
            Ok(path.to_path_buf())
        } else {
            std::env::current_dir()
                .map_err(|e| e.to_string())
                .map(|current_dir| current_dir.join(path))
        }
    }
}

pub fn get_os_name() -> String {
    match std::env::consts::OS {
        "linux" => "linux".to_string(),
        "macos" => "macos".to_string(),
        "windows" => "windows".to_string(),
        os => {
            eprintln!(
                "{} Unsupported operating system '{}'.",
                "Error:".red().bold(),
                os
            );
            os.to_string()
        }
    }
}

pub fn symlink<P: AsRef<std::path::Path>>(source: P, target: P) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(source, target)
    }
    #[cfg(windows)]
    {
        if source.as_ref().is_dir() {
            std::os::windows::fs::symlink_dir(source, target)
        } else {
            std::os::windows::fs::symlink_file(source, target)
        }
    }
}

pub fn is_on_path(cmd: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|dir| {
                let candidate = dir.join(cmd);
                candidate.is_file() && is_executable(&candidate)
            })
        })
        .unwrap_or(false)
}

#[cfg(unix)]
fn is_executable(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(_path: &std::path::Path) -> bool {
    true
}

pub fn get_hostname() -> String {
    std::process::Command::new("hostname")
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}
