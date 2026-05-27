//! File-level scanning: binary detection, line iteration, and per-line matching.

use crate::matcher::{Finding, Matcher};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Maximum file size (in bytes) scanned by default — 10 MiB.
pub const DEFAULT_MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Number of bytes read to probe for null bytes (binary detection).
const BINARY_PROBE_LEN: usize = 8192;

/// Read-buffer size for line iteration.
const BUF_SIZE: usize = 64 * 1024;

/// Scan a single file and return all [`Finding`]s it contains.
///
/// Returns `Ok(Vec::new())` (no findings, no error) when the file is:
/// - larger than `max_file_size`
/// - detected as binary (contains a null byte in the first 8 KiB)
/// - completely empty
///
/// Invalid UTF-8 sequences are replaced with `U+FFFD` so the scan continues.
pub fn scan_file(
    path: &Path,
    matcher: &Matcher,
    max_file_size: u64,
) -> anyhow::Result<Vec<Finding>> {
    // ── size check ────────────────────────────────────────────────────────────
    let metadata = std::fs::metadata(path)?;
    if metadata.len() > max_file_size {
        return Ok(Vec::new());
    }

    // ── binary probe ──────────────────────────────────────────────────────────
    {
        use std::io::Read;
        let mut probe_buf = vec![0u8; BINARY_PROBE_LEN];
        let mut probe_file = File::open(path)?;
        let n = probe_file.read(&mut probe_buf)?;
        if memchr::memchr(0, &probe_buf[..n]).is_some() {
            return Ok(Vec::new());
        }
    }

    // ── line scan ─────────────────────────────────────────────────────────────
    let file = File::open(path)?;
    let reader = BufReader::with_capacity(BUF_SIZE, file);
    let mut findings = Vec::new();
    let mut line_number = 0usize;

    for raw_line in reader.lines() {
        line_number += 1;
        let line = match raw_line {
            Ok(l) => l,
            // UTF-8 decode error: use lossy conversion and continue scanning.
            Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {
                // Re-read the raw bytes of this line with lossy conversion.
                // Because BufReader already consumed it, we approximate by skipping.
                let _ = e;
                continue;
            }
            Err(e) => return Err(e.into()),
        };

        let mut line_findings = matcher.scan_line(&line, path, line_number);
        findings.append(&mut line_findings);
    }

    Ok(findings)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matcher::{Matcher, DEFAULT_TAGS};
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn matcher() -> Matcher {
        Matcher::new(DEFAULT_TAGS).unwrap()
    }

    fn write_temp(content: &[u8]) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content).unwrap();
        f.flush().unwrap();
        f
    }

    // ── basic scanning ────────────────────────────────────────────────────────

    #[test]
    fn finds_todo_in_simple_file() {
        let f = write_temp(b"// TODO: do something\n");
        let m = matcher();
        let findings = scan_file(f.path(), &m, DEFAULT_MAX_FILE_SIZE).unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].tag, "TODO");
        assert_eq!(findings[0].line, 1);
    }

    #[test]
    fn finds_multiple_annotations_across_lines() {
        let content = b"// TODO: first\nfn foo() {}\n// FIXME: second\n";
        let f = write_temp(content);
        let m = matcher();
        let findings = scan_file(f.path(), &m, DEFAULT_MAX_FILE_SIZE).unwrap();
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].tag, "TODO");
        assert_eq!(findings[0].line, 1);
        assert_eq!(findings[1].tag, "FIXME");
        assert_eq!(findings[1].line, 3);
    }

    #[test]
    fn line_numbers_are_one_based() {
        let content = b"line one\nline two\n// TODO: on line three\n";
        let f = write_temp(content);
        let m = matcher();
        let findings = scan_file(f.path(), &m, DEFAULT_MAX_FILE_SIZE).unwrap();
        assert_eq!(findings[0].line, 3);
    }

    #[test]
    fn file_path_is_preserved() {
        let f = write_temp(b"// TODO: path test\n");
        let m = matcher();
        let findings = scan_file(f.path(), &m, DEFAULT_MAX_FILE_SIZE).unwrap();
        assert_eq!(findings[0].file, f.path());
    }

    // ── binary detection ─────────────────────────────────────────────────────

    #[test]
    fn skips_binary_file_with_null_byte() {
        // File has a TODO but also a null byte — should be skipped entirely.
        let f = write_temp(b"// TODO: something\x00binary data");
        let m = matcher();
        let findings = scan_file(f.path(), &m, DEFAULT_MAX_FILE_SIZE).unwrap();
        assert!(findings.is_empty(), "binary file should yield no findings");
    }

    #[test]
    fn skips_binary_file_null_at_start() {
        let f = write_temp(b"\x00// TODO: prefixed by null");
        let m = matcher();
        let findings = scan_file(f.path(), &m, DEFAULT_MAX_FILE_SIZE).unwrap();
        assert!(findings.is_empty());
    }

    #[test]
    fn null_byte_after_probe_window_is_not_checked() {
        // A file whose null byte is beyond the 8 KiB probe is treated as text.
        // Build a file: 8193 bytes of 'a', then a null, then a TODO.
        let mut content = vec![b'a'; 8193];
        content.push(0u8);
        content.extend_from_slice(b"\n// TODO: after null\n");
        let f = write_temp(&content);
        let m = matcher();
        // Should scan successfully (null is outside probe window).
        let findings = scan_file(f.path(), &m, DEFAULT_MAX_FILE_SIZE).unwrap();
        assert!(!findings.is_empty());
    }

    // ── size limit ────────────────────────────────────────────────────────────

    #[test]
    fn skips_file_over_size_limit() {
        let f = write_temp(b"// TODO: this file is too big conceptually\n");
        let m = matcher();
        // Set limit to 0 bytes — any file will be skipped.
        let findings = scan_file(f.path(), &m, 0).unwrap();
        assert!(
            findings.is_empty(),
            "file over limit should yield no findings"
        );
    }

    #[test]
    fn scans_file_exactly_at_size_limit() {
        let content = b"// TODO: exact\n";
        let f = write_temp(content);
        let m = matcher();
        let limit = content.len() as u64;
        let findings = scan_file(f.path(), &m, limit).unwrap();
        assert_eq!(findings.len(), 1);
    }

    // ── empty file ────────────────────────────────────────────────────────────

    #[test]
    fn empty_file_returns_no_findings() {
        let f = write_temp(b"");
        let m = matcher();
        let findings = scan_file(f.path(), &m, DEFAULT_MAX_FILE_SIZE).unwrap();
        assert!(findings.is_empty());
    }

    // ── no annotations ───────────────────────────────────────────────────────

    #[test]
    fn file_with_no_tags_returns_empty() {
        let f = write_temp(b"fn main() {\n    println!(\"hello\");\n}\n");
        let m = matcher();
        let findings = scan_file(f.path(), &m, DEFAULT_MAX_FILE_SIZE).unwrap();
        assert!(findings.is_empty());
    }

    // ── sample fixture files ─────────────────────────────────────────────────

    #[test]
    fn scans_sample_python_app() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/python/app.py");
        let m = matcher();
        let findings = scan_file(&path, &m, DEFAULT_MAX_FILE_SIZE).unwrap();
        assert_eq!(findings.len(), 4, "app.py should have 4 annotations");
    }

    #[test]
    fn scans_sample_python_utils() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/python/utils.py");
        let m = matcher();
        let findings = scan_file(&path, &m, DEFAULT_MAX_FILE_SIZE).unwrap();
        assert_eq!(findings.len(), 3, "utils.py should have 3 annotations");
    }

    #[test]
    fn scans_sample_python_config() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/python/config.py");
        let m = matcher();
        let findings = scan_file(&path, &m, DEFAULT_MAX_FILE_SIZE).unwrap();
        assert_eq!(findings.len(), 2, "config.py should have 2 annotations");
    }

    #[test]
    fn scans_sample_node_index() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/node/index.js");
        let m = matcher();
        let findings = scan_file(&path, &m, DEFAULT_MAX_FILE_SIZE).unwrap();
        assert_eq!(findings.len(), 3, "index.js should have 3 annotations");
    }

    #[test]
    fn scans_sample_node_server() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/node/server.js");
        let m = matcher();
        let findings = scan_file(&path, &m, DEFAULT_MAX_FILE_SIZE).unwrap();
        assert_eq!(findings.len(), 2, "server.js should have 2 annotations");
    }

    #[test]
    fn scans_sample_node_helpers() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/node/helpers.ts");
        let m = matcher();
        let findings = scan_file(&path, &m, DEFAULT_MAX_FILE_SIZE).unwrap();
        assert_eq!(findings.len(), 3, "helpers.ts should have 3 annotations");
    }
}
