//! Thin binary entry point — parses CLI args and delegates to `tome::run()`.

use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    // Handle Ctrl-C gracefully: print a clean message and exit.
    // Tome's sync pipeline is crash-safe (atomic writes, idempotent manifest),
    // so no cleanup is needed — just exit cleanly.
    if let Err(e) = ctrlc::set_handler(|| {
        eprintln!("\ninterrupted — run `tome sync` to resume");
        std::process::exit(130); // 128 + SIGINT(2)
    }) {
        eprintln!("warning: could not set signal handler: {e}");
    }

    let cli = tome::cli::Cli::parse();

    match tome::run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            // HARD-04: typed exit-code mapping. Both branches currently emit
            // ExitCode::FAILURE (1), but the downcast lets future Phase 16/17
            // work differentiate exit codes per error class without churning
            // every site.
            if let Some(lint_failed) = e.downcast_ref::<tome::LintFailed>() {
                eprintln!("error: {lint_failed}");
                return ExitCode::FAILURE;
            }
            if let Some(migration_failed) = e.downcast_ref::<tome::MigrationPartialOrFailed>() {
                eprintln!("error: {migration_failed}");
                return ExitCode::FAILURE;
            }
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}
