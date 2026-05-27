//! Output formatting — trait definition and per-format implementations.

pub mod text;

use crate::matcher::Finding;
use std::io::Write;

/// Formats a slice of [`Finding`]s to a writer.
pub trait Formatter {
    /// Write all findings to `writer`.
    fn format(&self, findings: &[Finding], writer: &mut dyn Write) -> anyhow::Result<()>;
}
