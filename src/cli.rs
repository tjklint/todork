use clap::Parser;
use std::path::PathBuf;

/// todork — hyper-fast annotation scanner.
///
/// Scans source files for TODO, FIXME, HACK, and other annotation comments.
/// Exits 0 when annotations are found, 1 when none are found, 2 on error.
#[derive(Parser, Debug, Clone)]
#[command(
    name = "todork",
    version,
    about,
    long_about = None,
    after_help = "Examples:\n  todork .\n  todork src/ --format json\n  todork . --tags todo,fixme"
)]
pub struct Args {
    /// Paths to scan (files or directories). Defaults to the current directory.
    #[arg(default_value = ".")]
    pub paths: Vec<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_path_is_dot() {
        let args = Args::parse_from(["todork"]);
        assert_eq!(args.paths, vec![PathBuf::from(".")]);
    }

    #[test]
    fn explicit_paths_are_preserved() {
        let args = Args::parse_from(["todork", "src/", "tests/"]);
        assert_eq!(
            args.paths,
            vec![PathBuf::from("src/"), PathBuf::from("tests/")]
        );
    }
}
