//! Git blame enrichment for [`Finding`](crate::matcher::Finding)s.
//!
//! Shells out to `git blame --porcelain` for fast blame lookups — no libgit2
//! dependency at runtime, and benefits from the native git binary's
//! incremental blame cache.

use crate::matcher::Finding;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

/// Enrich `findings` with git blame information in-place.
///
/// For each finding the commit that last touched that exact line is resolved
/// and stored in the `blame_*` fields.
///
/// **Silently no-ops** when:
/// - `git` is not on PATH
/// - the scanned path is not inside a git repository
/// - a file is untracked / not yet committed
/// - any other error occurs
///
/// Findings that cannot be blamed are left with all `blame_*` fields as
/// `None`; the rest of the finding is untouched.
pub fn enrich_with_blame(findings: &mut [Finding], _root: &Path) {
    // Group finding indices by file so we issue one `git blame` per file.
    let mut by_file: HashMap<std::path::PathBuf, Vec<usize>> = HashMap::new();
    for (i, f) in findings.iter().enumerate() {
        by_file.entry(f.file.clone()).or_default().push(i);
    }

    for (file_path, indices) in &by_file {
        let output = match Command::new("git")
            .arg("blame")
            .arg("--porcelain")
            .arg(file_path)
            .current_dir(file_path.parent().unwrap_or(Path::new(".")))
            .output()
        {
            Ok(o) if o.status.success() => o,
            _ => continue,
        };

        let blame_map = parse_porcelain(&output.stdout);

        for &idx in indices {
            let line = findings[idx].line;
            if let Some(entry) = blame_map.get(&line) {
                findings[idx].blame_author = Some(entry.author.clone());
                findings[idx].blame_date = Some(entry.timestamp);
                findings[idx].blame_commit = Some(entry.commit.clone());
            }
        }
    }
}

struct BlameEntry {
    author: String,
    timestamp: i64,
    commit: String,
}

/// Parse `git blame --porcelain` output into a map of
/// line-number → (author, timestamp, commit).
fn parse_porcelain(stdout: &[u8]) -> HashMap<usize, BlameEntry> {
    let mut result = HashMap::new();

    // State for the current hunk being parsed.
    let mut current_line: Option<usize> = None;
    let mut current_author = String::new();
    let mut current_email = String::new();
    let mut current_timestamp: i64 = 0;
    let mut current_commit = String::new();

    for line in stdout.split(|&b| b == b'\n') {
        if line.is_empty() {
            continue;
        }

        // A porcelain hunk header looks like:
        //   <40-char hex sha> <orig-line> <final-line> [<num-lines>]
        // We detect it by checking the first 40 bytes are hex.
        if line.len() >= 40 && line[..40].iter().all(|b| b.is_ascii_hexdigit()) {
            // Flush the previous hunk.
            flush_entry(
                &mut result,
                &mut current_line,
                &current_author,
                &current_email,
                current_timestamp,
                &current_commit,
            );

            // Parse new header — we only need final-line and the short sha.
            let header = unsafe { std::str::from_utf8_unchecked(line) };
            let mut parts = header.split_whitespace();
            if let Some(hash) = parts.next() {
                current_commit = hash[..hash.len().min(7)].to_string();
            }
            // skip orig-line
            parts.next();
            current_line = parts.next().and_then(|s| s.parse().ok());

            current_author.clear();
            current_email.clear();
            current_timestamp = 0;
        } else if line.starts_with(b"author ") {
            current_author = String::from_utf8_lossy(&line[7..]).into_owned();
        } else if line.starts_with(b"author-mail ") {
            let raw = String::from_utf8_lossy(&line[12..]).into_owned();
            current_email = raw.trim_matches(|c| c == '<' || c == '>').to_string();
        } else if line.starts_with(b"author-time ") {
            if let Ok(t) = String::from_utf8_lossy(&line[12..]).trim().parse::<i64>() {
                current_timestamp = t;
            }
        }
    }

    // Flush the last hunk.
    flush_entry(
        &mut result,
        &mut current_line,
        &current_author,
        &current_email,
        current_timestamp,
        &current_commit,
    );

    result
}

fn flush_entry(
    map: &mut HashMap<usize, BlameEntry>,
    line: &mut Option<usize>,
    author: &str,
    email: &str,
    timestamp: i64,
    commit: &str,
) {
    if let Some(ln) = line.take() {
        let full_author = if email.is_empty() {
            author.to_string()
        } else {
            format!("{author} <{email}>")
        };
        map.insert(
            ln,
            BlameEntry {
                author: full_author,
                timestamp,
                commit: commit.to_string(),
            },
        );
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
    use std::path::PathBuf;
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

    /// Run `git init` + configure + commit via subprocess so tests exercise
    /// the same `git` binary the real code uses.
    fn init_repo_with_commit(dir: &TempDir, filename: &str, content: &str) {
        let dir = dir.path();
        git_cmd(dir, &["init"]);
        git_cmd(dir, &["config", "user.name", "Test Author"]);
        git_cmd(dir, &["config", "user.email", "author@test.com"]);
        std::fs::write(dir.join(filename), content).unwrap();
        git_cmd(dir, &["add", filename]);
        git_cmd(dir, &["commit", "-m", "initial commit"]);
    }

    fn git_cmd(cwd: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .status()
            .expect("git should be available");
        assert!(status.success(), "git {args:?} failed in {cwd:?}");
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
        assert_eq!(format_age(now_secs() - 5 * 86_400), "5 days ago");
    }

    #[test]
    fn format_age_weeks() {
        assert_eq!(format_age(now_secs() - 21 * 86_400), "3 weeks ago");
    }

    #[test]
    fn format_age_one_week_singular() {
        assert_eq!(format_age(now_secs() - 9 * 86_400), "1 week ago");
    }

    #[test]
    fn format_age_months() {
        assert_eq!(format_age(now_secs() - 90 * 86_400), "3 months ago");
    }

    #[test]
    fn format_age_one_month_singular() {
        assert_eq!(format_age(now_secs() - 30 * 86_400), "1 month ago");
    }

    #[test]
    fn format_age_years() {
        assert_eq!(format_age(now_secs() - 2 * 365 * 86_400), "2 years ago");
    }

    #[test]
    fn format_age_one_year_singular() {
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
        init_repo_with_commit(&dir, "keep", "x\n");
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
        init_repo_with_commit(&dir, "a.rs", "// TODO: in a\n");
        // Second commit for a different file.
        let file_b = dir.path().join("b.rs");
        std::fs::write(&file_b, "// FIXME: in b\n").unwrap();
        git_cmd(dir.path(), &["add", "b.rs"]);
        git_cmd(dir.path(), &["commit", "-m", "add b"]);

        let file_a = dir.path().join("a.rs");
        let mut findings = vec![plain_finding(file_a, 1), plain_finding(file_b, 1)];
        enrich_with_blame(&mut findings, dir.path());
        assert!(findings[0].blame_author.is_some());
        assert!(findings[1].blame_author.is_some());
    }
}
