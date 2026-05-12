//! Tracing subscriber installation. Single entry point: `install(LogLevel)`.
//!
//! Wires `tracing-subscriber` per Phase 18 decisions:
//! - Writer: stderr (D-OUT-2 — matches Unix convention + Phase 16 D-UX01-4)
//! - ANSI: gated on stderr.is_terminal() (RESEARCH Pitfall 2 — CI safety)
//! - Format: compact, no target, info-level prefix suppressed (D-OUT-4)
//! - Span events: CLOSE only — auto-emits `time.busy`/`time.idle` fields
//!   (D-SPAN-2; "elapsed_ms" in OBS-03 success criterion is conceptual —
//!   auto-emitted timing fields satisfy it; see 18-RESEARCH.md §elapsed_ms
//!   FINDING and Pitfall 5 for grep regex)
//! - Filter: TOME_LOG env wins; falls back to LogLevel-derived directive
//!   (D-ENV-1)
//! - Default level: info (D-ENV-2)
//!
//! Idempotency: `try_init` returns `Err` if a global subscriber is already
//! installed. We propagate that as `anyhow::Error`; the caller in `main.rs`
//! emits a stderr warning and continues — events drop silently rather than
//! the process aborting.

use std::io::IsTerminal;

use anyhow::Result;
use tracing_subscriber::{
    EnvFilter,
    fmt::{self, format::FmtSpan},
};

use crate::cli::LogLevel;

/// Install the global tracing subscriber. Idempotent in spirit — repeated
/// calls inside the same process return an `Err` that the caller may
/// downgrade to a non-fatal warning.
pub fn install(level: LogLevel) -> Result<()> {
    let filter = EnvFilter::try_from_env("TOME_LOG")
        .unwrap_or_else(|_| EnvFilter::new(level.directive()));

    fmt::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(std::io::stderr().is_terminal())
        .with_target(false)
        .with_span_events(FmtSpan::CLOSE)
        .compact()
        .with_env_filter(filter)
        .try_init()
        .map_err(|e| anyhow::anyhow!("tracing subscriber init failed: {e}"))?;

    Ok(())
}
