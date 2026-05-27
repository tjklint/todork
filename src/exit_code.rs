/// Process exit codes for todork.
///
/// - `Success` (0): scan completed and at least one annotation was found.
/// - `NotFound` (1): scan completed but no annotations were found.
/// - `Error` (2): a fatal error occurred (bad arguments, I/O failure, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ExitCode {
    /// Annotations were found.
    Success = 0,
    /// No annotations were found.
    NotFound = 1,
    /// A fatal error occurred.
    Error = 2,
}

impl From<ExitCode> for std::process::ExitCode {
    fn from(code: ExitCode) -> Self {
        std::process::ExitCode::from(code as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_code_values() {
        assert_eq!(ExitCode::Success as i32, 0);
        assert_eq!(ExitCode::NotFound as i32, 1);
        assert_eq!(ExitCode::Error as i32, 2);
    }

    #[test]
    fn exit_code_into_process_exit_code() {
        let _: std::process::ExitCode = ExitCode::Success.into();
        let _: std::process::ExitCode = ExitCode::NotFound.into();
        let _: std::process::ExitCode = ExitCode::Error.into();
    }
}
