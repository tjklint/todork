pub mod cli;
pub mod config;
pub mod error;
pub mod exit_code;
pub mod formatter;
pub mod matcher;
pub mod scanner;
pub mod walker;

use crate::config::Config;
use crate::exit_code::ExitCode;
use crate::formatter::text::TextFormatter;
use crate::matcher::{Matcher, DEFAULT_TAGS};
use crate::walker::walk_parallel;
use crossbeam_channel::bounded;
use std::sync::Arc;
use termcolor::{ColorChoice, StandardStream};

/// Run todork with the given configuration and write results to stdout.
///
/// Returns [`ExitCode::Success`] when annotations are found,
/// [`ExitCode::NotFound`] when none are found.
pub fn run(config: Config) -> anyhow::Result<ExitCode> {
    let matcher = Arc::new(Matcher::new(DEFAULT_TAGS)?);
    let (tx, rx) = bounded(256);

    // Walk in parallel on a background thread.
    let walk_config = config.clone();
    let walk_matcher = matcher.clone();
    let walk_handle = std::thread::spawn(move || walk_parallel(&walk_config, walk_matcher, tx));

    // Collect findings on the main thread while the walker runs.
    let mut all_findings: Vec<_> = rx.into_iter().flatten().collect();

    walk_handle
        .join()
        .expect("walker thread should not panic")?;

    // Sort deterministically: by file path, then by line number.
    all_findings.sort_unstable_by(|a, b| a.file.cmp(&b.file).then_with(|| a.line.cmp(&b.line)));

    // Detect whether stdout is a TTY and whether the user forced/disabled colour.
    let colour_choice = if std::env::var_os("NO_COLOR").is_some() {
        ColorChoice::Never
    } else {
        ColorChoice::Auto
    };
    let stdout = StandardStream::stdout(colour_choice);
    let use_colour = colour_choice != ColorChoice::Never;
    let mut fmt = TextFormatter::new(stdout, use_colour);
    fmt.write_all(&all_findings)?;

    if all_findings.is_empty() {
        Ok(ExitCode::NotFound)
    } else {
        Ok(ExitCode::Success)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Args;
    use clap::Parser;
    use tempfile::TempDir;

    fn run_on_dir(dir: &TempDir) -> anyhow::Result<ExitCode> {
        let args = Args::parse_from(["todork", dir.path().to_str().unwrap()]);
        let config = Config::from_args(args)?;
        run(config)
    }

    #[test]
    fn empty_dir_returns_not_found() {
        let dir = TempDir::new().unwrap();
        let code = run_on_dir(&dir).unwrap();
        assert_eq!(code, ExitCode::NotFound);
    }

    #[test]
    fn dir_with_annotation_returns_success() {
        use std::io::Write;
        let dir = TempDir::new().unwrap();
        let mut f = std::fs::File::create(dir.path().join("t.rs")).unwrap();
        writeln!(f, "// TODO: test").unwrap();
        let code = run_on_dir(&dir).unwrap();
        assert_eq!(code, ExitCode::Success);
    }

    #[test]
    fn run_on_samples_returns_success() {
        let samples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("samples");
        let args = Args::parse_from(["todork", samples.to_str().unwrap()]);
        let config = Config::from_args(args).unwrap();
        let code = run(config).unwrap();
        assert_eq!(code, ExitCode::Success);
    }
}
