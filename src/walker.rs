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
        .hidden(!config.search_hidden)
        .git_ignore(config.use_gitignore)
        .git_global(config.use_gitignore)
        .git_exclude(config.use_gitignore)
        // Respect .gitignore even when the path is not inside a git repository.
        .require_git(false)
        .threads(config.threads);

    if let Some(depth) = config.max_depth {
        builder.max_depth(Some(depth));
    }

    let max_file_size = config.max_file_size;
    let include = config.include.clone();
    let exclude = config.exclude.clone();

    builder.build_parallel().run(|| {
        let matcher = matcher.clone();
        let sender = sender.clone();
        let include = include.clone();
        let exclude = exclude.clone();
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

            // ── glob filtering ────────────────────────────────────────────────
            // Match against both the full path AND the bare filename so that a
            // pattern like `*.rs` or `skip.rs` works without needing `**/`.
            let filename = path.file_name().unwrap_or_default();
            if let Some(ref ex) = exclude {
                if ex.is_match(path) || ex.is_match(filename) {
                    return WalkState::Continue;
                }
            }
            if let Some(ref inc) = include {
                if !inc.is_match(path) && !inc.is_match(filename) {
                    return WalkState::Continue;
                }
            }

            // ── scan ──────────────────────────────────────────────────────────
            match scan_file(path, &matcher, max_file_size) {
                Ok(mut findings) if !findings.is_empty() => {
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

    fn config_for_dir_with(dir: &TempDir, extra: &[&str]) -> Config {
        let path = dir.path().to_str().unwrap();
        let mut argv = vec!["todork", path];
        argv.extend_from_slice(extra);
        let args = Args::parse_from(&argv);
        Config::from_args(args).unwrap()
    }

    fn collect_findings(config: &Config) -> Vec<Finding> {
        let matcher = Arc::new(Matcher::new(&config.tags).unwrap());
        let (tx, rx) = bounded(256);
        walk_parallel(config, matcher, tx).unwrap();
        rx.into_iter().flatten().collect()
    }

    fn write_file(dir: &TempDir, name: &str, content: &str) {
        let mut f = std::fs::File::create(dir.path().join(name)).unwrap();
        writeln!(f, "{content}").unwrap();
    }

    // ── basic walk ────────────────────────────────────────────────────────────

    #[test]
    fn finds_todo_in_single_file() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, "a.rs", "// TODO: test finding");
        let findings = collect_findings(&config_for_dir(&dir));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].tag, "TODO");
    }

    #[test]
    fn walks_subdirectories() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        write_file(&dir, "sub/b.py", "# FIXME: deep file");
        let findings = collect_findings(&config_for_dir(&dir));
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn empty_directory_returns_no_findings() {
        let dir = TempDir::new().unwrap();
        assert!(collect_findings(&config_for_dir(&dir)).is_empty());
    }

    #[test]
    fn directory_with_no_annotations_returns_empty() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, "clean.rs", "fn main() {}");
        assert!(collect_findings(&config_for_dir(&dir)).is_empty());
    }

    // ── gitignore ─────────────────────────────────────────────────────────────

    #[test]
    fn respects_gitignore() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, ".gitignore", "ignored.rs");
        write_file(&dir, "ignored.rs", "// TODO: should be ignored");
        assert!(
            collect_findings(&config_for_dir(&dir)).is_empty(),
            "gitignored file should not be scanned"
        );
    }

    #[test]
    fn no_gitignore_flag_scans_ignored_files() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, ".gitignore", "ignored.rs");
        write_file(&dir, "ignored.rs", "// TODO: should appear now");
        let findings = collect_findings(&config_for_dir_with(&dir, &["--no-gitignore"]));
        assert_eq!(findings.len(), 1);
    }

    // ── include / exclude globs ───────────────────────────────────────────────

    #[test]
    fn include_glob_filters_to_matching_extension() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, "a.rs", "// TODO: rust");
        write_file(&dir, "b.py", "# TODO: python");
        let findings = collect_findings(&config_for_dir_with(&dir, &["--include", "*.rs"]));
        assert_eq!(findings.len(), 1);
        assert!(findings[0].file.to_str().unwrap().ends_with(".rs"));
    }

    #[test]
    fn exclude_glob_skips_matching_files() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, "keep.rs", "// TODO: keep");
        write_file(&dir, "skip.rs", "// TODO: skip");
        let findings = collect_findings(&config_for_dir_with(&dir, &["--exclude", "skip.rs"]));
        assert_eq!(findings.len(), 1);
        assert!(findings[0].file.to_str().unwrap().contains("keep"));
    }

    // ── hidden files ──────────────────────────────────────────────────────────

    #[test]
    fn hidden_files_skipped_by_default() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, ".hidden.rs", "// TODO: hidden");
        assert!(collect_findings(&config_for_dir(&dir)).is_empty());
    }

    #[test]
    fn hidden_flag_includes_hidden_files() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, ".hidden.rs", "// TODO: hidden");
        let findings = collect_findings(&config_for_dir_with(&dir, &["--hidden"]));
        assert_eq!(findings.len(), 1);
    }

    // ── max depth ─────────────────────────────────────────────────────────────

    #[test]
    fn max_depth_limits_traversal() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("a/b")).unwrap();
        write_file(&dir, "top.rs", "// TODO: top level");
        write_file(&dir, "a/mid.rs", "// TODO: mid level");
        write_file(&dir, "a/b/deep.rs", "// TODO: deep level");

        // depth=1 means only the root and its immediate children.
        let findings = collect_findings(&config_for_dir_with(&dir, &["--max-depth", "1"]));
        for f in &findings {
            let depth = f
                .file
                .strip_prefix(dir.path())
                .unwrap()
                .components()
                .count();
            assert!(depth <= 1, "depth {depth} > 1 for {:?}", f.file);
        }
    }

    // ── tag filtering ─────────────────────────────────────────────────────────

    #[test]
    fn tags_filter_limits_matches() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, "f.rs", "// TODO: one\n// FIXME: two");
        let findings = collect_findings(&config_for_dir_with(&dir, &["--tags", "fixme"]));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].tag, "FIXME");
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
