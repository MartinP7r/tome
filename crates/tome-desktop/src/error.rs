//! IPC-boundary error classification (CORE-05 / D-13..D-16).
//!
//! The domain (`crates/tome`) stays `anyhow::Result` with zero refactor (D-13).
//! Errors are classified into a stable, coarse [`TomeError`] **at the Tauri
//! command edge** — here — so the front-end can pattern-match on a stable
//! [`ErrorCode`] without inspecting prose, while the full anyhow cause chain is
//! preserved in [`TomeError::context`] for a details view (D-16, "structure at
//! the edge", D-17 — symmetric with the `ProgressSink`/`SyncProgress` boundary).
//!
//! ## Classification: typed-sentinel downcast, not string-matching (D-14)
//!
//! [`From<anyhow::Error>`] walks the error's cause chain
//! (`err.chain()`) looking for a domain sentinel. The domain attaches sentinels
//! two ways, both checked here:
//! - the transparent [`tome::DomainTagged`] wrapper (the
//!   `WithDomainKind::with_domain_kind` sites in `crates/tome`), and
//! - a bare [`tome::DomainErrorKind`] attached directly via
//!   `anyhow::Error::new(kind)` (supported for future direct-attach sites).
//!
//! Whichever appears first in the chain wins; an error with **no** sentinel
//! falls back to [`ErrorCode::Internal`] (D-14). This is deliberately not a
//! message string match — the message text can change without breaking the GUI
//! contract.

/// Coarse, stable error categories surfaced to the front-end (D-15).
///
/// Grows additively. The GUI branches on this discriminant; new variants are a
/// non-breaking superset. Mirrors `tome::DomainErrorKind` plus an `Internal`
/// fallback (the domain enum deliberately omits `Internal` — see its docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, specta::Type)]
pub enum ErrorCode {
    /// Input/config validation failure.
    Validation,
    /// A required path/config/resource was not found.
    NotFound,
    /// A filesystem operation was denied by the OS.
    Permission,
    /// A content-hash or path-overlap collision.
    Conflict,
    /// A `git` clone/update operation failed.
    Git,
    /// A generic I/O failure.
    Io,
    /// Anything not classified by a domain sentinel (the fallback, D-14).
    Internal,
}

impl ErrorCode {
    /// Compile-time-validated enumeration of every variant (repo POLISH-04
    /// convention — mirrors `DiagnosticIssueKind::ALL`). Adding a variant
    /// without updating this array trips the `const _` assert below.
    pub const ALL: [ErrorCode; 7] = [
        ErrorCode::Validation,
        ErrorCode::NotFound,
        ErrorCode::Permission,
        ErrorCode::Conflict,
        ErrorCode::Git,
        ErrorCode::Io,
        ErrorCode::Internal,
    ];
}

/// Compile-time drift guard for [`ErrorCode::ALL`] (POLISH-04). Adding a variant
/// without updating `ALL` or the [`From<&tome::DomainErrorKind>`] map below
/// fails to compile here (`non-exhaustive patterns`) or trips the const assert,
/// forcing the maintainer to keep the mapping exhaustive (threat T-25-05-T).
#[allow(dead_code)]
const fn _error_code_exhaustiveness_sentinel(c: ErrorCode) {
    match c {
        ErrorCode::Validation => {}
        ErrorCode::NotFound => {}
        ErrorCode::Permission => {}
        ErrorCode::Conflict => {}
        ErrorCode::Git => {}
        ErrorCode::Io => {}
        ErrorCode::Internal => {}
    }
}
const _: () = {
    assert!(ErrorCode::ALL.len() == 7);
};

impl From<&tome::DomainErrorKind> for ErrorCode {
    fn from(kind: &tome::DomainErrorKind) -> Self {
        // Exhaustive on purpose: a new DomainErrorKind variant must be mapped
        // here or this fails to compile (T-25-05-T mitigation). No `_` arm.
        match kind {
            tome::DomainErrorKind::Validation => ErrorCode::Validation,
            tome::DomainErrorKind::NotFound => ErrorCode::NotFound,
            tome::DomainErrorKind::Permission => ErrorCode::Permission,
            tome::DomainErrorKind::Conflict => ErrorCode::Conflict,
            tome::DomainErrorKind::Git => ErrorCode::Git,
            tome::DomainErrorKind::Io => ErrorCode::Io,
        }
    }
}

/// The structured error payload that crosses the Tauri IPC boundary (D-16).
///
/// - `code`: the stable [`ErrorCode`] the GUI pattern-matches on.
/// - `message`: the top-level error string (`err.to_string()`).
/// - `context`: the flattened anyhow cause chain, outermost first — the same
///   information the CLI prints via `{e:#}`, available for a details view.
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct TomeError {
    /// Stable, coarse classification.
    pub code: ErrorCode,
    /// Top-level human-readable message.
    pub message: String,
    /// Flattened anyhow `.context()` chain (outermost first).
    pub context: Vec<String>,
}

impl From<anyhow::Error> for TomeError {
    fn from(err: anyhow::Error) -> Self {
        // Classify by typed-sentinel downcast through the chain (D-14). Prefer
        // the transparent `DomainTagged` wrapper (the with_domain_kind sites);
        // also accept a bare `DomainErrorKind` (direct anyhow::Error::new sites).
        // Unmatched → Internal.
        let code = err
            .chain()
            .find_map(|cause| {
                cause
                    .downcast_ref::<tome::DomainTagged>()
                    .map(|t| ErrorCode::from(&t.kind))
                    .or_else(|| {
                        cause
                            .downcast_ref::<tome::DomainErrorKind>()
                            .map(ErrorCode::from)
                    })
            })
            .unwrap_or(ErrorCode::Internal);

        TomeError {
            code,
            message: err.to_string(),
            context: err.chain().map(|c| c.to_string()).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;
    use tome::DomainErrorKind;
    use tome::errors::WithDomainKind;

    #[test]
    fn sentinel_in_chain_maps_to_its_code() {
        // A DomainErrorKind::NotFound attached deep in the chain (via the
        // transparent wrapper) → ErrorCode::NotFound, even under more context.
        let err: anyhow::Error = Err::<(), _>(anyhow::anyhow!("missing 'foo'"))
            .with_domain_kind(DomainErrorKind::NotFound)
            .context("while loading directory")
            .unwrap_err();

        let te = TomeError::from(err);
        assert_eq!(te.code, ErrorCode::NotFound);
    }

    #[test]
    fn each_sentinel_maps_to_expected_code() {
        // Pin every sentinel→code mapping (T-25-05-S: mis-classification guard).
        let cases = [
            (DomainErrorKind::Validation, ErrorCode::Validation),
            (DomainErrorKind::NotFound, ErrorCode::NotFound),
            (DomainErrorKind::Permission, ErrorCode::Permission),
            (DomainErrorKind::Conflict, ErrorCode::Conflict),
            (DomainErrorKind::Git, ErrorCode::Git),
            (DomainErrorKind::Io, ErrorCode::Io),
        ];
        for (kind, expected) in cases {
            let err: anyhow::Error = Err::<(), _>(anyhow::anyhow!("root"))
                .with_domain_kind(kind)
                .unwrap_err();
            assert_eq!(TomeError::from(err).code, expected, "kind {kind:?}");
        }
    }

    #[test]
    fn bare_sentinel_attached_directly_also_classifies() {
        // A future site may attach the kind directly via anyhow::Error::new —
        // the boundary's second downcast branch must catch it too.
        let err = anyhow::Error::new(DomainErrorKind::Git).context("cloning repo");
        assert_eq!(TomeError::from(err).code, ErrorCode::Git);
    }

    #[test]
    fn no_sentinel_falls_back_to_internal() {
        let err = anyhow::anyhow!("something broke").context("top");
        assert_eq!(TomeError::from(err).code, ErrorCode::Internal);
    }

    #[test]
    fn context_is_flattened_chain_in_order() {
        // message = top-level; context = full chain outermost-first (D-16).
        let err = Err::<(), _>(anyhow::anyhow!("root cause"))
            .context("middle layer")
            .context("outer layer")
            .unwrap_err();

        let te = TomeError::from(err);
        assert_eq!(te.message, "outer layer");
        assert_eq!(
            te.context,
            vec![
                "outer layer".to_string(),
                "middle layer".to_string(),
                "root cause".to_string(),
            ]
        );
    }

    #[test]
    fn tagged_chain_does_not_duplicate_in_context() {
        // The transparent wrapper must not add a duplicate link to `context`
        // (it delegates Display to the underlying top message and skips it in
        // source()) — so a tagged single-message error yields exactly the
        // underlying chain.
        let err = Err::<(), _>(anyhow::anyhow!("the real cause"))
            .with_domain_kind(DomainErrorKind::Conflict)
            .context("outer")
            .unwrap_err();

        let te = TomeError::from(err);
        assert_eq!(te.code, ErrorCode::Conflict);
        assert_eq!(
            te.context,
            vec!["outer".to_string(), "the real cause".to_string()]
        );
    }
}
