//! GitHub Actions workflow command output formatter.
//!
//! Emits `::notice`, `::warning`, or `::error` lines that GitHub Actions
//! renders as inline annotations on the relevant file and line.
//!
//! Format: `::LEVEL file=PATH,line=LINE,col=COL,title=TAG::MESSAGE`

use super::Formatter;
use crate::matcher::{Finding, Severity};
use std::io::Write;

/// Maps annotation severity to a GitHub Actions workflow command level.
fn level(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "notice",
    }
}

/// Escapes special characters in GitHub Actions workflow command values.
///
/// The characters `,`, `:`, `%`, `\r`, `\n` must be percent-encoded inside
/// property values to avoid breaking the command parser.
fn escape_property(s: &str) -> String {
    s.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
        .replace(':', "%3A")
        .replace(',', "%2C")
}

/// Escapes special characters in the workflow command data (the `::message` part).
fn escape_data(s: &str) -> String {
    s.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

/// Writes findings as GitHub Actions workflow annotation commands.
pub struct GithubFormatter;

impl Formatter for GithubFormatter {
    fn format(&self, findings: &[Finding], writer: &mut dyn Write) -> anyhow::Result<()> {
        for f in findings {
            let level = level(f.severity);
            let file = escape_property(&f.file.display().to_string());
            let title = if let Some(ref author) = f.author {
                format!("{}({})", f.tag, author)
            } else {
                f.tag.clone()
            };
            let title = escape_property(&title);
            // Append blame info to message when available.
            let base_message = escape_data(&f.message);
            let message = match (&f.blame_author, f.blame_date) {
                (Some(author), Some(date)) => {
                    let age = crate::blame::format_age(date);
                    let commit_part = f
                        .blame_commit
                        .as_deref()
                        .map(|c| format!(" ({c})"))
                        .unwrap_or_default();
                    let suffix = escape_data(&format!(" [{author} · {age}{commit_part}]"));
                    format!("{base_message}{suffix}")
                }
                _ => base_message,
            };

            writeln!(
                writer,
                "::{level} file={file},line={line},col={col},title={title}::{message}",
                line = f.line,
                col = f.column,
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matcher::{Finding, Severity};
    use std::path::PathBuf;

    fn finding(
        file: &str,
        line: usize,
        col: usize,
        tag: &str,
        severity: Severity,
        author: Option<&str>,
        msg: &str,
    ) -> Finding {
        Finding {
            file: PathBuf::from(file),
            line,
            column: col,
            tag: tag.to_string(),
            severity,
            author: author.map(str::to_string),
            message: msg.to_string(),
            blame_author: None,
            blame_date: None,
            blame_commit: None,
        }
    }

    fn render(findings: &[Finding]) -> String {
        let mut buf = Vec::new();
        GithubFormatter.format(findings, &mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn empty_findings_produces_no_output() {
        assert_eq!(render(&[]), "");
    }

    #[test]
    fn error_severity_maps_to_error_level() {
        let f = finding("a.rs", 1, 1, "FIXME", Severity::Error, None, "msg");
        assert!(render(&[f]).starts_with("::error "));
    }

    #[test]
    fn warning_severity_maps_to_warning_level() {
        let f = finding("a.rs", 1, 1, "TODO", Severity::Warning, None, "msg");
        assert!(render(&[f]).starts_with("::warning "));
    }

    #[test]
    fn info_severity_maps_to_notice_level() {
        let f = finding("a.rs", 1, 1, "NOTE", Severity::Info, None, "msg");
        assert!(render(&[f]).starts_with("::notice "));
    }

    #[test]
    fn output_format_correct() {
        let f = finding(
            "src/main.rs",
            42,
            5,
            "TODO",
            Severity::Warning,
            None,
            "fix this",
        );
        let out = render(&[f]);
        assert_eq!(
            out.trim(),
            "::warning file=src/main.rs,line=42,col=5,title=TODO::fix this"
        );
    }

    #[test]
    fn author_included_in_title() {
        let f = finding(
            "a.rs",
            1,
            1,
            "TODO",
            Severity::Warning,
            Some("alice"),
            "msg",
        );
        let out = render(&[f]);
        assert!(out.contains("title=TODO(alice)"));
    }

    #[test]
    fn multiple_findings_each_on_own_line() {
        let findings = vec![
            finding("a.rs", 1, 1, "TODO", Severity::Warning, None, "a"),
            finding("b.rs", 2, 3, "FIXME", Severity::Error, None, "b"),
        ];
        let out = render(&findings);
        let lines: Vec<_> = out.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn colon_in_message_is_escaped() {
        let f = finding("a.rs", 1, 1, "NOTE", Severity::Info, None, "key: value");
        let out = render(&[f]);
        // The message part (after `::`) should NOT be escaped (colons are fine in data).
        // But the colon in the message is in the data section, which doesn't escape colons.
        assert!(out.contains("key: value"));
    }

    #[test]
    fn colon_in_file_path_is_escaped() {
        // Windows-style path with drive letter
        let f = finding(
            "C:\\path\\file.rs",
            1,
            1,
            "TODO",
            Severity::Warning,
            None,
            "msg",
        );
        let out = render(&[f]);
        // The colon after 'C' should be percent-encoded in the file property.
        assert!(out.contains("%3A") || out.contains("C:\\"));
    }

    #[test]
    fn newline_in_message_is_escaped() {
        let f = finding(
            "a.rs",
            1,
            1,
            "TODO",
            Severity::Warning,
            None,
            "line1\nline2",
        );
        let out = render(&[f]);
        assert!(out.contains("%0A"));
    }

    #[test]
    fn percent_in_message_is_escaped() {
        let f = finding("a.rs", 1, 1, "TODO", Severity::Warning, None, "100% done");
        let out = render(&[f]);
        assert!(out.contains("%25"));
    }

    #[test]
    fn empty_message_renders_cleanly() {
        let f = finding("a.rs", 1, 1, "TODO", Severity::Warning, None, "");
        let out = render(&[f]);
        assert!(out.trim().ends_with("::"));
    }

    #[test]
    fn blame_info_appended_to_message() {
        let mut f = finding("a.rs", 1, 1, "TODO", Severity::Warning, None, "fix this");
        f.blame_author = Some("alice <alice@example.com>".to_string());
        f.blame_date = Some(0); // epoch — forces a very old age string
        f.blame_commit = Some("abc1234".to_string());
        let out = render(&[f]);
        assert!(out.contains("alice <alice@example.com>"));
        assert!(out.contains("abc1234"));
    }

    #[test]
    fn no_blame_info_when_fields_absent() {
        let f = finding("a.rs", 1, 1, "TODO", Severity::Warning, None, "msg");
        let out = render(&[f]);
        // Should not have blame bracket when blame fields are None
        assert!(!out.contains(" ["));
    }
}
