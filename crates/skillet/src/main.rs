//! Thin binary entry point — parses CLI args and delegates to `skillet::run()`.

use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    let cli = skillet::cli::Cli::parse();

    match skillet::run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}
