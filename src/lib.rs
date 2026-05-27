pub mod cli;
pub mod config;
pub mod error;
pub mod exit_code;

use crate::config::Config;
use crate::exit_code::ExitCode;

/// Run todork with the given configuration.
///
/// Returns the appropriate [`ExitCode`]:
/// - [`ExitCode::Success`] when annotations are found.
/// - [`ExitCode::NotFound`] when no annotations are found.
/// - Propagates errors via `anyhow::Result`.
pub fn run(_config: Config) -> anyhow::Result<ExitCode> {
    // Scanning logic will be added in PR 3.
    // For now we just confirm the binary wires up correctly.
    Ok(ExitCode::NotFound)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Args;
    use clap::Parser;

    #[test]
    fn run_returns_not_found_for_stub() {
        let args = Args::parse_from(["todork"]);
        let config = Config::from_args(args).unwrap();
        let code = run(config).unwrap();
        assert_eq!(code, ExitCode::NotFound);
    }
}
