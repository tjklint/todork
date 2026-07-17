use clap::Parser;
use todork::cli::{Args, Command};
use todork::config::Config;
use todork::exit_code::ExitCode;

// Include the generated version file so the binary is recompiled when the
// package version changes. The constant itself is not used here.
mod _version {
    include!(concat!(env!("OUT_DIR"), "/version.rs"));
}

fn main() -> std::process::ExitCode {
    let args = Args::parse();

    if let Some(Command::Upgrade) = &args.command {
        return match todork::upgrade::run() {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("todork upgrade: {e}");
                ExitCode::Error.into()
            }
        };
    }

    let config = match Config::from_args(args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("todork: {e}");
            return ExitCode::Error.into();
        }
    };

    match todork::run(config) {
        Ok(code) => code.into(),
        Err(e) => {
            eprintln!("todork: {e}");
            ExitCode::Error.into()
        }
    }
}
