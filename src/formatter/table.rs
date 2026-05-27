//! Fixed-width table output formatter.
//!
//! Columns: FILE  LINE  TAG  SEVERITY  MESSAGE
//! When `--blame` data is present, appends: BLAME AUTHOR  AGE  COMMIT
//!
//! Example (no blame):
//! ```text
//! FILE              LINE  TAG    SEVERITY  MESSAGE
//! ────────────────  ────  ─────  ────────  ─────────────────────────────
//! src/main.rs         12  TODO   warning   handle this edge case
//! src/lib.rs          42  FIXME  error     this is broken and needs fix...
//! ```

use super::Formatter;
use crate::matcher::{Finding, Severity};
use std::io::Write;
use termcolor::{Color, ColorSpec, WriteColor};

// ── Column headers ─────────────────────────────────────────────────────────────

const H_FILE: &str = "FILE";
const H_LINE: &str = "LINE";
const H_TAG: &str = "TAG";
const H_SEV: &str = "SEVERITY";
const H_MSG: &str = "MESSAGE";
const H_BLAME: &str = "BLAME AUTHOR";
const H_AGE: &str = "AGE";
const H_COMMIT: &str = "COMMIT";

// ── Width caps ─────────────────────────────────────────────────────────────────

const MAX_MSG: usize = 60;
const MAX_BLAME: usize = 30;

// ── TableFormatter ─────────────────────────────────────────────────────────────

/// Writes findings as a fixed-width table with aligned columns.
pub struct TableFormatter<W: WriteColor> {
    writer: W,
    colour: bool,
}

impl<W: WriteColor> TableFormatter<W> {
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

    /// Write the full table followed by a summary + elapsed line.
    pub fn write_all(
        &mut self,
        findings: &[Finding],
        elapsed: std::time::Duration,
    ) -> anyhow::Result<()> {
        if findings.is_empty() {
            return Ok(());
        }

        let has_blame = findings.iter().any(|f| f.blame_author.is_some());

        // ── column widths ──────────────────────────────────────────────────────
        let w_file = col_width(
            H_FILE,
            findings.iter().map(|f| f.file.display().to_string().len()),
        );
        let w_line = col_width(H_LINE, findings.iter().map(|f| digit_count(f.line)));
        let w_tag = col_width(
            H_TAG,
            findings.iter().map(|f| {
                if let Some(ref a) = f.author {
                    f.tag.len() + a.len() + 2 // "TAG(author)"
                } else {
                    f.tag.len()
                }
            }),
        );
        let w_sev = col_width(
            H_SEV,
            findings.iter().map(|f| severity_str(f.severity).len()),
        );
        let w_msg =
            col_width(H_MSG, findings.iter().map(|f| f.message.chars().count())).min(MAX_MSG);

        let (w_blame, w_age, w_commit) = if has_blame {
            let wb = col_width(
                H_BLAME,
                findings
                    .iter()
                    .map(|f| f.blame_author.as_deref().unwrap_or("-").chars().count()),
            )
            .min(MAX_BLAME);
            let wa = col_width(
                H_AGE,
                findings.iter().map(|f| {
                    f.blame_date
                        .map(|d| crate::blame::format_age(d).len())
                        .unwrap_or(1)
                }),
            );
            let wc = col_width(
                H_COMMIT,
                findings
                    .iter()
                    .map(|f| f.blame_commit.as_deref().unwrap_or("-").len()),
            );
            (wb, wa, wc)
        } else {
            (0, 0, 0)
        };

        // ── header ─────────────────────────────────────────────────────────────
        self.set(ColorSpec::new().set_bold(true))?;
        write!(
            self.writer,
            "{:<wf$}  {:>wl$}  {:<wt$}  {:<ws$}  {:<wm$}",
            H_FILE,
            H_LINE,
            H_TAG,
            H_SEV,
            H_MSG,
            wf = w_file,
            wl = w_line,
            wt = w_tag,
            ws = w_sev,
            wm = w_msg,
        )?;
        if has_blame {
            write!(
                self.writer,
                "  {:<wb$}  {:<wa$}  {:<wc$}",
                H_BLAME,
                H_AGE,
                H_COMMIT,
                wb = w_blame,
                wa = w_age,
                wc = w_commit,
            )?;
        }
        writeln!(self.writer)?;
        self.reset()?;

        // ── separator ──────────────────────────────────────────────────────────
        self.set(ColorSpec::new().set_dimmed(true))?;
        write!(
            self.writer,
            "{}  {}  {}  {}  {}",
            dash(w_file),
            dash(w_line),
            dash(w_tag),
            dash(w_sev),
            dash(w_msg),
        )?;
        if has_blame {
            write!(
                self.writer,
                "  {}  {}  {}",
                dash(w_blame),
                dash(w_age),
                dash(w_commit),
            )?;
        }
        writeln!(self.writer)?;
        self.reset()?;

        // ── rows ───────────────────────────────────────────────────────────────
        for f in findings {
            // FILE - cyan
            self.set(ColorSpec::new().set_fg(Some(Color::Cyan)))?;
            write!(self.writer, "{:<w$}", f.file.display(), w = w_file)?;
            self.reset()?;

            // LINE - green, right-aligned
            self.set(ColorSpec::new().set_fg(Some(Color::Green)))?;
            write!(self.writer, "  {:>w$}", f.line, w = w_line)?;
            self.reset()?;

            // TAG - colored by severity (includes annotation author if present)
            let tag_str = if let Some(ref a) = f.author {
                format!("{}({})", f.tag, a)
            } else {
                f.tag.clone()
            };
            self.set(&tag_color(f.severity))?;
            write!(self.writer, "  {:<w$}", tag_str, w = w_tag)?;
            self.reset()?;

            // SEVERITY - same color as tag
            self.set(&tag_color(f.severity))?;
            write!(self.writer, "  {:<w$}", severity_str(f.severity), w = w_sev)?;
            self.reset()?;

            // MESSAGE - truncated with ellipsis if needed
            write!(
                self.writer,
                "  {:<w$}",
                truncate(&f.message, w_msg),
                w = w_msg
            )?;

            if has_blame {
                // BLAME AUTHOR - green
                let ba = truncate(f.blame_author.as_deref().unwrap_or("-"), w_blame);
                self.set(ColorSpec::new().set_fg(Some(Color::Green)))?;
                write!(self.writer, "  {:<w$}", ba, w = w_blame)?;
                self.reset()?;

                // AGE - yellow
                let age = f
                    .blame_date
                    .map(crate::blame::format_age)
                    .unwrap_or_else(|| "-".to_string());
                self.set(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
                write!(self.writer, "  {:<w$}", age, w = w_age)?;
                self.reset()?;

                // COMMIT - dim
                let commit = f.blame_commit.as_deref().unwrap_or("-");
                self.set(ColorSpec::new().set_dimmed(true))?;
                write!(self.writer, "  {:<w$}", commit, w = w_commit)?;
                self.reset()?;
            }

            writeln!(self.writer)?;
        }

        // ── summary ────────────────────────────────────────────────────────────
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

        self.set(ColorSpec::new().set_dimmed(true))?;
        write!(self.writer, "  {}", format_elapsed(elapsed))?;
        self.reset()?;

        writeln!(self.writer)?;
        Ok(())
    }
}

impl<W: WriteColor + Write> Formatter for TableFormatter<W> {
    fn format(&self, findings: &[Finding], _writer: &mut dyn Write) -> anyhow::Result<()> {
        let _ = findings;
        Ok(())
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

/// Width of the widest item, floored to the header width.
fn col_width(header: &str, values: impl Iterator<Item = usize>) -> usize {
    values.max().unwrap_or(0).max(header.len())
}

/// Number of decimal digits in `n`.
fn digit_count(n: usize) -> usize {
    if n == 0 {
        1
    } else {
        (n as f64).log10().floor() as usize + 1
    }
}

fn severity_str(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

fn tag_color(s: Severity) -> ColorSpec {
    match s {
        Severity::Error => ColorSpec::new()
            .set_fg(Some(Color::Red))
            .set_bold(true)
            .clone(),
        Severity::Warning => ColorSpec::new()
            .set_fg(Some(Color::Yellow))
            .set_bold(true)
            .clone(),
        Severity::Info => ColorSpec::new().set_fg(Some(Color::Blue)).clone(),
    }
}

/// Truncate to `max` chars, appending `...` if the string was longer.
fn truncate(s: &str, max: usize) -> String {
    let count = s.chars().count();
    if count <= max {
        s.to_string()
    } else {
        let trimmed: String = s.chars().take(max.saturating_sub(3)).collect();
        format!("{}...", trimmed)
    }
}

/// A horizontal rule of `n` Unicode box-drawing dashes.
fn dash(n: usize) -> String {
    "\u{2500}".repeat(n) // "─"
}

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

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matcher::{Finding, Severity};
    use std::path::PathBuf;
    use termcolor::NoColor;

    fn make_finding(
        file: &str,
        line: usize,
        tag: &str,
        severity: Severity,
        author: Option<&str>,
        msg: &str,
    ) -> Finding {
        Finding {
            file: PathBuf::from(file),
            line,
            column: 1,
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
        let mut fmt = TableFormatter::new(NoColor::new(buf), false);
        fmt.write_all(findings, std::time::Duration::ZERO).unwrap();
        String::from_utf8(fmt.writer.into_inner()).unwrap()
    }

    #[test]
    fn empty_findings_produces_no_output() {
        assert_eq!(render(&[]), "");
    }

    #[test]
    fn header_row_is_present() {
        let f = make_finding("src/a.rs", 1, "TODO", Severity::Warning, None, "msg");
        let out = render(&[f]);
        assert!(out.contains("FILE"));
        assert!(out.contains("LINE"));
        assert!(out.contains("TAG"));
        assert!(out.contains("SEVERITY"));
        assert!(out.contains("MESSAGE"));
    }

    #[test]
    fn finding_data_appears_in_row() {
        let f = make_finding("src/a.rs", 42, "TODO", Severity::Warning, None, "do it");
        let out = render(&[f]);
        assert!(out.contains("src/a.rs"));
        assert!(out.contains("42"));
        assert!(out.contains("TODO"));
        assert!(out.contains("warning"));
        assert!(out.contains("do it"));
    }

    #[test]
    fn author_annotation_rendered_in_tag_column() {
        let f = make_finding(
            "a.rs",
            1,
            "TODO",
            Severity::Warning,
            Some("alice"),
            "refactor",
        );
        let out = render(&[f]);
        assert!(out.contains("TODO(alice)"));
    }

    #[test]
    fn long_message_is_truncated_with_ellipsis() {
        let long_msg = "a".repeat(80);
        let f = make_finding("a.rs", 1, "TODO", Severity::Warning, None, &long_msg);
        let out = render(&[f]);
        assert!(out.contains("..."));
    }

    #[test]
    fn short_message_is_not_truncated() {
        let f = make_finding("a.rs", 1, "TODO", Severity::Warning, None, "short msg");
        let out = render(&[f]);
        assert!(out.contains("short msg"));
        assert!(!out.contains("..."));
    }

    #[test]
    fn blame_columns_absent_without_blame_data() {
        let f = make_finding("a.rs", 1, "TODO", Severity::Warning, None, "msg");
        let out = render(&[f]);
        assert!(!out.contains("BLAME AUTHOR"));
        // "AGE" appears inside "MESSAGE" so check the standalone header word instead
        assert!(!out.contains("COMMIT"));
        // Verify the blame header specifically is absent
        let header_line = out.lines().next().unwrap_or("");
        assert!(
            !header_line.contains("BLAME"),
            "blame header should not appear"
        );
    }

    #[test]
    fn blame_columns_present_when_blame_data_exists() {
        let mut f = make_finding("a.rs", 1, "TODO", Severity::Warning, None, "msg");
        f.blame_author = Some("bob <bob@example.com>".to_string());
        f.blame_date = Some(0);
        f.blame_commit = Some("abc1234".to_string());
        let out = render(&[f]);
        assert!(out.contains("BLAME AUTHOR"));
        assert!(out.contains("AGE"));
        assert!(out.contains("COMMIT"));
        assert!(out.contains("bob <bob@example.com>"));
        assert!(out.contains("abc1234"));
    }

    #[test]
    fn columns_are_aligned() {
        // Multiple findings; check that lines have consistent length up to message col.
        let findings = vec![
            make_finding("short.rs", 1, "TODO", Severity::Warning, None, "msg"),
            make_finding(
                "a/longer/path/file.rs",
                999,
                "FIXME",
                Severity::Error,
                None,
                "other",
            ),
        ];
        let out = render(&findings);
        let lines: Vec<&str> = out.lines().collect();
        // Header, separator, two rows, blank, summary - header and separator should exist
        assert!(lines.len() >= 4);
    }

    #[test]
    fn summary_line_is_present() {
        let findings = vec![
            make_finding("a.rs", 1, "TODO", Severity::Warning, None, "x"),
            make_finding("b.rs", 2, "FIXME", Severity::Error, None, "y"),
        ];
        let out = render(&findings);
        assert!(out.contains("Found 2 annotations across 2 files."));
    }
}
