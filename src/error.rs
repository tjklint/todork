use thiserror::Error;

/// All errors that todork can produce.
#[derive(Debug, Error)]
pub enum TodorkError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("File walk error: {0}")]
    Walk(String),

    #[error("Invalid glob pattern '{pattern}': {reason}")]
    InvalidGlob { pattern: String, reason: String },

    #[error("Invalid tag '{0}': tags must be non-empty ASCII identifiers")]
    InvalidTag(String),
}
