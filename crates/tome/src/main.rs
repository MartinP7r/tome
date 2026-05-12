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

    // Install tracing subscriber per Phase 18 OBS-01/OBS-02. Failure is
    // non-fatal — we fall back to no-subscriber (events drop silently) and
    // warn on stderr. The typed-error downcasts below stay on raw eprintln!
    // per D-OUT-1's "main.rs error printer stays raw" carve-out — they must
    // print even if subscriber init failed.
    if let Err(e) = tome::tracing_init::install(cli.log_level()) {
        eprintln!("warning: tracing init failed: {e:#} — continuing without structured logging");
    }

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
