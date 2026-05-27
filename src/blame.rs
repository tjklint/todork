//! Git blame enrichment for [`Finding`](crate::matcher::Finding)s.
//!
//! Uses [`git2`] (libgit2) for pure-Rust blame lookups — no subprocess
//! spawning, one repository handle shared across all files.

use crate::matcher::Finding;
use git2::Repository;
use std::collections::HashMap;
use std::path::Path;

/// Enrich `findings` with git blame information in-place.
///
/// Discovers the git repository that contains `root` using the standard
/// parent-directory walk.  For each finding the commit that last touched
/// that exact line is resolved and stored in the `blame_*` fields.
///
/// **Silently no-ops** when:
/// - `root` is not inside a git repository
/// - the repo is bare (no working directory)
/// - a file is untracked / not yet committed
/// - any other libgit2 error occurs
///
/// Findings that cannot be blamed are left with all `blame_*` fields as
/// `None`; the rest of the finding is untouched.
pub fn enrich_with_blame(findings: &mut [Finding], root: &Path) {
    let repo = match Repository::discover(root) {
        Ok(r) => r,
        Err(_) => return, // not a git repo — silently skip
    };
    let workdir = match repo.workdir() {
        Some(d) => d.to_path_buf(),
        None => return, // bare repo
    };

    // Canonicalize the workdir once so strip_prefix works even through symlinks.
    let workdir_canon = workdir.canonicalize().unwrap_or(workdir);

    // Group finding indices by file so we issue one blame_file() per file.
    let mut by_file: HashMap<std::path::PathBuf, Vec<usize>> = HashMap::new();
    for (i, f) in findings.iter().enumerate() {
        by_file.entry(f.file.clone()).or_default().push(i);
    }

    for (file_path, indices) in &by_file {
        // Resolve absolute path → path relative to the repo root.
        let canon = match file_path.canonicalize() {
            Ok(p) => p,
            Err(_) => file_path.clone(),
        };
        let rel = match canon.strip_prefix(&workdir_canon) {
            Ok(r) => r.to_path_buf(),
            Err(_) => continue, // file is outside this repo
        };

        let blame = match repo.blame_file(&rel, None) {
            Ok(b) => b,
            Err(_) => continue, // untracked, new file, etc.
        };

        for &idx in indices {
            let line = findings[idx].line;
            let hunk = match blame.get_line(line) {
                Some(h) => h,
                None => continue,
            };

            let sig = hunk.final_signature();
            let name = sig.name().unwrap_or("unknown");
            let email = sig.email().unwrap_or("");
            let author = if email.is_empty() {
                name.to_string()
            } else {
                format!("{name} <{email}>")
            };
            let date = sig.when().seconds();
            let full_sha = hunk.final_commit_id().to_string();
            let commit = full_sha[..full_sha.len().min(7)].to_string();

            findings[idx].blame_author = Some(author);
            findings[idx].blame_date = Some(date);
            findings[idx].blame_commit = Some(commit);
        }
    }
}

/// Format a Unix timestamp as a human-readable age string.
///
/// | Range | Example |
/// |-------|---------|
/// | same day | `"today"` |
/// | 1 day | `"1 day ago"` |
/// | 2–13 days | `"5 days ago"` |
/// | 2–8 weeks | `"3 weeks ago"` |
/// | 2–23 months | `"8 months ago"` |
/// | 2+ years | `"2 years ago"` |
pub fn format_age(timestamp: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let days = ((now - timestamp).max(0)) / 86_400;

    if days == 0 {
        "today".to_string()
    } else if days == 1 {
        "1 day ago".to_string()
    } else if days < 7 {
        format!("{days} days ago")
    } else if days < 30 {
        // 7–29 days → weeks  (7→1 wk, 14→2 wks, 21→3 wks, 28→4 wks)
        let weeks = days / 7;
        format!("{weeks} week{} ago", if weeks == 1 { "" } else { "s" })
    } else if days < 365 {
        // 30–364 days → months  (30→1 mo, 60→2 mo, …, 330→11 mo)
        let months = days / 30;
        format!("{months} month{} ago", if months == 1 { "" } else { "s" })
    } else {
        // 365+ days → years  (365→1 yr, 730→2 yrs, …)
        let years = days / 365;
        format!("{years} year{} ago", if years == 1 { "" } else { "s" })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matcher::{Finding, Severity};
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    // ── helpers ──────────────────────────────────────────────────────────────

    fn plain_finding(file: PathBuf, line: usize) -> Finding {
        Finding {
            file,
            line,
            column: 1,
            tag: "TODO".to_string(),
            severity: Severity::Warning,
            author: None,
            message: "test".to_string(),
            blame_author: None,
            blame_date: None,
            blame_commit: None,
        }
    }

    /// Add `filename` with `content` to `repo` and commit it.
    ///
    /// Handles both the initial root commit and subsequent commits
    /// by automatically finding the existing HEAD if one is present.
    fn commit_file(repo: &git2::Repository, filename: &str, content: &str) {
        std::fs::write(repo.workdir().unwrap().join(filename), content).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new(filename)).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = git2::Signature::now("Test Author", "author@test.com").unwrap();
        // Use existing HEAD as parent if the repo already has at least one commit.
        let parent_commit = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        let parents: Vec<&git2::Commit<'_>> = parent_commit.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, "commit", &tree, &parents)
            .unwrap();
    }

    /// Initialize a git repo in `dir`, write `filename` with `content`, and
    /// commit it so blame data is available.
    fn init_repo_with_commit(dir: &TempDir, filename: &str, content: &str) {
        let repo = git2::Repository::init(dir.path()).unwrap();
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.name", "Test Author").unwrap();
        cfg.set_str("user.email", "author@test.com").unwrap();
        drop(cfg);
        commit_file(&repo, filename, content);
    }

    // ── format_age ───────────────────────────────────────────────────────────

    fn now_secs() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    #[test]
    fn format_age_today() {
        assert_eq!(format_age(now_secs()), "today");
    }

    #[test]
    fn format_age_one_day() {
        assert_eq!(format_age(now_secs() - 86_400), "1 day ago");
    }

    #[test]
    fn format_age_several_days() {
        // 5 days < 7 → "N days ago"
        assert_eq!(format_age(now_secs() - 5 * 86_400), "5 days ago");
    }

    #[test]
    fn format_age_weeks() {
        // 21 days / 7 = 3 weeks
        assert_eq!(format_age(now_secs() - 21 * 86_400), "3 weeks ago");
    }

    #[test]
    fn format_age_one_week_singular() {
        // 9 days: 7 <= 9 < 30 → weeks; 9/7 = 1 week
        assert_eq!(format_age(now_secs() - 9 * 86_400), "1 week ago");
    }

    #[test]
    fn format_age_months() {
        // 90 days: 30 <= 90 < 365 → months; 90/30 = 3 months
        assert_eq!(format_age(now_secs() - 90 * 86_400), "3 months ago");
    }

    #[test]
    fn format_age_one_month_singular() {
        // 30 days: 30 <= 30 < 365 → months; 30/30 = 1 month
        assert_eq!(format_age(now_secs() - 30 * 86_400), "1 month ago");
    }

    #[test]
    fn format_age_years() {
        // 2 * 365 days / 365 = 2 years
        assert_eq!(format_age(now_secs() - 2 * 365 * 86_400), "2 years ago");
    }

    #[test]
    fn format_age_one_year_singular() {
        // 365 days: >= 365 → years; 365/365 = 1 year
        assert_eq!(format_age(now_secs() - 365 * 86_400), "1 year ago");
    }

    // ── enrich_with_blame ────────────────────────────────────────────────────

    #[test]
    fn non_git_dir_leaves_blame_fields_none() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("a.rs");
        std::fs::write(&file, "// TODO: test\n").unwrap();
        let mut findings = vec![plain_finding(file, 1)];
        enrich_with_blame(&mut findings, dir.path());
        assert!(findings[0].blame_author.is_none());
        assert!(findings[0].blame_date.is_none());
        assert!(findings[0].blame_commit.is_none());
    }

    #[test]
    fn uncommitted_file_leaves_blame_none() {
        let dir = TempDir::new().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        // File exists but is never committed — blame_file will error.
        let file = dir.path().join("a.rs");
        std::fs::write(&file, "// TODO: test\n").unwrap();
        let mut findings = vec![plain_finding(file, 1)];
        enrich_with_blame(&mut findings, dir.path());
        assert!(findings[0].blame_author.is_none());
    }

    #[test]
    fn committed_file_populates_blame_author() {
        let dir = TempDir::new().unwrap();
        init_repo_with_commit(&dir, "a.rs", "// TODO: test\n");
        let file = dir.path().join("a.rs");
        let mut findings = vec![plain_finding(file, 1)];
        enrich_with_blame(&mut findings, dir.path());
        let author = findings[0].blame_author.as_deref().unwrap();
        assert!(author.contains("Test Author"));
        assert!(author.contains("author@test.com"));
    }

    #[test]
    fn committed_file_populates_blame_date() {
        let dir = TempDir::new().unwrap();
        init_repo_with_commit(&dir, "a.rs", "// TODO: test\n");
        let file = dir.path().join("a.rs");
        let mut findings = vec![plain_finding(file, 1)];
        enrich_with_blame(&mut findings, dir.path());
        assert!(findings[0].blame_date.unwrap() > 0);
    }

    #[test]
    fn committed_file_populates_short_commit_hash() {
        let dir = TempDir::new().unwrap();
        init_repo_with_commit(&dir, "a.rs", "// TODO: test\n");
        let file = dir.path().join("a.rs");
        let mut findings = vec![plain_finding(file, 1)];
        enrich_with_blame(&mut findings, dir.path());
        let commit = findings[0].blame_commit.as_deref().unwrap();
        assert_eq!(commit.len(), 7, "short hash should be 7 chars");
        assert!(
            commit.chars().all(|c| c.is_ascii_hexdigit()),
            "hash must be hex"
        );
    }

    #[test]
    fn multiple_findings_in_same_file_all_enriched() {
        let dir = TempDir::new().unwrap();
        init_repo_with_commit(&dir, "a.rs", "// TODO: one\n// FIXME: two\n");
        let file = dir.path().join("a.rs");
        let mut findings = vec![plain_finding(file.clone(), 1), plain_finding(file, 2)];
        enrich_with_blame(&mut findings, dir.path());
        assert!(findings[0].blame_author.is_some());
        assert!(findings[1].blame_author.is_some());
    }

    #[test]
    fn findings_in_different_files_both_enriched() {
        let dir = TempDir::new().unwrap();
        // Both files must be committed to the SAME repo (two separate commits).
        let repo = git2::Repository::init(dir.path()).unwrap();
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.name", "Test Author").unwrap();
        cfg.set_str("user.email", "author@test.com").unwrap();
        drop(cfg);
        commit_file(&repo, "a.rs", "// TODO: in a\n");
        commit_file(&repo, "b.rs", "// FIXME: in b\n");

        let file_a = dir.path().join("a.rs");
        let file_b = dir.path().join("b.rs");
        let mut findings = vec![plain_finding(file_a, 1), plain_finding(file_b, 1)];
        enrich_with_blame(&mut findings, dir.path());
        assert!(findings[0].blame_author.is_some());
        assert!(findings[1].blame_author.is_some());
    }
}
