use crate::cli::Args;
use crate::scanner::DEFAULT_MAX_FILE_SIZE;
use std::path::PathBuf;

/// Resolved, validated configuration derived from CLI arguments.
#[derive(Debug, Clone)]
pub struct Config {
    /// Paths to scan.
    pub paths: Vec<PathBuf>,
    /// Maximum file size in bytes before a file is skipped.
    pub max_file_size: u64,
}

impl Config {
    /// Build a `Config` from parsed CLI arguments.
    pub fn from_args(args: Args) -> anyhow::Result<Self> {
        Ok(Self {
            paths: args.paths,
            max_file_size: DEFAULT_MAX_FILE_SIZE,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Args;
    use clap::Parser;

    #[test]
    fn config_from_default_args() {
        let args = Args::parse_from(["todork"]);
        let config = Config::from_args(args).unwrap();
        assert_eq!(config.paths, vec![PathBuf::from(".")]);
    }

    #[test]
    fn config_preserves_multiple_paths() {
        let args = Args::parse_from(["todork", "src/", "tests/"]);
        let config = Config::from_args(args).unwrap();
        assert_eq!(config.paths.len(), 2);
    }

    #[test]
    fn default_max_file_size_is_10mb() {
        let args = Args::parse_from(["todork"]);
        let config = Config::from_args(args).unwrap();
        assert_eq!(config.max_file_size, 10 * 1024 * 1024);
    }
}
