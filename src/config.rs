use crate::cli::Args;
use std::path::PathBuf;

/// Resolved, validated configuration derived from CLI arguments.
#[derive(Debug, Clone)]
pub struct Config {
    /// Paths to scan.
    pub paths: Vec<PathBuf>,
}

impl Config {
    /// Build a `Config` from parsed CLI arguments.
    pub fn from_args(args: Args) -> anyhow::Result<Self> {
        Ok(Self { paths: args.paths })
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
}
