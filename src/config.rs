use crate::cli::{Args, ColorWhen, Format, SortOrder};
use crate::error::TodorkError;
use crate::matcher::{Tag, DEFAULT_TAGS};
use crate::scanner::DEFAULT_MAX_FILE_SIZE;
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::PathBuf;

/// Resolved, validated configuration derived from CLI arguments.
#[derive(Debug, Clone)]
pub struct Config {
    /// Paths to scan.
    pub paths: Vec<PathBuf>,
    /// Output format.
    pub format: Format,
    /// Tags to search for (filtered from DEFAULT_TAGS or custom).
    pub tags: Vec<Tag>,
    /// Only scan files matching these globs (None = no include filter).
    pub include: Option<GlobSet>,
    /// Skip files matching these globs (None = no exclude filter).
    pub exclude: Option<GlobSet>,
    /// When false, respect .gitignore / .ignore files.
    pub use_gitignore: bool,
    /// When true, scan hidden files/dirs.
    pub search_hidden: bool,
    /// Maximum directory depth (None = unlimited).
    pub max_depth: Option<usize>,
    /// Maximum file size in bytes before skipping.
    pub max_file_size: u64,
    /// Number of parallel threads.
    pub threads: usize,
    /// Colour output setting.
    pub color: ColorWhen,
    /// When true, always exit 0 even when annotations are found.
    pub exit_zero: bool,
    /// When true, enrich findings with git blame data.
    pub blame: bool,
    /// Sort order applied to findings before output.
    pub sort: SortOrder,
    /// Maximum number of findings to display.
    pub limit: Option<usize>,
}

impl Config {
    /// Build a validated `Config` from parsed CLI arguments.
    pub fn from_args(args: Args) -> anyhow::Result<Self> {
        // ── tag resolution ────────────────────────────────────────────────────
        let tags = if args.tags.is_empty() {
            DEFAULT_TAGS.to_vec()
        } else {
            resolve_tags(&args.tags)?
        };

        // ── glob compilation ──────────────────────────────────────────────────
        let include = build_glob_set(&args.include)?;
        let exclude = build_glob_set(&args.exclude)?;

        // ── thread count ──────────────────────────────────────────────────────
        let threads = match args.threads {
            Some(0) => {
                return Err(TodorkError::InvalidTag("--threads must be at least 1".into()).into())
            }
            Some(n) => n,
            None => num_cpus::get(),
        };

        let blame = args.blame || matches!(args.sort, SortOrder::Oldest | SortOrder::Newest);

        Ok(Self {
            paths: args.paths,
            format: args.format,
            tags,
            include,
            exclude,
            use_gitignore: !args.no_gitignore,
            search_hidden: args.hidden,
            max_depth: args.max_depth,
            max_file_size: args.max_filesize.unwrap_or(DEFAULT_MAX_FILE_SIZE),
            threads,
            color: args.color,
            exit_zero: args.exit_zero,
            blame,
            sort: args.sort,
            limit: args.limit,
        })
    }
}

/// Resolve a list of tag name strings (case-insensitive) against DEFAULT_TAGS.
///
/// Unknown tag names are silently allowed as custom tags with Warning severity —
/// this lets users scan for project-specific annotations like SAFETY or PERF.
fn resolve_tags(names: &[String]) -> anyhow::Result<Vec<Tag>> {
    use crate::matcher::Severity;

    let mut resolved = Vec::with_capacity(names.len());
    for name in names {
        let upper = name.to_uppercase();
        if upper.is_empty() || !upper.chars().all(|c| c.is_ascii_alphabetic() || c == '_') {
            return Err(TodorkError::InvalidTag(name.clone()).into());
        }
        // Look up in defaults first so severity is preserved.
        let tag = DEFAULT_TAGS
            .iter()
            .find(|t| t.name == upper)
            .cloned()
            .unwrap_or_else(|| {
                // Custom tag — leak the string so it gets a `'static` lifetime.
                Tag {
                    name: Box::leak(upper.into_boxed_str()),
                    severity: Severity::Warning,
                }
            });
        resolved.push(tag);
    }
    Ok(resolved)
}

/// Compile a list of glob patterns into a [`GlobSet`].
/// Returns `None` if the list is empty.
fn build_glob_set(patterns: &[String]) -> anyhow::Result<Option<GlobSet>> {
    if patterns.is_empty() {
        return Ok(None);
    }
    let mut builder = GlobSetBuilder::new();
    for pat in patterns {
        let glob = Glob::new(pat).map_err(|e| TodorkError::InvalidGlob {
            pattern: pat.clone(),
            reason: e.to_string(),
        })?;
        builder.add(glob);
    }
    Ok(Some(builder.build().map_err(|e| {
        TodorkError::InvalidGlob {
            pattern: patterns.join(", "),
            reason: e.to_string(),
        }
    })?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Args;
    use clap::Parser;

    fn parse(args: &[&str]) -> Config {
        let args = Args::parse_from(args);
        Config::from_args(args).unwrap()
    }

    fn parse_err(args: &[&str]) -> anyhow::Error {
        let args = Args::parse_from(args);
        Config::from_args(args).unwrap_err()
    }

    // ── basics ────────────────────────────────────────────────────────────────

    #[test]
    fn config_from_default_args() {
        let c = parse(&["todork"]);
        assert_eq!(c.paths, vec![PathBuf::from(".")]);
    }

    #[test]
    fn config_preserves_multiple_paths() {
        let c = parse(&["todork", "src/", "tests/"]);
        assert_eq!(c.paths.len(), 2);
    }

    #[test]
    fn default_max_file_size_is_10mb() {
        let c = parse(&["todork"]);
        assert_eq!(c.max_file_size, 10 * 1024 * 1024);
    }

    #[test]
    fn custom_max_filesize() {
        let c = parse(&["todork", "--max-filesize", "1000"]);
        assert_eq!(c.max_file_size, 1000);
    }

    // ── format ────────────────────────────────────────────────────────────────

    #[test]
    fn default_format_is_text() {
        assert_eq!(parse(&["todork"]).format, Format::Text);
    }

    #[test]
    fn format_json() {
        assert_eq!(parse(&["todork", "--format", "json"]).format, Format::Json);
    }

    #[test]
    fn format_github_annotations() {
        assert_eq!(
            parse(&["todork", "--format", "github-annotations"]).format,
            Format::GithubAnnotations
        );
    }

    // ── tags ──────────────────────────────────────────────────────────────────

    #[test]
    fn default_tags_all_present() {
        let c = parse(&["todork"]);
        assert_eq!(c.tags.len(), DEFAULT_TAGS.len());
    }

    #[test]
    fn single_tag_filter() {
        let c = parse(&["todork", "--tags", "todo"]);
        assert_eq!(c.tags.len(), 1);
        assert_eq!(c.tags[0].name, "TODO");
    }

    #[test]
    fn multiple_tag_filter() {
        let c = parse(&["todork", "--tags", "todo,fixme"]);
        assert_eq!(c.tags.len(), 2);
    }

    #[test]
    fn tags_are_case_insensitive() {
        let c = parse(&["todork", "--tags", "Todo"]);
        assert_eq!(c.tags[0].name, "TODO");
    }

    #[test]
    fn custom_tag_accepted() {
        let c = parse(&["todork", "--tags", "SAFETY"]);
        assert_eq!(c.tags[0].name, "SAFETY");
    }

    #[test]
    fn empty_tag_name_is_error() {
        // Empty tag names are rejected by resolve_tags directly.
        let err = resolve_tags(&["".to_string()]);
        assert!(err.is_err());
    }

    // ── glob filters ──────────────────────────────────────────────────────────

    #[test]
    fn no_include_returns_none() {
        assert!(parse(&["todork"]).include.is_none());
    }

    #[test]
    fn include_glob_compiled() {
        let c = parse(&["todork", "--include", "*.rs"]);
        assert!(c.include.is_some());
    }

    #[test]
    fn exclude_glob_compiled() {
        let c = parse(&["todork", "--exclude", "target/*"]);
        assert!(c.exclude.is_some());
    }

    #[test]
    fn include_matches_expected_path() {
        let c = parse(&["todork", "--include", "*.rs"]);
        let gs = c.include.unwrap();
        assert!(gs.is_match("foo.rs"));
        assert!(!gs.is_match("foo.py"));
    }

    #[test]
    fn exclude_matches_expected_path() {
        let c = parse(&["todork", "--exclude", "*.min.js"]);
        let gs = c.exclude.unwrap();
        assert!(gs.is_match("app.min.js"));
        assert!(!gs.is_match("app.js"));
    }

    // ── boolean flags ─────────────────────────────────────────────────────────

    #[test]
    fn default_uses_gitignore() {
        assert!(parse(&["todork"]).use_gitignore);
    }

    #[test]
    fn no_gitignore_disables_it() {
        assert!(!parse(&["todork", "--no-gitignore"]).use_gitignore);
    }

    #[test]
    fn default_skips_hidden() {
        assert!(!parse(&["todork"]).search_hidden);
    }

    #[test]
    fn hidden_flag_enables_hidden() {
        assert!(parse(&["todork", "--hidden"]).search_hidden);
    }

    #[test]
    fn exit_zero_default_false() {
        assert!(!parse(&["todork"]).exit_zero);
    }

    #[test]
    fn exit_zero_flag_sets_true() {
        assert!(parse(&["todork", "--exit-zero"]).exit_zero);
    }

    // ── threads ───────────────────────────────────────────────────────────────

    #[test]
    fn threads_zero_is_error() {
        let err = parse_err(&["todork", "--threads", "0"]);
        assert!(err.to_string().contains("threads"));
    }

    #[test]
    fn threads_custom_value() {
        assert_eq!(parse(&["todork", "--threads", "4"]).threads, 4);
    }

    // ── colour ────────────────────────────────────────────────────────────────

    #[test]
    fn default_color_is_auto() {
        assert_eq!(parse(&["todork"]).color, ColorWhen::Auto);
    }

    #[test]
    fn color_never() {
        assert_eq!(
            parse(&["todork", "--color", "never"]).color,
            ColorWhen::Never
        );
    }

    #[test]
    fn color_always() {
        assert_eq!(
            parse(&["todork", "--color", "always"]).color,
            ColorWhen::Always
        );
    }

    // ── blame ─────────────────────────────────────────────────────────────────

    #[test]
    fn blame_default_false() {
        assert!(!parse(&["todork"]).blame);
    }

    #[test]
    fn blame_flag_sets_true() {
        assert!(parse(&["todork", "--blame"]).blame);
    }

    // ── sort ──────────────────────────────────────────────────────────────────

    #[test]
    fn sort_default_is_path() {
        assert_eq!(parse(&["todork"]).sort, SortOrder::Path);
    }

    #[test]
    fn sort_oldest_implies_blame() {
        let c = parse(&["todork", "--sort", "oldest"]);
        assert_eq!(c.sort, SortOrder::Oldest);
        assert!(c.blame, "--sort oldest should imply --blame");
    }

    #[test]
    fn sort_newest_implies_blame() {
        let c = parse(&["todork", "--sort", "newest"]);
        assert_eq!(c.sort, SortOrder::Newest);
        assert!(c.blame, "--sort newest should imply --blame");
    }

    #[test]
    fn sort_path_does_not_imply_blame() {
        let c = parse(&["todork", "--sort", "path"]);
        assert!(!c.blame);
    }

    #[test]
    fn limit_default_none() {
        let c = parse(&["todork"]);
        assert_eq!(c.limit, None);
    }

    #[test]
    fn limit_flag_parsed() {
        let c = parse(&["todork", "--limit", "50"]);
        assert_eq!(c.limit, Some(50));
    }
}
