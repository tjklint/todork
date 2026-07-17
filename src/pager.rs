//! Auto-pager support for large human-readable output.

use std::io::Write;
use std::process::{Command, Stdio};

/// Spawn `less -R` and feed it `output`.
///
/// Returns an error if `less` is not available or the pipe fails.
/// The caller is responsible for ensuring this is only invoked in an
/// interactive terminal context.
pub fn page_output(output: &[u8]) -> anyhow::Result<()> {
    let mut child = Command::new("less")
        .arg("-R")
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to spawn `less`: {e}"))?;

    let mut stdin = child.stdin.take().expect("piped stdin");
    stdin.write_all(output)?;
    stdin.flush()?;
    drop(stdin);

    child.wait()?;
    Ok(())
}
