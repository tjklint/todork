//! Tag definitions, the AhoCorasick-based multi-pattern matcher, and the
//! [`Finding`] type that represents a single located annotation.

use aho_corasick::AhoCorasick;
use regex::Regex;
use serde::Serialize;
use std::path::PathBuf;

// ── Severity ──────────────────────────────────────────────────────────────────

/// How urgent an annotation is considered to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Blocking issues that should be fixed before shipping (FIXME, BUG, XXX).
    Error,
    /// Things that need attention but are not yet blocking (TODO, HACK, DEPRECATED).
    Warning,
    /// Informational annotations (NOTE, OPTIMIZE).
    Info,
}

// ── Tag ───────────────────────────────────────────────────────────────────────

/// A single annotation keyword with its associated severity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    /// The keyword exactly as it appears in source code (e.g. `"TODO"`).
    pub name: &'static str,
    /// Severity level used for output colouring and exit-code decisions.
    pub severity: Severity,
}

/// The default set of annotation tags recognised by todork.
pub const DEFAULT_TAGS: &[Tag] = &[
    Tag {
        name: "FIXME",
        severity: Severity::Error,
    },
    Tag {
        name: "BUG",
        severity: Severity::Error,
    },
    Tag {
        name: "XXX",
        severity: Severity::Error,
    },
    Tag {
        name: "TODO",
        severity: Severity::Warning,
    },
    Tag {
        name: "HACK",
        severity: Severity::Warning,
    },
    Tag {
        name: "DEPRECATED",
        severity: Severity::Warning,
    },
    Tag {
        name: "NOTE",
        severity: Severity::Info,
    },
    Tag {
        name: "OPTIMIZE",
        severity: Severity::Info,
    },
];

// ── Finding ───────────────────────────────────────────────────────────────────

/// A single located annotation in a source file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    /// Path to the file that contains the annotation.
    pub file: PathBuf,
    /// 1-based line number.
    pub line: usize,
    /// 1-based byte column of the tag's first character.
    pub column: usize,
    /// The matched tag keyword (e.g. `"TODO"`).
    pub tag: String,
    /// Severity inherited from the matched [`Tag`].
    pub severity: Severity,
    /// Optional author extracted from `TAG(author):` syntax.
    pub author: Option<String>,
    /// The text that follows the tag marker on the same line.
    pub message: String,
}

// ── Matcher ───────────────────────────────────────────────────────────────────

/// Pre-compiled multi-pattern scanner.
///
/// Build once with [`Matcher::new`], then call [`Matcher::scan_line`] for each
/// line of a file.  The inner [`AhoCorasick`] automaton scans all tag keywords
/// simultaneously in a single O(n) pass.
pub struct Matcher {
    /// Compiled multi-pattern automaton.
    ac: AhoCorasick,
    /// The tag slice in the same order as the patterns given to `AhoCorasick`.
    tags: Vec<Tag>,
    /// Extracts `(author, message)` from the text starting at a tag hit.
    ///
    /// Pattern: `TAG\s*(?:\(([^)]*)\))?\s*:?\s*(.*)`
    detail_re: Regex,
}

impl Matcher {
    /// Build a new `Matcher` from a slice of tags.
    ///
    /// # Errors
    /// Returns an error if the AhoCorasick automaton or the detail regex
    /// cannot be compiled (should not happen with well-formed tag names).
    pub fn new(tags: &[Tag]) -> anyhow::Result<Self> {
        let patterns: Vec<&str> = tags.iter().map(|t| t.name).collect();
        let ac = AhoCorasick::builder()
            .match_kind(aho_corasick::MatchKind::LeftmostFirst)
            .build(&patterns)?;

        // Matches a tag keyword followed by optional (author) and message.
        // We anchor to the beginning of the slice we pass in, so we only need
        // to match the first thing we see.
        let detail_re = Regex::new(r"(?:[A-Z]+)\s*(?:\(([^)]*)\))?\s*:?\s*(.*)")?;

        Ok(Self {
            ac,
            tags: tags.to_vec(),
            detail_re,
        })
    }

    /// Scan a single `line` (bytes, **without** the trailing newline) and
    /// return all findings it contains.
    ///
    /// `file` and `line_number` are used only to populate the returned
    /// [`Finding`] structs.
    pub fn scan_line(
        &self,
        line: &str,
        file: &std::path::Path,
        line_number: usize,
    ) -> Vec<Finding> {
        let mut findings = Vec::new();

        for mat in self.ac.find_iter(line) {
            let tag = &self.tags[mat.pattern()];
            let match_start = mat.start();

            // Verify word boundary on the right: the character immediately
            // after the tag must not be an ASCII alphabetic character.
            // This prevents "FIXME" matching inside e.g. "NOTFIXME".
            // Left boundary is guaranteed by AhoCorasick starting the search
            // at a fresh offset each time, but we also check right side.
            let after = line.as_bytes().get(mat.end());
            if matches!(after, Some(b) if b.is_ascii_alphabetic() || *b == b'_') {
                continue;
            }
            // Also verify the character before is not alphabetic/underscore.
            if match_start > 0 {
                let before = line.as_bytes()[match_start - 1];
                if before.is_ascii_alphabetic() || before == b'_' {
                    continue;
                }
            }

            let (author, message) = self.extract_detail(&line[match_start..]);
            let column = match_start + 1; // 1-based

            findings.push(Finding {
                file: file.to_path_buf(),
                line: line_number,
                column,
                tag: tag.name.to_string(),
                severity: tag.severity,
                author,
                message,
            });
        }

        findings
    }

    /// Extract `(author, message)` from a slice that starts at the tag keyword.
    fn extract_detail(&self, slice: &str) -> (Option<String>, String) {
        match self.detail_re.captures(slice) {
            Some(caps) => {
                let author = caps
                    .get(1)
                    .map(|m| m.as_str().trim().to_string())
                    .filter(|s| !s.is_empty());
                let message = caps
                    .get(2)
                    .map(|m| m.as_str().trim().to_string())
                    .unwrap_or_default();
                (author, message)
            }
            None => (None, String::new()),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn matcher() -> Matcher {
        Matcher::new(DEFAULT_TAGS).unwrap()
    }

    fn p() -> &'static Path {
        Path::new("test.rs")
    }

    // ── construction ────────────────────────────────────────────────────────

    #[test]
    fn new_with_default_tags_succeeds() {
        assert!(Matcher::new(DEFAULT_TAGS).is_ok());
    }

    #[test]
    fn new_with_single_tag_succeeds() {
        let tags = [Tag {
            name: "TODO",
            severity: Severity::Warning,
        }];
        assert!(Matcher::new(&tags).is_ok());
    }

    // ── basic detection ──────────────────────────────────────────────────────

    #[test]
    fn detects_todo() {
        let m = matcher();
        let findings = m.scan_line("// TODO: do something", p(), 1);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].tag, "TODO");
    }

    #[test]
    fn detects_fixme() {
        let m = matcher();
        let findings = m.scan_line("// FIXME: crashes here", p(), 1);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].tag, "FIXME");
    }

    #[test]
    fn detects_hack() {
        let m = matcher();
        let findings = m.scan_line("# HACK: workaround", p(), 1);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].tag, "HACK");
    }

    #[test]
    fn detects_xxx() {
        let m = matcher();
        let findings = m.scan_line("// XXX: unsafe cast", p(), 1);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].tag, "XXX");
    }

    #[test]
    fn detects_note() {
        let m = matcher();
        let findings = m.scan_line("// NOTE: assumes UTC", p(), 1);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].tag, "NOTE");
    }

    #[test]
    fn detects_bug() {
        let m = matcher();
        let findings = m.scan_line("// BUG: off by one", p(), 1);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].tag, "BUG");
    }

    #[test]
    fn detects_optimize() {
        let m = matcher();
        let findings = m.scan_line("// OPTIMIZE: cache this", p(), 1);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].tag, "OPTIMIZE");
    }

    #[test]
    fn detects_deprecated() {
        let m = matcher();
        let findings = m.scan_line("// DEPRECATED: use new_fn()", p(), 1);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].tag, "DEPRECATED");
    }

    // ── author extraction ────────────────────────────────────────────────────

    #[test]
    fn extracts_author() {
        let m = matcher();
        let findings = m.scan_line("// TODO(alice): refactor this", p(), 1);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].author.as_deref(), Some("alice"));
        assert_eq!(findings[0].message, "refactor this");
    }

    #[test]
    fn no_author_when_no_parens() {
        let m = matcher();
        let findings = m.scan_line("// TODO: plain message", p(), 1);
        assert_eq!(findings[0].author, None);
        assert_eq!(findings[0].message, "plain message");
    }

    #[test]
    fn author_with_spaces_trimmed() {
        let m = matcher();
        let findings = m.scan_line("// HACK( bob ): monkey patch", p(), 1);
        assert_eq!(findings[0].author.as_deref(), Some("bob"));
    }

    // ── message extraction ───────────────────────────────────────────────────

    #[test]
    fn message_extracted_without_colon() {
        let m = matcher();
        // No colon after tag — message still extracted
        let findings = m.scan_line("// TODO do the thing", p(), 1);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].message, "do the thing");
    }

    #[test]
    fn message_is_empty_when_no_text_follows() {
        let m = matcher();
        let findings = m.scan_line("// TODO", p(), 1);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].message, "");
    }

    #[test]
    fn message_trimmed() {
        let m = matcher();
        let findings = m.scan_line("// TODO:   lots of spaces   ", p(), 1);
        assert_eq!(findings[0].message, "lots of spaces");
    }

    // ── line / column numbers ────────────────────────────────────────────────

    #[test]
    fn line_number_preserved() {
        let m = matcher();
        let findings = m.scan_line("// TODO: test", p(), 42);
        assert_eq!(findings[0].line, 42);
    }

    #[test]
    fn column_is_one_based() {
        let m = matcher();
        // Tag starts at byte 0 → column 1
        let findings = m.scan_line("TODO: at start", p(), 1);
        assert_eq!(findings[0].column, 1);
    }

    #[test]
    fn column_accounts_for_prefix() {
        let m = matcher();
        // "// TODO" — T is at byte offset 3 → column 4
        let findings = m.scan_line("// TODO: indented", p(), 1);
        assert_eq!(findings[0].column, 4);
    }

    // ── word boundary enforcement ────────────────────────────────────────────

    #[test]
    fn does_not_match_debug_as_bug() {
        let m = matcher();
        // "DEBUG" contains "BUG" as a substring — must not fire
        let findings = m.scan_line("let debug = true; // DEBUG mode", p(), 1);
        assert!(
            findings.iter().all(|f| f.tag != "BUG"),
            "BUG should not match inside DEBUG"
        );
    }

    #[test]
    fn does_not_match_notable_as_note() {
        let m = matcher();
        let findings = m.scan_line("let notable = true;", p(), 1);
        assert!(findings.iter().all(|f| f.tag != "NOTE"));
    }

    #[test]
    fn does_not_match_prefix_word() {
        let m = matcher();
        // "SOMETODO" — TODO at end of word, should not match
        let findings = m.scan_line("let SOMETODO = 1;", p(), 1);
        assert!(findings.iter().all(|f| f.tag != "TODO"));
    }

    // ── multiple findings per line ───────────────────────────────────────────

    #[test]
    fn multiple_tags_on_same_line() {
        let m = matcher();
        // Unusual but valid: two tags on one line
        let findings = m.scan_line("// TODO: first  FIXME: second", p(), 1);
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].tag, "TODO");
        assert_eq!(findings[1].tag, "FIXME");
    }

    // ── severity ─────────────────────────────────────────────────────────────

    #[test]
    fn todo_has_warning_severity() {
        let m = matcher();
        let findings = m.scan_line("// TODO: x", p(), 1);
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn fixme_has_error_severity() {
        let m = matcher();
        let findings = m.scan_line("// FIXME: x", p(), 1);
        assert_eq!(findings[0].severity, Severity::Error);
    }

    #[test]
    fn note_has_info_severity() {
        let m = matcher();
        let findings = m.scan_line("// NOTE: x", p(), 1);
        assert_eq!(findings[0].severity, Severity::Info);
    }

    // ── block comment / multiline style ─────────────────────────────────────

    #[test]
    fn detects_todo_in_block_comment() {
        let m = matcher();
        let findings = m.scan_line(" * TODO: inside a block comment", p(), 1);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].tag, "TODO");
    }

    // ── empty / trivial input ────────────────────────────────────────────────

    #[test]
    fn empty_line_returns_no_findings() {
        let m = matcher();
        assert!(m.scan_line("", p(), 1).is_empty());
    }

    #[test]
    fn line_with_no_tags_returns_empty() {
        let m = matcher();
        assert!(m
            .scan_line("fn main() { println!(\"hello\"); }", p(), 1)
            .is_empty());
    }

    // ── file path preserved ──────────────────────────────────────────────────

    #[test]
    fn file_path_preserved_in_finding() {
        let m = matcher();
        let path = Path::new("src/foo.rs");
        let findings = m.scan_line("// TODO: check", path, 5);
        assert_eq!(findings[0].file, path);
    }
}
