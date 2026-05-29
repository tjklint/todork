//! End-to-end integration tests for every CLI flag.
//!
//! These spawn the real `todork` binary so they test the full pipeline
//! including argument parsing, output serialization, and exit codes.

use assert_cmd::Command;
use std::io::Write;
use tempfile::TempDir;

fn todork() -> Command {
    Command::cargo_bin("todork").expect("binary should be present")
}

/// Write `content` to `name` inside `dir`.
fn write_file(dir: &TempDir, name: &str, content: &str) {
    let path = dir.path().join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let mut f = std::fs::File::create(path).unwrap();
    writeln!(f, "{content}").unwrap();
}

fn samples_dir() -> String {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("samples")
        .to_str()
        .unwrap()
        .to_string()
}

// ── --format text (default) ───────────────────────────────────────────────────

#[test]
fn text_format_contains_tag_and_file() {
    let dir = TempDir::new().unwrap();
    write_file(&dir, "a.rs", "// TODO: hello");
    todork()
        .args([dir.path().to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicates::str::contains("TODO"))
        .stdout(predicates::str::contains("a.rs"));
}

// ── --format json ─────────────────────────────────────────────────────────────

#[test]
fn json_format_produces_valid_json() {
    let output = todork()
        .args([&samples_dir(), "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let _: Vec<serde_json::Value> =
        serde_json::from_slice(&output.stdout).expect("must be valid JSON");
}

#[test]
fn json_format_empty_dir_produces_empty_array() {
    let dir = TempDir::new().unwrap();
    let output = todork()
        .args([dir.path().to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1)); // NotFound
    let parsed: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert!(parsed.is_empty());
}

#[test]
fn json_contains_expected_fields() {
    let dir = TempDir::new().unwrap();
    write_file(&dir, "f.rs", "// TODO(alice): test message");
    let output = todork()
        .args([dir.path().to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    let arr: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(arr.len(), 1);
    let obj = &arr[0];
    assert_eq!(obj["tag"], "TODO");
    assert_eq!(obj["author"], "alice");
    assert_eq!(obj["message"], "test message");
    assert!(obj["line"].as_u64().unwrap() > 0);
    assert!(obj["column"].as_u64().unwrap() > 0);
}

#[test]
fn json_total_count_matches_samples() {
    let output = todork()
        .args([&samples_dir(), "--format", "json"])
        .output()
        .unwrap();
    let arr: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(arr.len(), 41, "samples should have exactly 41 annotations");
}

// ── --format github-annotations ───────────────────────────────────────────────

#[test]
fn github_annotations_format_produces_correct_prefix() {
    let dir = TempDir::new().unwrap();
    write_file(&dir, "a.rs", "// FIXME: crash");
    let output = todork()
        .args([
            dir.path().to_str().unwrap(),
            "--format",
            "github-annotations",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.starts_with("::error ") || stdout.starts_with("::warning "));
}

#[test]
fn github_annotations_empty_dir_no_output() {
    let dir = TempDir::new().unwrap();
    let output = todork()
        .args([
            dir.path().to_str().unwrap(),
            "--format",
            "github-annotations",
        ])
        .output()
        .unwrap();
    assert!(output.stdout.is_empty());
    assert_eq!(output.status.code(), Some(1));
}

// ── --tags ────────────────────────────────────────────────────────────────────

#[test]
fn tags_filter_fixme_only() {
    let output = todork()
        .args([&samples_dir(), "--format", "json", "--tags", "fixme"])
        .output()
        .unwrap();
    let arr: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert!(!arr.is_empty());
    for item in &arr {
        assert_eq!(item["tag"], "FIXME");
    }
}

#[test]
fn tags_filter_case_insensitive() {
    let output = todork()
        .args([&samples_dir(), "--format", "json", "--tags", "Todo"])
        .output()
        .unwrap();
    let arr: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    for item in &arr {
        assert_eq!(item["tag"], "TODO");
    }
}

#[test]
fn tags_filter_multiple() {
    let output = todork()
        .args([&samples_dir(), "--format", "json", "--tags", "todo,fixme"])
        .output()
        .unwrap();
    let arr: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    for item in &arr {
        let tag = item["tag"].as_str().unwrap();
        assert!(tag == "TODO" || tag == "FIXME");
    }
}

// ── --include ─────────────────────────────────────────────────────────────────

#[test]
fn include_glob_py_excludes_js_ts() {
    let output = todork()
        .args([&samples_dir(), "--format", "json", "--include", "*.py"])
        .output()
        .unwrap();
    let arr: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert!(!arr.is_empty());
    for item in &arr {
        let file = item["file"].as_str().unwrap();
        assert!(file.ends_with(".py"), "expected .py file, got {file}");
    }
}

#[test]
fn include_glob_rs_finds_nothing_in_samples() {
    // samples/ has .py, .js, .ts, .zig, .qs, .lua — no .rs files.
    let output = todork()
        .args([&samples_dir(), "--format", "json", "--include", "*.rs"])
        .output()
        .unwrap();
    let arr: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert!(arr.is_empty());
    assert_eq!(output.status.code(), Some(1));
}

// ── --exclude ─────────────────────────────────────────────────────────────────

#[test]
fn exclude_glob_skips_python_files() {
    let output = todork()
        .args([&samples_dir(), "--format", "json", "--exclude", "*.py"])
        .output()
        .unwrap();
    let arr: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    for item in &arr {
        let file = item["file"].as_str().unwrap();
        assert!(
            !file.ends_with(".py"),
            "python file should be excluded: {file}"
        );
    }
}

#[test]
fn exclude_glob_skips_directory() {
    let output = todork()
        .args([&samples_dir(), "--format", "json", "--exclude", "*/node/*"])
        .output()
        .unwrap();
    let arr: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    for item in &arr {
        let file = item["file"].as_str().unwrap();
        assert!(
            !file.contains("/node/"),
            "node file should be excluded: {file}"
        );
    }
}

// ── --no-gitignore ────────────────────────────────────────────────────────────

#[test]
fn no_gitignore_finds_ignored_files() {
    let dir = TempDir::new().unwrap();
    write_file(&dir, ".gitignore", "hidden.rs");
    write_file(
        &dir,
        "hidden.rs",
        "// TODO: should appear with --no-gitignore",
    );

    // Without --no-gitignore: 0 findings.
    let out1 = todork()
        .args([dir.path().to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    let arr1: Vec<serde_json::Value> = serde_json::from_slice(&out1.stdout).unwrap();
    assert!(arr1.is_empty());

    // With --no-gitignore: 1 finding.
    let out2 = todork()
        .args([
            dir.path().to_str().unwrap(),
            "--format",
            "json",
            "--no-gitignore",
        ])
        .output()
        .unwrap();
    let arr2: Vec<serde_json::Value> = serde_json::from_slice(&out2.stdout).unwrap();
    assert_eq!(arr2.len(), 1);
}

// ── --hidden ──────────────────────────────────────────────────────────────────

#[test]
fn hidden_finds_dot_files() {
    let dir = TempDir::new().unwrap();
    write_file(&dir, ".secret.rs", "// TODO: hidden file");

    // Without --hidden: 0 findings.
    let out1 = todork()
        .args([dir.path().to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    let arr1: Vec<serde_json::Value> = serde_json::from_slice(&out1.stdout).unwrap();
    assert!(arr1.is_empty());

    // With --hidden: 1 finding.
    let out2 = todork()
        .args([dir.path().to_str().unwrap(), "--format", "json", "--hidden"])
        .output()
        .unwrap();
    let arr2: Vec<serde_json::Value> = serde_json::from_slice(&out2.stdout).unwrap();
    assert_eq!(arr2.len(), 1);
}

// ── --exit-zero ───────────────────────────────────────────────────────────────

#[test]
fn exit_zero_empty_dir_still_exits_1() {
    // --exit-zero only changes exit code when *findings* exist.
    // An empty directory still exits 1 (NotFound) because there's nothing.
    let dir = TempDir::new().unwrap();
    todork()
        .args([dir.path().to_str().unwrap(), "--exit-zero"])
        .assert()
        .code(1);
}

#[test]
fn exit_zero_with_findings_exits_0() {
    let dir = TempDir::new().unwrap();
    write_file(&dir, "t.rs", "// TODO: important");
    todork()
        .args([dir.path().to_str().unwrap(), "--exit-zero"])
        .assert()
        .success(); // exit 0
}

#[test]
fn without_exit_zero_findings_exits_0() {
    // Default: annotations found → exit 0 (Success).
    let dir = TempDir::new().unwrap();
    write_file(&dir, "t.rs", "// TODO: something");
    todork()
        .args([dir.path().to_str().unwrap()])
        .assert()
        .success();
}

// ── --color ───────────────────────────────────────────────────────────────────

#[test]
fn color_never_produces_no_ansi_codes() {
    let dir = TempDir::new().unwrap();
    write_file(&dir, "a.rs", "// TODO: test");
    let output = todork()
        .args([dir.path().to_str().unwrap(), "--color", "never"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        !stdout.contains('\x1b'),
        "ANSI escape found in --color=never output"
    );
}

// ── --max-depth ───────────────────────────────────────────────────────────────

#[test]
fn max_depth_1_does_not_scan_subdirs() {
    let dir = TempDir::new().unwrap();
    write_file(&dir, "top.rs", "// TODO: top");
    std::fs::create_dir(dir.path().join("sub")).unwrap();
    write_file(&dir, "sub/deep.rs", "// TODO: deep");

    let out = todork()
        .args([
            dir.path().to_str().unwrap(),
            "--format",
            "json",
            "--max-depth",
            "1",
        ])
        .output()
        .unwrap();
    let arr: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    // Only the top-level file should be found.
    assert_eq!(arr.len(), 1);
    assert!(arr[0]["file"].as_str().unwrap().ends_with("top.rs"));
}

// ── --max-filesize ────────────────────────────────────────────────────────────

#[test]
fn max_filesize_skips_large_file() {
    let dir = TempDir::new().unwrap();
    write_file(&dir, "big.rs", "// TODO: in a big file");
    // Set limit to 1 byte — the file is larger.
    let out = todork()
        .args([
            dir.path().to_str().unwrap(),
            "--format",
            "json",
            "--max-filesize",
            "1",
        ])
        .output()
        .unwrap();
    let arr: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert!(arr.is_empty());
}

// ── --threads ─────────────────────────────────────────────────────────────────

#[test]
fn threads_1_produces_same_results_as_default() {
    let out_default = todork()
        .args([&samples_dir(), "--format", "json"])
        .output()
        .unwrap();
    let out_1thread = todork()
        .args([&samples_dir(), "--format", "json", "--threads", "1"])
        .output()
        .unwrap();

    let mut arr_default: Vec<serde_json::Value> =
        serde_json::from_slice(&out_default.stdout).unwrap();
    let mut arr_1thread: Vec<serde_json::Value> =
        serde_json::from_slice(&out_1thread.stdout).unwrap();

    // Sort both by file+line so ordering is deterministic.
    let key = |v: &serde_json::Value| {
        format!(
            "{}:{}",
            v["file"].as_str().unwrap_or(""),
            v["line"].as_u64().unwrap_or(0)
        )
    };
    arr_default.sort_by_key(key);
    arr_1thread.sort_by_key(key);

    assert_eq!(arr_default, arr_1thread);
}

// ── --sort ────────────────────────────────────────────────────────────────────

#[test]
fn sort_path_is_default_and_ordered_by_file_then_line() {
    let dir = TempDir::new().unwrap();
    write_file(&dir, "b.rs", "// TODO: second file");
    write_file(&dir, "a.rs", "// TODO: first file");
    let out = todork()
        .args([dir.path().to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    let arr: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(arr.len(), 2);
    assert!(
        arr[0]["file"].as_str().unwrap() < arr[1]["file"].as_str().unwrap(),
        "default sort should order by file path"
    );
}

#[test]
fn sort_path_explicit_matches_default() {
    let dir = TempDir::new().unwrap();
    write_file(&dir, "a.rs", "// TODO: line 1\n// FIXME: line 2");
    let out_default = todork()
        .args([dir.path().to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    let out_explicit = todork()
        .args([
            dir.path().to_str().unwrap(),
            "--format",
            "json",
            "--sort",
            "path",
        ])
        .output()
        .unwrap();
    assert_eq!(out_default.stdout, out_explicit.stdout);
}

#[test]
fn sort_oldest_produces_same_findings_as_default() {
    // Without a git repo, blame_date is None for all findings.
    // Stable sort keeps count equal; we just verify no findings are lost.
    let dir = TempDir::new().unwrap();
    write_file(&dir, "a.rs", "// TODO: alpha");
    write_file(&dir, "b.rs", "// TODO: beta");
    let out = todork()
        .args([
            dir.path().to_str().unwrap(),
            "--format",
            "json",
            "--sort",
            "oldest",
        ])
        .output()
        .unwrap();
    let arr: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(arr.len(), 2, "--sort oldest should not drop any findings");
}

#[test]
fn sort_newest_produces_same_findings_as_default() {
    let dir = TempDir::new().unwrap();
    write_file(&dir, "a.rs", "// TODO: alpha");
    write_file(&dir, "b.rs", "// TODO: beta");
    let out = todork()
        .args([
            dir.path().to_str().unwrap(),
            "--format",
            "json",
            "--sort",
            "newest",
        ])
        .output()
        .unwrap();
    let arr: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(arr.len(), 2, "--sort newest should not drop any findings");
}

#[test]
fn sort_short_flag_works() {
    let dir = TempDir::new().unwrap();
    write_file(&dir, "a.rs", "// TODO: test");
    todork()
        .args([
            dir.path().to_str().unwrap(),
            "--format",
            "json",
            "-s",
            "oldest",
        ])
        .assert()
        .success();
}

// ── todork upgrade ────────────────────────────────────────────────────────────

#[test]
fn upgrade_subcommand_is_recognised() {
    // Without a network, upgrade will fail — but argument parsing must succeed,
    // which means the binary should not exit with code 2 (bad args).
    let output = todork().arg("upgrade").output().unwrap();
    let exit = output.status.code().unwrap_or(-1);
    assert_ne!(exit, 2, "upgrade should be a recognised subcommand, not an unknown arg");
}

#[test]
fn help_subcommand_exits_zero() {
    todork().arg("help").assert().success();
}

#[test]
fn help_flag_exits_zero() {
    todork().arg("--help").assert().success();
}

#[test]
fn version_flag_exits_zero() {
    todork().arg("--version").assert().success();
}

#[test]
fn upgrade_help_shows_description() {
    let output = todork().args(["help", "upgrade"]).output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("Upgrade") || stdout.contains("upgrade"),
        "help upgrade should describe the upgrade command"
    );
}
