//! Parallel directory walker built on top of the [`ignore`] crate.
//!
//! Uses `WalkParallel` (the same engine as ripgrep) to walk directory trees
//! in parallel while automatically respecting `.gitignore` rules.

use crate::config::Config;
use crate::matcher::{Finding, Matcher};
use crate::scanner::scan_file;
use crossbeam_channel::Sender;
use ignore::WalkBuilder;
use std::sync::Arc;

/// Walk all paths in `config` in parallel and send batches of [`Finding`]s
/// through `sender`.
///
/// The caller is responsible for draining the channel; this function blocks
/// until the walk is complete.
pub fn walk_parallel(
    config: &Config,
    matcher: Arc<Matcher>,
    sender: Sender<Vec<Finding>>,
) -> anyhow::Result<()> {
    if config.paths.is_empty() {
        return Ok(());
    }

    let mut builder = WalkBuilder::new(&config.paths[0]);
    for path in &config.paths[1..] {
        builder.add(path);
    }

    builder
        .hidden(true) // skip hidden files/dirs by default
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        // Respect .gitignore even when the path is not inside a git repository.
        .require_git(false)
        .threads(num_cpus::get());

    let max_file_size = config.max_file_size;

    builder.build_parallel().run(|| {
        let matcher = matcher.clone();
        let sender = sender.clone();
        Box::new(move |entry_result| {
            use ignore::WalkState;

            let entry = match entry_result {
                Ok(e) => e,
                Err(_) => return WalkState::Continue,
            };

            // Only process regular files.
            if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                return WalkState::Continue;
            }

            let path = entry.path();
            match scan_file(path, &matcher, max_file_size) {
                Ok(mut findings) if !findings.is_empty() => {
                    // Set canonical path so output is consistent.
                    for f in &mut findings {
                        f.file = path.to_path_buf();
                    }
                    let _ = sender.send(findings);
                }
                _ => {}
            }

            WalkState::Continue
        })
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Args;
    use crate::config::Config;
    use crate::matcher::{Matcher, DEFAULT_TAGS};
    use clap::Parser;
    use crossbeam_channel::bounded;
    use std::io::Write;
    use tempfile::TempDir;

    fn config_for_dir(dir: &TempDir) -> Config {
        let args = Args::parse_from(["todork", dir.path().to_str().unwrap()]);
        Config::from_args(args).unwrap()
    }

    fn collect_findings(config: &Config) -> Vec<Finding> {
        let matcher = Arc::new(Matcher::new(DEFAULT_TAGS).unwrap());
        let (tx, rx) = bounded(256);
        walk_parallel(config, matcher, tx).unwrap();
        rx.into_iter().flatten().collect()
    }

    // ── basic walk ────────────────────────────────────────────────────────────

    #[test]
    fn finds_todo_in_single_file() {
        let dir = TempDir::new().unwrap();
        let mut f = std::fs::File::create(dir.path().join("a.rs")).unwrap();
        writeln!(f, "// TODO: test finding").unwrap();

        let findings = collect_findings(&config_for_dir(&dir));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].tag, "TODO");
    }

    #[test]
    fn walks_subdirectories() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        let mut f = std::fs::File::create(dir.path().join("sub/b.py")).unwrap();
        writeln!(f, "# FIXME: deep file").unwrap();

        let findings = collect_findings(&config_for_dir(&dir));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].tag, "FIXME");
    }

    #[test]
    fn empty_directory_returns_no_findings() {
        let dir = TempDir::new().unwrap();
        let findings = collect_findings(&config_for_dir(&dir));
        assert!(findings.is_empty());
    }

    #[test]
    fn directory_with_no_annotations_returns_empty() {
        let dir = TempDir::new().unwrap();
        let mut f = std::fs::File::create(dir.path().join("clean.rs")).unwrap();
        writeln!(f, "fn main() {{ println!(\"hi\"); }}").unwrap();

        let findings = collect_findings(&config_for_dir(&dir));
        assert!(findings.is_empty());
    }

    // ── gitignore ─────────────────────────────────────────────────────────────

    #[test]
    fn respects_gitignore() {
        let dir = TempDir::new().unwrap();

        // Write a .gitignore that excludes ignored.rs
        let mut gi = std::fs::File::create(dir.path().join(".gitignore")).unwrap();
        writeln!(gi, "ignored.rs").unwrap();

        // Write the ignored file with a TODO
        let mut ig = std::fs::File::create(dir.path().join("ignored.rs")).unwrap();
        writeln!(ig, "// TODO: should be ignored").unwrap();

        let findings = collect_findings(&config_for_dir(&dir));
        assert!(findings.is_empty(), "gitignored file should not be scanned");
    }

    // ── empty config ──────────────────────────────────────────────────────────

    #[test]
    fn empty_paths_returns_ok() {
        let mut config = config_for_dir(&TempDir::new().unwrap());
        config.paths.clear();
        let matcher = Arc::new(Matcher::new(DEFAULT_TAGS).unwrap());
        let (tx, _rx) = bounded(256);
        assert!(walk_parallel(&config, matcher, tx).is_ok());
    }
}
