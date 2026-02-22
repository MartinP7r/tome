//! Thin binary entry point — parses CLI args and delegates to `tome::run()`.

use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    let cli = tome::cli::Cli::parse();

    match tome::run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}
