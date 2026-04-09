//! Thin binary entry point — parses CLI args and delegates to `tome::run()`.

use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    // Handle Ctrl-C gracefully: print a clean message and exit.
    // Tome's sync pipeline is crash-safe (atomic writes, idempotent manifest),
    // so no cleanup is needed — just exit cleanly.
    if let Err(e) = ctrlc::set_handler(|| {
        // Second Ctrl-C: force quit immediately (default behavior restored)
        eprintln!("\ninterrupted — run `tome sync` to resume");
        std::process::exit(130); // 128 + SIGINT(2)
    }) {
        eprintln!("warning: could not set signal handler: {e}");
    }

    let cli = tome::cli::Cli::parse();

    match tome::run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}
