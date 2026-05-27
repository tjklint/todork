//! JSON array output formatter.

use super::Formatter;
use crate::matcher::Finding;
use std::io::Write;

/// Writes all findings as a pretty-printed JSON array.
///
/// Each element is a serialized [`Finding`]:
/// ```json
/// [
///   {
///     "file": "src/main.rs",
///     "line": 42,
///     "column": 5,
///     "tag": "TODO",
///     "severity": "warning",
///     "author": null,
///     "message": "handle this edge case"
///   }
/// ]
/// ```
pub struct JsonFormatter;

impl Formatter for JsonFormatter {
    fn format(&self, findings: &[Finding], writer: &mut dyn Write) -> anyhow::Result<()> {
        serde_json::to_writer_pretty(writer, findings)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matcher::{Finding, Severity};
    use std::path::PathBuf;

    fn finding(tag: &str, severity: Severity, author: Option<&str>) -> Finding {
        Finding {
            file: PathBuf::from("src/a.rs"),
            line: 1,
            column: 3,
            tag: tag.to_string(),
            severity,
            author: author.map(str::to_string),
            message: "test message".to_string(),
        }
    }

    fn render(findings: &[Finding]) -> String {
        let mut buf = Vec::new();
        JsonFormatter.format(findings, &mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn empty_findings_produces_empty_array() {
        let out = render(&[]);
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
        assert!(parsed.is_empty());
    }

    #[test]
    fn output_is_valid_json() {
        let f = finding("TODO", Severity::Warning, None);
        let out = render(&[f]);
        let _: Vec<serde_json::Value> = serde_json::from_str(&out).expect("must be valid JSON");
    }

    #[test]
    fn finding_fields_serialized() {
        let f = finding("TODO", Severity::Warning, None);
        let out = render(&[f]);
        let arr: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
        let obj = &arr[0];
        assert_eq!(obj["file"], "src/a.rs");
        assert_eq!(obj["line"], 1);
        assert_eq!(obj["column"], 3);
        assert_eq!(obj["tag"], "TODO");
        assert_eq!(obj["severity"], "warning");
        assert_eq!(obj["author"], serde_json::Value::Null);
        assert_eq!(obj["message"], "test message");
    }

    #[test]
    fn severity_error_serialized_as_lowercase() {
        let f = finding("FIXME", Severity::Error, None);
        let out = render(&[f]);
        let arr: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
        assert_eq!(arr[0]["severity"], "error");
    }

    #[test]
    fn severity_info_serialized_as_lowercase() {
        let f = finding("NOTE", Severity::Info, None);
        let out = render(&[f]);
        let arr: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
        assert_eq!(arr[0]["severity"], "info");
    }

    #[test]
    fn author_serialized_when_present() {
        let f = finding("TODO", Severity::Warning, Some("alice"));
        let out = render(&[f]);
        let arr: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
        assert_eq!(arr[0]["author"], "alice");
    }

    #[test]
    fn multiple_findings_all_serialized() {
        let findings = vec![
            finding("TODO", Severity::Warning, None),
            finding("FIXME", Severity::Error, Some("bob")),
        ];
        let out = render(&findings);
        let arr: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["tag"], "TODO");
        assert_eq!(arr[1]["tag"], "FIXME");
    }

    #[test]
    fn output_is_pretty_printed() {
        let f = finding("TODO", Severity::Warning, None);
        let out = render(&[f]);
        // Pretty-printed JSON has newlines and indentation.
        assert!(out.contains('\n'));
        assert!(out.contains("  "));
    }
}
