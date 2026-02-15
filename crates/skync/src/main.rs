use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    let cli = skync::cli::Cli::parse();

    match skync::run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}
