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
