use clap::Parser;
use todork::cli::{Args, Command};
use todork::config::Config;
use todork::exit_code::ExitCode;

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
