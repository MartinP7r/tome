//! Domain error sentinels (CORE-05 / D-14).
//!
//! [`DomainErrorKind`] is a small enum of *typed sentinels* attached to `anyhow`
//! errors at a deliberately small set of GUI-relevant failure sites via the
//! [`WithDomainKind`] extension trait. It exists so the `tome-desktop` IPC
//! boundary can classify an error into a coarse `ErrorCode` by **downcasting the
//! cause chain** (`err.chain().find_map(|c| c.downcast_ref::<DomainTagged>())`)
//! rather than string-matching the human-readable message (D-14).
//!
//! This generalizes the in-repo pattern already proven by
//! [`crate::LintFailed`]/[`crate::MigrationPartialOrFailed`], which `main.rs`
//! downcasts at the CLI exit-code boundary. Those CLI markers are **unchanged**
//! and keep mapping to exit codes; `DomainErrorKind` is the GUI-boundary half of
//! the same idea ("structure at the edge", D-17 — symmetric with `ProgressSink`).
//!
//! ## Why a [`DomainTagged`] wrapper, not `.with_context(|| DomainErrorKind::X)`
//!
//! The RESEARCH Code Example (lines 401-406) proposed
//! `.with_context(|| DomainErrorKind::NotFound)`. **That does not work** — when a
//! type is used as an anyhow *context value*, anyhow wraps it as a Display-only
//! layer that is **not** recoverable via `downcast_ref::<DomainErrorKind>()`
//! through the cause chain (verified empirically: `chain().find_map(...)` returns
//! `None`). [Rule 1 — Bug] in the documented pattern.
//!
//! The working idiom is a concrete wrapper error ([`DomainTagged`]) attached via
//! [`anyhow::Error::new`]. The wrapper is transparent on purpose:
//! - its `Display` delegates to the **top-level message** of the error it wraps,
//!   and
//! - its [`Error::source`] skips that already-printed top link and exposes the
//!   underlying error's *cause*,
//!
//! so the `{e:#}` chain reads **byte-for-byte identical** to the un-tagged error
//! (no CLI regression — verified for both single-message and multi-link chains),
//! while `chain().find_map(|c| c.downcast_ref::<DomainTagged>())` recovers the
//! typed [`DomainErrorKind`].
//!
//! There is deliberately **no `Internal` variant** — `Internal` is the boundary's
//! fallback (`tome-desktop`'s `ErrorCode::Internal`) for any error with no
//! sentinel in its chain. Keeping `Internal` out of the domain enum means a
//! domain author can only ever attach a *meaningful* classification.

use std::fmt;

/// Typed, downcastable error-classification sentinel kinds for the GUI boundary.
///
/// Attach via [`WithDomainKind::with_domain_kind`] at GUI-relevant failure
/// sites. The `tome-desktop` boundary maps each variant to a coarse `ErrorCode`
/// (`Validation`, `NotFound`, `Permission`, `Conflict`, `Git`, `Io`); anything
/// unclassified falls to `Internal` at the boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DomainErrorKind {
    /// A config/role/type or other input-validation failure
    /// (e.g. `Config::validate` rejecting an invalid role/type combo).
    #[error("validation failed")]
    Validation,
    /// A required path/config/directory was not found
    /// (e.g. an explicit `--config` path whose parent does not exist).
    #[error("not found")]
    NotFound,
    /// A filesystem operation was denied by the OS (`EACCES`/`EPERM`).
    #[error("permission denied")]
    Permission,
    /// A content-hash or path-overlap collision
    /// (e.g. `library_dir` overlapping a distribution directory).
    #[error("conflict")]
    Conflict,
    /// A `git` clone/update operation failed.
    #[error("git operation failed")]
    Git,
    /// A generic I/O failure that is neither a not-found nor a permission case.
    #[error("io failure")]
    Io,
}

/// A transparent wrapper error that tags an `anyhow::Error` with a
/// [`DomainErrorKind`] while preserving the original human-readable chain.
///
/// See the module docs for why this exists instead of `.with_context(|| kind)`.
/// The wrapper is downcastable: the GUI boundary does
/// `err.chain().find_map(|c| c.downcast_ref::<DomainTagged>())` and reads
/// [`DomainTagged::kind`].
#[derive(Debug)]
pub struct DomainTagged {
    /// The classification this error site carries.
    pub kind: DomainErrorKind,
    /// The original error, untouched.
    source: anyhow::Error,
}

impl fmt::Display for DomainTagged {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Delegate to the underlying error's TOP-LEVEL message only (not its
        // whole chain). Combined with `source()` skipping that same top link,
        // this makes the `{:#}` chain read identically to the un-tagged error.
        write!(f, "{}", self.source)
    }
}

impl std::error::Error for DomainTagged {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // Skip the underlying's top link (already printed by our `Display`) and
        // expose its cause, so anyhow's `{:#}` chain has no duplicated link.
        self.source.source()
    }
}

/// Attach a [`DomainErrorKind`] sentinel to a fallible result without changing
/// the human-readable error chain.
///
/// Use at GUI-relevant failure sites:
/// ```ignore
/// config.validate().with_domain_kind(DomainErrorKind::Validation)?;
/// ```
pub trait WithDomainKind<T> {
    /// Tag the `Err` value with `kind` (no-op on `Ok`).
    fn with_domain_kind(self, kind: DomainErrorKind) -> anyhow::Result<T>;
}

impl<T> WithDomainKind<T> for anyhow::Result<T> {
    fn with_domain_kind(self, kind: DomainErrorKind) -> anyhow::Result<T> {
        self.map_err(|source| anyhow::Error::new(DomainTagged { kind, source }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;

    #[test]
    fn sentinel_downcasts_through_layered_context() {
        // The whole point of D-14: a sentinel attached deep in the chain must be
        // recoverable even when further human-readable `.context()` strings are
        // layered on top of it. Mirrors `lint_failed_downcast_through_anyhow`.
        let base: anyhow::Result<()> = Err(anyhow::anyhow!("disk read failed"));
        let err = base
            .with_domain_kind(DomainErrorKind::NotFound)
            .context("while loading directory 'foo'")
            .context("while gathering status")
            .unwrap_err();

        let recovered = err
            .chain()
            .find_map(|cause| cause.downcast_ref::<DomainTagged>())
            .map(|t| t.kind);

        assert_eq!(
            recovered,
            Some(DomainErrorKind::NotFound),
            "sentinel must survive layered .context() and downcast via chain()"
        );
    }

    #[test]
    fn no_sentinel_yields_no_tag() {
        // An untagged error has no DomainTagged anywhere in its chain — the
        // boundary will fall back to Internal.
        let err = Err::<(), _>(anyhow::anyhow!("root"))
            .context("layer")
            .unwrap_err();
        assert!(
            err.chain()
                .find_map(|c| c.downcast_ref::<DomainTagged>())
                .is_none(),
            "untagged error must carry no DomainTagged"
        );
    }

    #[test]
    fn sentinel_does_not_change_human_readable_chain_single_message() {
        // No CLI regression: tagging leaves the `{:#}` chain byte-for-byte the
        // same for a typical single-message (`bail!`) underlying error.
        let underlying = anyhow::anyhow!("library_dir overlaps directory 'work'");
        let untagged = format!("{underlying:#}");

        let tagged = Err::<(), _>(anyhow::anyhow!("library_dir overlaps directory 'work'"))
            .with_domain_kind(DomainErrorKind::Conflict)
            .unwrap_err();

        assert_eq!(
            format!("{tagged:#}"),
            untagged,
            "tagging a single-message error must not change its {{:#}} rendering"
        );
        // ...and the classifying substring survives for `assert_cmd` contains() tests.
        assert!(format!("{tagged:#}").contains("overlaps"));
    }

    #[test]
    fn sentinel_does_not_change_human_readable_chain_multi_link() {
        // Same invariant for an underlying error that already had its own
        // `.context()` layers — the skip-the-top-link `source()` keeps the chain
        // free of duplicates.
        let make = || -> anyhow::Result<()> {
            Err(anyhow::anyhow!("disk error")).context("failed to read dir 'foo'")
        };
        let untagged = format!("{:#}", make().unwrap_err());

        let tagged = make().with_domain_kind(DomainErrorKind::Io).unwrap_err();

        assert_eq!(
            format!("{tagged:#}"),
            untagged,
            "tagging a multi-link error must not duplicate or drop chain links"
        );
        // The buried sentinel is still recoverable.
        assert_eq!(
            tagged
                .chain()
                .find_map(|c| c.downcast_ref::<DomainTagged>())
                .map(|t| t.kind),
            Some(DomainErrorKind::Io),
        );
    }
}
