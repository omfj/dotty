#[derive(Debug, thiserror::Error)]
pub enum DottyError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Command '{command}' failed: {message}")]
    CommandError { command: String, message: String },
    #[error("Path error: {0}")]
    PathError(String),
}

impl From<String> for DottyError {
    fn from(err_msg: String) -> Self {
        DottyError::PathError(err_msg)
    }
}
