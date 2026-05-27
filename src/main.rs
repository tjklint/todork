use clap::Parser;
use todork::cli::Args;
use todork::config::Config;
use todork::exit_code::ExitCode;

fn main() -> std::process::ExitCode {
    let args = Args::parse();
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
