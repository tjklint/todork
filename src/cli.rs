use clap::{Parser, ValueEnum};
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
    after_help = "Examples:\n  todork .\n  todork src/ --format json\n  todork . --tags todo,fixme\n  todork . --include '*.rs' --exclude 'tests/*'"
)]
pub struct Args {
    /// Paths to scan (files or directories). Defaults to the current directory.
    #[arg(default_value = ".")]
    pub paths: Vec<PathBuf>,

    /// Output format.
    #[arg(long, short = 'f', value_enum, default_value = "text")]
    pub format: Format,

    /// Comma-separated list of tags to scan for (case-insensitive).
    /// Defaults to all built-in tags: TODO,FIXME,HACK,XXX,NOTE,OPTIMIZE,BUG,DEPRECATED
    #[arg(long, short = 't', value_delimiter = ',')]
    pub tags: Vec<String>,

    /// Only scan files matching this glob pattern (repeatable).
    #[arg(long, short = 'i')]
    pub include: Vec<String>,

    /// Skip files matching this glob pattern (repeatable).
    #[arg(long, short = 'e')]
    pub exclude: Vec<String>,

    /// Disable .gitignore / .ignore file respecting.
    #[arg(long)]
    pub no_gitignore: bool,

    /// Include hidden files and directories (those starting with '.').
    #[arg(long)]
    pub hidden: bool,

    /// Maximum directory traversal depth.
    #[arg(long)]
    pub max_depth: Option<usize>,

    /// Skip files larger than this many bytes.
    #[arg(long)]
    pub max_filesize: Option<u64>,

    /// Number of parallel threads. Defaults to the number of logical CPUs.
    #[arg(long)]
    pub threads: Option<usize>,

    /// When to use colour in text output.
    #[arg(long, value_enum, default_value = "auto")]
    pub color: ColorWhen,

    /// Always exit with code 0, even when annotations are found.
    /// Useful for non-blocking CI annotation reporting.
    #[arg(long)]
    pub exit_zero: bool,

    /// Enrich each finding with git blame data: who committed the line and when.
    /// Requires the scanned path to be inside a git repository.
    /// No-ops silently on non-git directories.
    #[arg(long)]
    pub blame: bool,
}

/// Output format selection.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// Human-readable coloured text (default).
    Text,
    /// Fixed-width aligned table.
    Table,
    /// JSON array of finding objects.
    Json,
    /// GitHub Actions workflow commands (::warning / ::error / ::notice).
    #[value(name = "github-annotations")]
    GithubAnnotations,
}

/// Colour output control.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorWhen {
    /// Emit colour codes only when stdout is a TTY and NO_COLOR is unset.
    Auto,
    /// Always emit colour codes.
    Always,
    /// Never emit colour codes.
    Never,
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

    #[test]
    fn default_format_is_text() {
        let args = Args::parse_from(["todork"]);
        assert_eq!(args.format, Format::Text);
    }

    #[test]
    fn format_json_parsed() {
        let args = Args::parse_from(["todork", "--format", "json"]);
        assert_eq!(args.format, Format::Json);
    }

    #[test]
    fn format_github_annotations_parsed() {
        let args = Args::parse_from(["todork", "--format", "github-annotations"]);
        assert_eq!(args.format, Format::GithubAnnotations);
    }

    #[test]
    fn tags_parsed_as_comma_list() {
        let args = Args::parse_from(["todork", "--tags", "todo,fixme"]);
        assert_eq!(args.tags, vec!["todo", "fixme"]);
    }

    #[test]
    fn include_glob_repeatable() {
        let args = Args::parse_from(["todork", "--include", "*.rs", "--include", "*.py"]);
        assert_eq!(args.include, vec!["*.rs", "*.py"]);
    }

    #[test]
    fn exclude_glob_repeatable() {
        let args = Args::parse_from(["todork", "--exclude", "target/*"]);
        assert_eq!(args.exclude, vec!["target/*"]);
    }

    #[test]
    fn no_gitignore_flag() {
        let args = Args::parse_from(["todork", "--no-gitignore"]);
        assert!(args.no_gitignore);
    }

    #[test]
    fn hidden_flag() {
        let args = Args::parse_from(["todork", "--hidden"]);
        assert!(args.hidden);
    }

    #[test]
    fn max_depth_parsed() {
        let args = Args::parse_from(["todork", "--max-depth", "3"]);
        assert_eq!(args.max_depth, Some(3));
    }

    #[test]
    fn max_filesize_parsed() {
        let args = Args::parse_from(["todork", "--max-filesize", "1048576"]);
        assert_eq!(args.max_filesize, Some(1_048_576));
    }

    #[test]
    fn threads_parsed() {
        let args = Args::parse_from(["todork", "--threads", "4"]);
        assert_eq!(args.threads, Some(4));
    }

    #[test]
    fn color_never_parsed() {
        let args = Args::parse_from(["todork", "--color", "never"]);
        assert_eq!(args.color, ColorWhen::Never);
    }

    #[test]
    fn color_always_parsed() {
        let args = Args::parse_from(["todork", "--color", "always"]);
        assert_eq!(args.color, ColorWhen::Always);
    }

    #[test]
    fn exit_zero_flag() {
        let args = Args::parse_from(["todork", "--exit-zero"]);
        assert!(args.exit_zero);
    }

    #[test]
    fn blame_default_false() {
        let args = Args::parse_from(["todork"]);
        assert!(!args.blame);
    }

    #[test]
    fn blame_flag() {
        let args = Args::parse_from(["todork", "--blame"]);
        assert!(args.blame);
    }
}
