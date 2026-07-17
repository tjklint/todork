pub mod blame;
pub mod cli;
pub mod config;
pub mod error;
pub mod exit_code;
pub mod formatter;
pub mod matcher;
pub mod pager;
pub mod scanner;
pub mod upgrade;
pub mod walker;

use crate::cli::{ColorWhen, Format, SortOrder};
use crate::config::Config;
use crate::exit_code::ExitCode;
use crate::formatter::github::GithubFormatter;
use crate::formatter::json::JsonFormatter;
use crate::formatter::table::TableFormatter;
use crate::formatter::text::TextFormatter;
use crate::formatter::Formatter;
use crate::matcher::{Matcher, DEFAULT_TAGS};
use crate::walker::walk_parallel;
use crossbeam_channel::bounded;
use std::io::{IsTerminal, Write};
use std::sync::Arc;
use termcolor::{Buffer, ColorChoice, StandardStream};

/// Number of findings displayed on a TTY before human-readable output is
/// automatically piped through `less -R`.
const PAGER_THRESHOLD: usize = 50;

/// Run todork with the given configuration and write results to stdout.
///
/// Returns [`ExitCode::Success`] when annotations are found,
/// [`ExitCode::NotFound`] when none are found (or when `--exit-zero` is set
/// and no annotations were found).
pub fn run(config: Config) -> anyhow::Result<ExitCode> {
    let started = std::time::Instant::now();
    let tags = if config.tags.is_empty() {
        DEFAULT_TAGS.to_vec()
    } else {
        config.tags.clone()
    };
    let matcher = Arc::new(Matcher::new(&tags)?);
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

    // ── git blame enrichment (opt-in) ─────────────────────────────────────────
    if config.blame && !all_findings.is_empty() {
        blame::enrich_with_blame(&mut all_findings, &config.paths[0]);
    }

    // ── sort ──────────────────────────────────────────────────────────────────
    match config.sort {
        SortOrder::Path => {
            all_findings
                .sort_unstable_by(|a, b| a.file.cmp(&b.file).then_with(|| a.line.cmp(&b.line)));
        }
        SortOrder::Oldest => {
            all_findings.sort_unstable_by(|a, b| {
                a.blame_date
                    .cmp(&b.blame_date)
                    .then_with(|| a.file.cmp(&b.file))
                    .then_with(|| a.line.cmp(&b.line))
            });
        }
        SortOrder::Newest => {
            all_findings.sort_unstable_by(|a, b| {
                b.blame_date
                    .cmp(&a.blame_date)
                    .then_with(|| a.file.cmp(&b.file))
                    .then_with(|| a.line.cmp(&b.line))
            });
        }
    }

    // ── apply limit & prepare output slice ────────────────────────────────────
    let total_count = all_findings.len();
    let displayed_count = config.limit.map_or(total_count, |n| total_count.min(n));
    let findings = &all_findings[..displayed_count];

    // ── output ────────────────────────────────────────────────────────────────
    match config.format {
        Format::Text => {
            let colour_choice = resolve_color(config.color);
            let use_colour = colour_choice != ColorChoice::Never;
            let should_page = std::io::stdout().is_terminal()
                && std::io::stdin().is_terminal()
                && std::io::stderr().is_terminal()
                && displayed_count > PAGER_THRESHOLD;

            if should_page {
                let mut buf = if use_colour {
                    Buffer::ansi()
                } else {
                    Buffer::no_color()
                };
                {
                    let mut fmt = TextFormatter::new(&mut buf, use_colour);
                    fmt.write_all(findings, started.elapsed(), total_count)?;
                }
                if let Err(e) = pager::page_output(buf.as_slice()) {
                    eprintln!("todork: {e}");
                    let stdout = StandardStream::stdout(colour_choice);
                    let mut fmt = TextFormatter::new(stdout, use_colour);
                    fmt.write_all(findings, started.elapsed(), total_count)?;
                }
            } else {
                let stdout = StandardStream::stdout(colour_choice);
                let mut fmt = TextFormatter::new(stdout, use_colour);
                fmt.write_all(findings, started.elapsed(), total_count)?;
            }
        }
        Format::Table => {
            let colour_choice = resolve_color(config.color);
            let use_colour = colour_choice != ColorChoice::Never;
            let should_page = std::io::stdout().is_terminal()
                && std::io::stdin().is_terminal()
                && std::io::stderr().is_terminal()
                && displayed_count > PAGER_THRESHOLD;

            if should_page {
                let mut buf = if use_colour {
                    Buffer::ansi()
                } else {
                    Buffer::no_color()
                };
                {
                    let mut fmt = TableFormatter::new(&mut buf, use_colour);
                    fmt.write_all(findings, started.elapsed(), total_count)?;
                }
                if let Err(e) = pager::page_output(buf.as_slice()) {
                    eprintln!("todork: {e}");
                    let stdout = StandardStream::stdout(colour_choice);
                    let mut fmt = TableFormatter::new(stdout, use_colour);
                    fmt.write_all(findings, started.elapsed(), total_count)?;
                }
            } else {
                let stdout = StandardStream::stdout(colour_choice);
                let mut fmt = TableFormatter::new(stdout, use_colour);
                fmt.write_all(findings, started.elapsed(), total_count)?;
            }
        }
        Format::Json => {
            let mut stdout = std::io::stdout();
            JsonFormatter.format(findings, &mut stdout)?;
            // Ensure trailing newline after JSON.
            writeln!(stdout)?;
        }
        Format::GithubAnnotations => {
            let mut stdout = std::io::stdout();
            GithubFormatter.format(findings, &mut stdout)?;
        }
    }

    // ── exit code ─────────────────────────────────────────────────────────────
    if config.exit_zero || all_findings.is_empty() {
        if all_findings.is_empty() {
            Ok(ExitCode::NotFound)
        } else {
            Ok(ExitCode::Success)
        }
    } else {
        Ok(ExitCode::Success)
    }
}

/// Translate the CLI [`ColorWhen`] setting into a termcolor [`ColorChoice`],
/// also honouring the `NO_COLOR` and `FORCE_COLOR` environment variables.
fn resolve_color(when: ColorWhen) -> ColorChoice {
    match when {
        ColorWhen::Always => ColorChoice::Always,
        ColorWhen::Never => ColorChoice::Never,
        ColorWhen::Auto => {
            if std::env::var_os("NO_COLOR").is_some() {
                ColorChoice::Never
            } else {
                ColorChoice::Auto
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Args;
    use clap::Parser;
    use tempfile::TempDir;

    fn run_on(argv: &[&str]) -> anyhow::Result<ExitCode> {
        let args = Args::parse_from(argv);
        let config = Config::from_args(args)?;
        run(config)
    }

    fn run_on_dir(dir: &TempDir) -> anyhow::Result<ExitCode> {
        run_on(&["todork", dir.path().to_str().unwrap()])
    }

    fn write_file(dir: &TempDir, name: &str, content: &str) {
        use std::io::Write;
        let mut f = std::fs::File::create(dir.path().join(name)).unwrap();
        writeln!(f, "{content}").unwrap();
    }

    // ── exit codes ────────────────────────────────────────────────────────────

    #[test]
    fn empty_dir_returns_not_found() {
        let dir = TempDir::new().unwrap();
        assert_eq!(run_on_dir(&dir).unwrap(), ExitCode::NotFound);
    }

    #[test]
    fn dir_with_annotation_returns_success() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, "t.rs", "// TODO: test");
        assert_eq!(run_on_dir(&dir).unwrap(), ExitCode::Success);
    }

    #[test]
    fn exit_zero_with_no_findings_returns_not_found() {
        let dir = TempDir::new().unwrap();
        let code = run_on(&["todork", "--exit-zero", dir.path().to_str().unwrap()]).unwrap();
        assert_eq!(code, ExitCode::NotFound);
    }

    #[test]
    fn exit_zero_with_findings_still_returns_success() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, "t.rs", "// TODO: test");
        let code = run_on(&["todork", "--exit-zero", dir.path().to_str().unwrap()]).unwrap();
        assert_eq!(code, ExitCode::Success);
    }

    // ── format dispatching ────────────────────────────────────────────────────

    #[test]
    fn run_on_samples_returns_success() {
        let samples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("samples");
        assert_eq!(
            run_on(&["todork", samples.to_str().unwrap()]).unwrap(),
            ExitCode::Success
        );
    }

    #[test]
    fn run_json_format_on_samples_returns_success() {
        let samples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("samples");
        assert_eq!(
            run_on(&["todork", "--format", "json", samples.to_str().unwrap()]).unwrap(),
            ExitCode::Success
        );
    }

    #[test]
    fn run_table_format_on_samples_returns_success() {
        let samples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("samples");
        assert_eq!(
            run_on(&["todork", "--format", "table", samples.to_str().unwrap()]).unwrap(),
            ExitCode::Success
        );
    }

    #[test]
    fn run_github_format_on_samples_returns_success() {
        let samples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("samples");
        assert_eq!(
            run_on(&[
                "todork",
                "--format",
                "github-annotations",
                samples.to_str().unwrap()
            ])
            .unwrap(),
            ExitCode::Success
        );
    }

    // ── colour resolution ─────────────────────────────────────────────────────

    #[test]
    fn resolve_color_always() {
        assert_eq!(resolve_color(ColorWhen::Always), ColorChoice::Always);
    }

    #[test]
    fn resolve_color_never() {
        assert_eq!(resolve_color(ColorWhen::Never), ColorChoice::Never);
    }

    #[test]
    fn resolve_color_auto_with_no_color_env() {
        std::env::set_var("NO_COLOR", "1");
        let result = resolve_color(ColorWhen::Auto);
        std::env::remove_var("NO_COLOR");
        assert_eq!(result, ColorChoice::Never);
    }
}
