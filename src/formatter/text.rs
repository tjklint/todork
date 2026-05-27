//! Human-readable, optionally-coloured text output (ripgrep-style).

use super::Formatter;
use crate::matcher::{Finding, Severity};
use std::io::Write;
use termcolor::{Color, ColorSpec, WriteColor};

/// Writes findings as coloured text grouped by file.
///
/// Each line looks like:
/// ```text
/// path/to/file.rs:12:3: TODO: message
/// path/to/file.rs:15:3: TODO(alice): message
/// ```
///
/// Files are separated by a blank line.  A summary line is printed at the end.
pub struct TextFormatter<W: WriteColor> {
    writer: W,
    /// When `true`, emit ANSI colour codes; when `false`, plain text.
    colour: bool,
}

impl<W: WriteColor> TextFormatter<W> {
    pub fn new(writer: W, colour: bool) -> Self {
        Self { writer, colour }
    }

    fn set(&mut self, spec: &ColorSpec) -> anyhow::Result<()> {
        if self.colour {
            self.writer.set_color(spec)?;
        }
        Ok(())
    }

    fn reset(&mut self) -> anyhow::Result<()> {
        if self.colour {
            self.writer.reset()?;
        }
        Ok(())
    }

    fn write_finding(&mut self, f: &Finding) -> anyhow::Result<()> {
        // Filename — bold cyan
        self.set(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true))?;
        write!(self.writer, "{}", f.file.display())?;
        self.reset()?;

        // :line:col — green
        self.set(ColorSpec::new().set_fg(Some(Color::Green)))?;
        write!(self.writer, ":{}", f.line)?;
        // Only print column when it's > 1 (matches common conventions).
        if f.column > 1 {
            write!(self.writer, ":{}", f.column)?;
        }
        write!(self.writer, ":")?;
        self.reset()?;

        write!(self.writer, " ")?;

        // Tag — colour by severity
        let tag_spec = match f.severity {
            Severity::Error => ColorSpec::new()
                .set_fg(Some(Color::Red))
                .set_bold(true)
                .clone(),
            Severity::Warning => ColorSpec::new()
                .set_fg(Some(Color::Yellow))
                .set_bold(true)
                .clone(),
            Severity::Info => ColorSpec::new().set_fg(Some(Color::Blue)).clone(),
        };
        self.set(&tag_spec)?;
        if let Some(ref author) = f.author {
            write!(self.writer, "{}({}):", f.tag, author)?;
        } else {
            write!(self.writer, "{}:", f.tag)?;
        }
        self.reset()?;

        // Message — default colour
        if !f.message.is_empty() {
            write!(self.writer, " {}", f.message)?;
        }

        writeln!(self.writer)?;

        // Blame line — shown only when --blame was used and data is available.
        if let (Some(ref author), Some(date)) = (&f.blame_author, f.blame_date) {
            let age = crate::blame::format_age(date);
            let commit_part = f
                .blame_commit
                .as_deref()
                .map(|c| format!("  ({c})"))
                .unwrap_or_default();

            self.set(ColorSpec::new().set_fg(Some(Color::White)))?;
            write!(self.writer, "  └─ {author}  ·  {age}{commit_part}")?;
            self.reset()?;
            writeln!(self.writer)?;
        }

        Ok(())
    }
}

impl<W: WriteColor + Write> Formatter for TextFormatter<W> {
    fn format(&self, findings: &[Finding], _writer: &mut dyn Write) -> anyhow::Result<()> {
        // This signature exists for trait object use, but TextFormatter owns its
        // writer, so the real work happens in `write_all`.
        let _ = findings;
        Ok(())
    }
}

impl<W: WriteColor> TextFormatter<W> {
    /// Write all findings grouped by file, then a summary line with elapsed time.
    pub fn write_all(
        &mut self,
        findings: &[Finding],
        elapsed: std::time::Duration,
    ) -> anyhow::Result<()> {
        if findings.is_empty() {
            return Ok(());
        }

        let mut current_file = findings[0].file.as_path();
        let mut first_group = true;

        for finding in findings {
            if finding.file.as_path() != current_file {
                writeln!(self.writer)?; // blank line between file groups
                current_file = finding.file.as_path();
                first_group = false;
            } else if first_group {
                first_group = false;
            }
            self.write_finding(finding)?;
        }

        // Summary line
        writeln!(self.writer)?;
        let file_count = {
            let mut files: Vec<_> = findings.iter().map(|f| &f.file).collect();
            files.dedup();
            files.len()
        };
        self.set(ColorSpec::new().set_bold(true))?;
        write!(
            self.writer,
            "Found {} annotation{} across {} file{}.",
            findings.len(),
            if findings.len() == 1 { "" } else { "s" },
            file_count,
            if file_count == 1 { "" } else { "s" },
        )?;
        self.reset()?;

        // Elapsed time — dim/gray, no emoji
        self.set(ColorSpec::new().set_dimmed(true))?;
        write!(self.writer, "  {}", format_elapsed(elapsed))?;
        self.reset()?;

        writeln!(self.writer)?;
        Ok(())
    }
}

/// Format a duration as a compact human-readable string.
///
/// Examples: `0.028s`, `1.23s`, `12.3s`
fn format_elapsed(d: std::time::Duration) -> String {
    let secs = d.as_secs_f64();
    if secs < 1.0 {
        format!("{secs:.3}s")
    } else if secs < 10.0 {
        format!("{secs:.2}s")
    } else {
        format!("{secs:.1}s")
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matcher::{Finding, Severity};
    use std::path::PathBuf;
    use termcolor::NoColor;

    fn make_finding(
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
        let buf = Vec::new();
        let mut fmt = TextFormatter::new(NoColor::new(buf), false);
        fmt.write_all(findings, std::time::Duration::ZERO).unwrap();
        String::from_utf8(fmt.writer.into_inner()).unwrap()
    }

    #[test]
    fn empty_findings_produces_no_output() {
        assert_eq!(render(&[]), "");
    }

    #[test]
    fn single_todo_renders_correctly() {
        let f = make_finding("src/a.rs", 10, 4, "TODO", Severity::Warning, None, "do it");
        let out = render(&[f]);
        assert!(out.contains("src/a.rs"));
        assert!(out.contains(":10:4:"));
        assert!(out.contains("TODO:"));
        assert!(out.contains("do it"));
    }

    #[test]
    fn finding_with_author_renders_parentheses() {
        let f = make_finding(
            "a.rs",
            1,
            1,
            "TODO",
            Severity::Warning,
            Some("alice"),
            "refactor",
        );
        let out = render(&[f]);
        assert!(out.contains("TODO(alice):"));
        assert!(out.contains("refactor"));
    }

    #[test]
    fn findings_from_same_file_have_no_blank_line_between_them() {
        let findings = vec![
            make_finding("f.rs", 1, 1, "TODO", Severity::Warning, None, "a"),
            make_finding("f.rs", 2, 1, "FIXME", Severity::Error, None, "b"),
        ];
        let out = render(&findings);
        // No consecutive blank lines within the same file group.
        assert!(!out.contains("\n\n\n"));
    }

    #[test]
    fn different_files_are_separated_by_blank_line() {
        let findings = vec![
            make_finding("a.rs", 1, 1, "TODO", Severity::Warning, None, "a"),
            make_finding("b.rs", 1, 1, "FIXME", Severity::Error, None, "b"),
        ];
        let out = render(&findings);
        assert!(out.contains("\n\n"));
    }

    #[test]
    fn summary_line_singular() {
        let f = make_finding("a.rs", 1, 1, "TODO", Severity::Warning, None, "x");
        let out = render(&[f]);
        assert!(out.contains("Found 1 annotation across 1 file."));
    }

    #[test]
    fn summary_line_plural() {
        let findings = vec![
            make_finding("a.rs", 1, 1, "TODO", Severity::Warning, None, "x"),
            make_finding("b.rs", 2, 1, "FIXME", Severity::Error, None, "y"),
        ];
        let out = render(&findings);
        assert!(out.contains("Found 2 annotations across 2 files."));
    }

    #[test]
    fn summary_multiple_findings_one_file() {
        let findings = vec![
            make_finding("a.rs", 1, 1, "TODO", Severity::Warning, None, "x"),
            make_finding("a.rs", 2, 1, "FIXME", Severity::Error, None, "y"),
        ];
        let out = render(&findings);
        assert!(out.contains("Found 2 annotations across 1 file."));
    }

    #[test]
    fn column_one_omitted() {
        let f = make_finding("a.rs", 5, 1, "TODO", Severity::Warning, None, "x");
        let out = render(&[f]);
        // Column 1 is omitted, so we expect ":5:" not ":5:1:"
        assert!(out.contains(":5:"));
        assert!(!out.contains(":5:1:"));
    }

    #[test]
    fn column_greater_than_one_included() {
        let f = make_finding("a.rs", 5, 4, "TODO", Severity::Warning, None, "x");
        let out = render(&[f]);
        assert!(out.contains(":5:4:"));
    }

    #[test]
    fn empty_message_still_renders_tag() {
        let f = make_finding("a.rs", 1, 1, "TODO", Severity::Warning, None, "");
        let out = render(&[f]);
        assert!(out.contains("TODO:"));
    }

    #[test]
    fn blame_line_rendered_when_present() {
        let mut f = make_finding("a.rs", 1, 1, "TODO", Severity::Warning, None, "fix");
        f.blame_author = Some("alice <alice@example.com>".to_string());
        f.blame_date = Some(0); // epoch → very old
        f.blame_commit = Some("abc1234".to_string());
        let out = render(&[f]);
        assert!(out.contains("└─"), "should have tree-art prefix");
        assert!(out.contains("alice <alice@example.com>"));
        assert!(out.contains("(abc1234)"));
    }

    #[test]
    fn no_blame_line_when_blame_absent() {
        let f = make_finding("a.rs", 1, 1, "TODO", Severity::Warning, None, "fix");
        let out = render(&[f]);
        assert!(!out.contains("└─"));
    }
}
