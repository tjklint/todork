//! Integration tests for `--blame` flag.
//!
//! These tests spawn the real `todork` binary and set up minimal git
//! repositories using `std::process::Command` to run `git` directly.

use assert_cmd::Command;
use std::io::Write;
use std::process;
use tempfile::TempDir;

fn todork() -> Command {
    Command::cargo_bin("todork").expect("binary should be present")
}

fn write_file(dir: &TempDir, name: &str, content: &str) {
    let path = dir.path().join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let mut f = std::fs::File::create(path).unwrap();
    writeln!(f, "{content}").unwrap();
}

/// Initialize a git repo in `dir`, write one file, and commit it so
/// that git blame data is available for integration tests.
fn setup_git_repo(dir: &TempDir, filename: &str, content: &str) {
    let git = |args: &[&str]| {
        process::Command::new("git")
            .args(args)
            .current_dir(dir.path())
            // Isolate from the developer's global ~/.gitconfig so that
            // user.name / user.email are always set to our test values.
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .output()
            .expect("git should be available")
    };

    git(&["init"]);
    git(&["config", "user.email", "ci@test.com"]);
    git(&["config", "user.name", "CI User"]);
    write_file(dir, filename, content);
    git(&["add", "."]);
    git(&["commit", "--no-gpg-sign", "-m", "initial commit"]);
}

// ── basic behaviour ───────────────────────────────────────────────────────────

#[test]
fn blame_non_git_dir_still_finds_todos() {
    // --blame on a plain directory should not error; findings are returned
    // without blame fields (silently no-ops when no .git found).
    let dir = TempDir::new().unwrap();
    write_file(&dir, "a.rs", "// TODO: test");
    let output = todork()
        .args([dir.path().to_str().unwrap(), "--blame", "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let arr: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(arr.len(), 1);
    // No blame fields since not a git repo.
    assert!(arr[0].get("blame_author").is_none());
}

// ── JSON output ───────────────────────────────────────────────────────────────

#[test]
fn blame_json_has_blame_author() {
    let dir = TempDir::new().unwrap();
    setup_git_repo(&dir, "todo.rs", "// TODO: needs blame");
    let output = todork()
        .args([dir.path().to_str().unwrap(), "--blame", "--format", "json"])
        .output()
        .unwrap();
    let arr: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(arr.len(), 1);
    let author = arr[0]["blame_author"]
        .as_str()
        .expect("blame_author should be present");
    assert!(author.contains("CI User"), "got: {author}");
}

#[test]
fn blame_json_has_blame_date() {
    let dir = TempDir::new().unwrap();
    setup_git_repo(&dir, "todo.rs", "// TODO: needs blame");
    let output = todork()
        .args([dir.path().to_str().unwrap(), "--blame", "--format", "json"])
        .output()
        .unwrap();
    let arr: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    let date = arr[0]["blame_date"]
        .as_i64()
        .expect("blame_date should be a number");
    assert!(date > 0, "blame_date should be a positive Unix timestamp");
}

#[test]
fn blame_json_has_blame_commit() {
    let dir = TempDir::new().unwrap();
    setup_git_repo(&dir, "todo.rs", "// TODO: needs blame");
    let output = todork()
        .args([dir.path().to_str().unwrap(), "--blame", "--format", "json"])
        .output()
        .unwrap();
    let arr: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    let commit = arr[0]["blame_commit"]
        .as_str()
        .expect("blame_commit should be present");
    assert_eq!(
        commit.len(),
        7,
        "short hash should be 7 chars, got: {commit}"
    );
    assert!(
        commit.chars().all(|c| c.is_ascii_hexdigit()),
        "commit hash must be hex, got: {commit}"
    );
}

#[test]
fn no_blame_flag_json_has_no_blame_fields() {
    let dir = TempDir::new().unwrap();
    setup_git_repo(&dir, "todo.rs", "// TODO: no blame");
    let output = todork()
        .args([dir.path().to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    let arr: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(arr.len(), 1);
    // blame fields must be absent when --blame was not passed
    assert!(
        arr[0].get("blame_author").is_none(),
        "blame_author should be absent"
    );
    assert!(
        arr[0].get("blame_date").is_none(),
        "blame_date should be absent"
    );
    assert!(
        arr[0].get("blame_commit").is_none(),
        "blame_commit should be absent"
    );
}

// ── text output ───────────────────────────────────────────────────────────────

#[test]
fn blame_text_shows_author_line() {
    let dir = TempDir::new().unwrap();
    setup_git_repo(&dir, "todo.rs", "// TODO: blame line test");
    let output = todork()
        .args([dir.path().to_str().unwrap(), "--blame", "--color", "never"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("CI User"),
        "text output should show blame author"
    );
    assert!(
        stdout.contains("└─"),
        "text output should have tree-art prefix"
    );
}

#[test]
fn blame_text_shows_commit_hash() {
    let dir = TempDir::new().unwrap();
    setup_git_repo(&dir, "todo.rs", "// TODO: hash test");
    let output = todork()
        .args([dir.path().to_str().unwrap(), "--blame", "--color", "never"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    // There should be a parenthesized 7-char hex hash somewhere.
    assert!(
        stdout.contains('(') && stdout.contains(')'),
        "text output should contain a commit hash in parens"
    );
}

// ── github-annotations output ─────────────────────────────────────────────────

#[test]
fn blame_github_annotations_includes_author_in_message() {
    let dir = TempDir::new().unwrap();
    setup_git_repo(&dir, "todo.rs", "// TODO: gh annotation test");
    let output = todork()
        .args([
            dir.path().to_str().unwrap(),
            "--blame",
            "--format",
            "github-annotations",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("CI User"),
        "github annotation should include blame author in message"
    );
}
