//! Library manifest — tracks provenance and content hashes for each skill in the library.
//!
//! The manifest file (`.tome-manifest.json`) lives at the tome home directory (`~/.tome/`) and records which
//! directory each skill came from, its content hash, and when it was last synced. This enables idempotent
//! copy-based consolidation: unchanged skills are skipped, modified skills are re-copied.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::config::DirectoryName;
use crate::discover::SkillName;
use crate::validation::ContentHash;

pub(crate) const MANIFEST_FILENAME: &str = ".tome-manifest.json";

/// The library manifest, tracking all skills and their provenance.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Manifest {
    skills: BTreeMap<SkillName, SkillEntry>,
    /// Timestamp of last successful `tome sync` completion (post-cleanup).
    /// Stamped by `sync()` after distribute + cleanup succeed (D-LSYNC-3).
    /// `None` for pre-v0.11 manifests; renders as "never" in `tome status`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_synced_at: Option<String>,
}

impl Manifest {
    /// Returns the entry for the given skill name, if present.
    pub fn get(&self, name: &str) -> Option<&SkillEntry> {
        self.skills.get(name)
    }

    /// Returns true if the manifest contains an entry for the given skill name.
    pub fn contains_key(&self, name: &str) -> bool {
        self.skills.contains_key(name)
    }

    /// Inserts a skill entry into the manifest, keyed by the given `SkillName`.
    pub fn insert(&mut self, name: SkillName, entry: SkillEntry) {
        self.skills.insert(name, entry);
    }

    /// Removes the entry for the given skill name.
    pub fn remove(&mut self, name: &str) {
        self.skills.remove(name);
    }

    /// Returns an iterator over the skill names in the manifest.
    pub fn keys(&self) -> impl Iterator<Item = &SkillName> {
        self.skills.keys()
    }

    /// Returns an iterator over (name, entry) pairs in the manifest.
    pub fn iter(&self) -> impl Iterator<Item = (&SkillName, &SkillEntry)> {
        self.skills.iter()
    }

    /// Returns true if the manifest has no entries.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    /// Returns the number of entries in the manifest.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.skills.len()
    }

    /// Update the source_name for an existing **owned** skill entry.
    ///
    /// Returns `true` if the skill was found AND was owned (source_name = Some),
    /// `false` if missing or already Unowned. Preserves `content_hash`,
    /// `synced_at`, and other fields. Does NOT transition Unowned → Owned —
    /// callers wanting that semantic should re-insert with `SkillEntry::new`.
    ///
    /// `dead_code` allow: the only production caller (`reassign::execute`)
    /// migrated to a snapshot-based approach in HARD-19 (closes #430), but
    /// the method is preserved as a public API surface for hand-edits and
    /// is exercised by unit tests + the HARD-19 drift-test.
    #[allow(dead_code)]
    pub fn update_source_name(&mut self, skill_name: &str, new_source: &DirectoryName) -> bool {
        if let Some(entry) = self.skills.get_mut(skill_name)
            && matches!(entry.ownership, SkillOwnership::Owned { .. })
        {
            entry.ownership = SkillOwnership::Owned {
                source: new_source.clone(),
            };
            true
        } else {
            false
        }
    }

    /// Mutable access to a skill entry by name. Used by downstream code that
    /// needs to mutate an entry's fields in place (e.g. transitioning
    /// `source_name` to `None` for the Unowned state per LIB-04 / D-10
    /// trigger 2 in `cleanup::cleanup_library`).
    ///
    /// Returns `None` if no entry exists with that name. Keep the surface
    /// minimal (`pub(crate)`) — external callers should use higher-level
    /// helpers like `update_source_name`.
    pub(crate) fn skills_get_mut(&mut self, name: &str) -> Option<&mut SkillEntry> {
        self.skills.get_mut(name)
    }

    /// RFC-3339 timestamp of last successful sync; `None` for pre-v0.11
    /// manifests or before any sync has completed (D-LSYNC-2).
    pub fn last_synced_at(&self) -> Option<&str> {
        self.last_synced_at.as_deref()
    }

    /// Stamps `last_synced_at` with the current UTC time in RFC-3339 form.
    /// Called by `sync()` after distribute + cleanup succeed (D-LSYNC-3).
    /// Crate-visible only — external mutation must go through sync.
    pub(crate) fn stamp_last_synced_at(&mut self) {
        self.last_synced_at = Some(now_iso8601());
    }
}

/// Ownership state of a skill in the library (#542 / D-08).
///
/// Replaces the old flat `source_name: Option<DirectoryName>` +
/// `previous_source: Option<DirectoryName>` pair on `SkillEntry` with a
/// single, type-safe enum that makes the Owned/Unowned distinction explicit
/// and impossible to misrepresent (e.g. an Owned skill can never carry a
/// `previous_source` breadcrumb).
///
/// Serializes as a TS-friendly **tagged union** so the v1.0 GUI receives a
/// discriminated union over the IPC boundary:
/// - `{ "kind": "owned",   "source": "foo" }`
/// - `{ "kind": "unowned", "last_owner": "bar" }` (or `"last_owner": null`)
///
/// Note: this is `SkillOwnership`, **not** `SkillProvenance` — the latter
/// already exists in `discover.rs` as package-manager metadata (D-08 / Pitfall
/// 3). Do not introduce a second `SkillProvenance`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum SkillOwnership {
    /// The skill is **owned** by a directory config entry in `tome.toml`.
    Owned {
        /// Which `[directories.*]` entry contributes this skill.
        source: DirectoryName,
    },
    /// The skill is **Unowned** — its source was removed from `tome.toml`
    /// but the library copy is preserved (LIB-04). `last_owner` records the
    /// directory that owned it before the transition (D-C1); `None` for
    /// entries that became Unowned before the breadcrumb was tracked.
    Unowned {
        /// Last directory that owned this skill before the Unowned transition.
        last_owner: Option<DirectoryName>,
    },
}

/// A single skill entry in the manifest.
///
/// Deserialized via [`SkillEntryRepr`] (`#[serde(from = ...)]`) so old
/// manifests carrying the flat `source_name` / `previous_source` fields
/// migrate on read into the [`SkillOwnership`] enum. `Serialize` is derived
/// directly on this struct, emitting the new enum shape — the next `tome
/// sync` rewrites old manifests naturally. The asymmetric serde (read via
/// `SkillEntryRepr`, write via the enum) is intentional and round-trip-safe.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
#[serde(from = "SkillEntryRepr")]
pub struct SkillEntry {
    /// Where this skill was originally copied from.
    pub source_path: PathBuf,
    /// Ownership state (Owned by a directory, or Unowned with a breadcrumb).
    /// Replaces the old flat `source_name` + `previous_source` fields (#542).
    pub ownership: SkillOwnership,
    /// SHA-256 hex digest of the directory contents.
    pub content_hash: ContentHash,
    /// ISO 8601 timestamp of when this skill was last synced.
    pub synced_at: String,
    /// Whether upstream sync feeds updates into this library entry (true =
    /// managed update channel, e.g. claude plugin install/update; false =
    /// local, library is canonical). Per LIB-02, this is now an "update
    /// channel" indicator — both managed and local skills live as real
    /// directory copies in the library after Phase 11. Defaults to `false`
    /// for backwards compatibility with pre-v0.2.1 manifests.
    #[serde(default)]
    pub managed: bool,
}

/// Deserialize-only mirror that tolerates **both** the old flat `SkillEntry`
/// shape and the new enum shape, so deserialization is symmetric with the
/// `Serialize` derive (round-trip safe) **and** backward-compatible.
///
/// - Old manifests carry `source_name` (string / null / absent) and an
///   optional `previous_source` — captured with `#[serde(default)]` tolerance
///   and folded into the [`SkillOwnership`] enum by `From`.
/// - Freshly-serialized entries carry the new `ownership` enum object — when
///   present it wins outright over the legacy flat fields.
///
/// Never constructed directly — serde builds it during `SkillEntry`
/// deserialization (the `from` container attribute on `SkillEntry`).
///
/// **specta note (25-04):** `SkillEntry` carries `#[serde(from =
/// "SkillEntryRepr")]`, and specta honors serde's `from` by requiring the
/// source type to also implement `Type`. So the `specta::Type` derive is
/// mirrored here under the `bindings` feature. `SkillEntry` is not (yet)
/// reachable from any registered Tauri command/event, so this type does not
/// surface in `bindings.ts` this wave — the derive exists only to keep
/// `tome --features bindings` compiling.
#[derive(Deserialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
struct SkillEntryRepr {
    source_path: PathBuf,
    /// New shape: present in entries written after #542. Wins over the
    /// legacy flat fields when present (round-trip path).
    #[serde(default)]
    ownership: Option<SkillOwnership>,
    /// Legacy flat field (pre-#542). Read for backward-compat only.
    #[serde(default)]
    source_name: Option<DirectoryName>,
    /// Legacy flat field (pre-#542). Read for backward-compat only.
    #[serde(default)]
    previous_source: Option<DirectoryName>,
    content_hash: ContentHash,
    synced_at: String,
    #[serde(default)]
    managed: bool,
}

impl From<SkillEntryRepr> for SkillEntry {
    fn from(r: SkillEntryRepr) -> Self {
        // New enum shape wins when present (round-trip path). Otherwise fold
        // the old flat fields:
        //   source_name: Some(x)            → Owned   { source: x }
        //   source_name: None (+ prev: bar) → Unowned { last_owner: bar }
        //   source_name: None (no prev)     → Unowned { last_owner: None }
        let ownership = r.ownership.unwrap_or(match r.source_name {
            Some(source) => SkillOwnership::Owned { source },
            None => SkillOwnership::Unowned {
                last_owner: r.previous_source,
            },
        });
        SkillEntry {
            source_path: r.source_path,
            ownership,
            content_hash: r.content_hash,
            synced_at: r.synced_at,
            managed: r.managed,
        }
    }
}

impl SkillEntry {
    /// Create a new `SkillEntry` for an **owned** skill (source known).
    /// Records the current timestamp automatically.
    pub fn new(
        source_path: PathBuf,
        source_name: DirectoryName,
        content_hash: ContentHash,
        managed: bool,
    ) -> Self {
        Self {
            source_path,
            ownership: SkillOwnership::Owned {
                source: source_name,
            },
            content_hash,
            synced_at: now_iso8601(),
            managed,
        }
    }

    /// The directory that owns this skill, or `None` if the skill is Unowned.
    ///
    /// Convenience accessor mirroring the pre-#542 `source_name` field so call
    /// sites that only need the owning directory don't have to match on the
    /// [`SkillOwnership`] enum. Returns `None` for Unowned entries.
    pub fn source_name(&self) -> Option<&DirectoryName> {
        match &self.ownership {
            SkillOwnership::Owned { source } => Some(source),
            SkillOwnership::Unowned { .. } => None,
        }
    }

    /// The last directory that owned this skill before it became Unowned, or
    /// `None` if the skill is Owned (or became Unowned before the breadcrumb
    /// was tracked).
    ///
    /// Convenience accessor mirroring the pre-#542 `previous_source` field.
    pub fn previous_source(&self) -> Option<&DirectoryName> {
        match &self.ownership {
            SkillOwnership::Owned { .. } => None,
            SkillOwnership::Unowned { last_owner } => last_owner.as_ref(),
        }
    }

    /// Create a new `SkillEntry` for an **Unowned** skill — its source was
    /// removed from `tome.toml` but the library copy is preserved (per LIB-04).
    /// Records the current timestamp automatically. Optionally records the
    /// `previous_source` (D-C1) — the last directory that owned this skill
    /// before the transition.
    //
    // Note: production transitions to Unowned in-place by replacing
    // `entry.ownership` with `SkillOwnership::Unowned { last_owner: <old source> }`
    // (which preserves the original `synced_at` timestamp — see `cleanup_library`
    // Case 1, `remove::execute`, and `apply_edit_decisions` Fork branch).
    //
    // dead_code allow: Phase 14 Plan 14-01 widens the signature with
    // `previous_source`. Production callers arrive in Plans 14-04 (reassign
    // re-anchor flow) and 14-05 (remove-skill plan/render/execute). Drop
    // this attr when those plans land. Tracked in deferred-items.md.
    #[allow(dead_code)]
    pub fn new_unowned(
        source_path: PathBuf,
        content_hash: ContentHash,
        managed: bool,
        previous_source: Option<DirectoryName>,
    ) -> Self {
        Self {
            source_path,
            ownership: SkillOwnership::Unowned {
                last_owner: previous_source,
            },
            content_hash,
            synced_at: now_iso8601(),
            managed,
        }
    }
}

/// The unix-epoch timestamp string. A `synced_at` value of exactly this
/// almost always means a partial-save artefact or a migration bug — no
/// production codepath legitimately produces it.
const EPOCH_ZERO_TIMESTAMP: &str = "1970-01-01T00:00:00Z";

/// Pure formatter for HARD-20 (closes #433): if `synced_at` is the unix
/// epoch, return a warning string naming the affected skill; otherwise
/// return `None`. The split between detect-and-format (here) and emit
/// (in `load`) keeps the message unit-testable without stderr capture.
///
/// `dead_code` allow: the function is currently consumed only by the
/// `load` warning path and its own unit tests; preserving it as a
/// crate-private helper means future callers (e.g. `tome doctor`) can
/// reuse the exact same message.
fn epoch_zero_warning(skill_name: &SkillName, synced_at: &str) -> Option<String> {
    if synced_at == EPOCH_ZERO_TIMESTAMP {
        Some(format!(
            "warning: manifest entry for '{skill}' has unix-epoch sync-timestamp \
             ({EPOCH_ZERO_TIMESTAMP}) — this almost always indicates a partial-save \
             or migration artefact. Run `tome sync` to refresh the entry, or \
             `tome doctor` for full diagnosis.",
            skill = skill_name.as_str(),
        ))
    } else {
        None
    }
}

/// Load the manifest from the tome home directory, or return an empty one if missing.
///
/// HARD-20 (closes #433): emits a stderr warning for any entry whose
/// `synced_at` field is the unix epoch (`1970-01-01T00:00:00Z`). The
/// warning fires once per load, never poisons downstream features (the
/// entry remains in the loaded manifest), and names the skill so the
/// user can act. Implementation goes through the pure
/// `epoch_zero_warning` formatter for testability.
pub fn load(tome_home: &Path) -> Result<Manifest> {
    let path = tome_home.join(MANIFEST_FILENAME);
    if !path.exists() {
        return Ok(Manifest::default());
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read manifest {}", path.display()))?;
    let manifest: Manifest = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse manifest {}", path.display()))?;
    for (name, entry) in manifest.iter() {
        if let Some(warning) = epoch_zero_warning(name, &entry.synced_at) {
            eprintln!("{warning}");
        }
    }
    Ok(manifest)
}

/// Save the manifest to the tome home directory.
///
/// Uses a write-to-temp-then-rename pattern so the manifest file is never left in a partially
/// written (corrupted) state if the process is killed mid-write. `rename` is atomic on POSIX
/// filesystems when source and destination are on the same filesystem.
pub fn save(manifest: &Manifest, tome_home: &Path) -> Result<()> {
    let path = tome_home.join(MANIFEST_FILENAME);
    let tmp_path = tome_home.join(".tome-manifest.tmp");
    let content = serde_json::to_string_pretty(manifest).context("failed to serialize manifest")?;
    std::fs::write(&tmp_path, &content)
        .with_context(|| format!("failed to write temporary manifest {}", tmp_path.display()))?;
    if let Err(e) = std::fs::rename(&tmp_path, &path) {
        // Best-effort cleanup so a stale `.tome-manifest.tmp` doesn't
        // accumulate after a failed save (e.g. read-only target). We
        // ignore the cleanup result on purpose: the rename error is the
        // real failure to surface; masking it with a cleanup error
        // would hide the actual cause.
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e).with_context(|| {
            format!(
                "failed to rename manifest {} -> {}",
                tmp_path.display(),
                path.display()
            )
        });
    }
    Ok(())
}

/// Compute a deterministic SHA-256 hash of a directory's contents.
///
/// Walks all files in sorted order by relative path, hashing each file's
/// relative path and content into a single digest.
pub fn hash_directory(dir: &Path) -> Result<ContentHash> {
    let mut entries: Vec<(String, PathBuf)> = Vec::new();

    for entry in WalkDir::new(dir).follow_links(false).into_iter() {
        let entry = entry.with_context(|| format!("failed to walk directory {}", dir.display()))?;
        if entry.file_type().is_file() {
            let rel = entry
                .path()
                .strip_prefix(dir)
                .with_context(|| {
                    format!(
                        "BUG: WalkDir yielded path {} not under root {}",
                        entry.path().display(),
                        dir.display()
                    )
                })?
                .to_string_lossy()
                .to_string();
            entries.push((rel, entry.path().to_path_buf()));
        }
    }

    // Sort by relative path for determinism
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut hasher = Sha256::new();
    for (rel_path, abs_path) in &entries {
        hasher.update(rel_path.as_bytes());
        hasher.update(b"\0");
        let content = std::fs::read(abs_path)
            .with_context(|| format!("failed to read {}", abs_path.display()))?;
        hasher.update(&content);
    }

    Ok(ContentHash::new(
        hasher
            .finalize()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>(),
    )
    .expect("SHA-256 always produces 64 valid hex characters"))
}

/// Get the current timestamp as an ISO 8601 string (UTC, second precision).
pub(crate) fn now_iso8601() -> String {
    // Use std::time for a simple UTC timestamp without pulling in chrono
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|e| {
            eprintln!("warning: system clock appears to be set before Unix epoch: {e}");
            std::time::Duration::ZERO
        });
    let secs = duration.as_secs();

    // Manual UTC formatting: YYYY-MM-DDTHH:MM:SSZ
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    // Days since 1970-01-01
    let (year, month, day) = days_to_ymd(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

/// Convert days since epoch to (year, month, day).
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::test_hash;
    use tempfile::TempDir;

    // ---- HARD-20 epoch-0 timestamp warning (closes #433) -------------------
    // An on-disk manifest entry with `synced_at = "1970-01-01T00:00:00Z"`
    // almost always means a partial save or a migration artefact: the field
    // is the unix epoch, which never appears legitimately in production.
    // Manifest::load surfaces it as a stderr warning (informational, not
    // fatal) so future diff comparisons or display output don't silently
    // present garbage data.

    #[test]
    fn epoch_zero_warning_returns_some_for_unix_epoch() {
        let name = crate::discover::SkillName::new("ghost-skill").unwrap();
        let warning = epoch_zero_warning(&name, "1970-01-01T00:00:00Z")
            .expect("epoch-0 timestamp must produce a warning");
        // The warning must name the affected skill so the user can act.
        assert!(
            warning.contains("ghost-skill"),
            "warning must name the affected skill, got: {warning}"
        );
        // And mention "warning" or "epoch" so the user knows what they're seeing.
        assert!(
            warning.to_lowercase().contains("warning") || warning.to_lowercase().contains("epoch"),
            "warning must self-identify, got: {warning}"
        );
    }

    #[test]
    fn epoch_zero_warning_returns_none_for_normal_timestamp() {
        let name = crate::discover::SkillName::new("normal-skill").unwrap();
        assert!(
            epoch_zero_warning(&name, "2024-06-01T12:34:56Z").is_none(),
            "non-epoch timestamps must not trigger a warning"
        );
        assert!(
            epoch_zero_warning(&name, "2026-05-08T00:00:00Z").is_none(),
            "non-epoch timestamps must not trigger a warning"
        );
    }

    #[test]
    fn epoch_zero_warning_is_offered_to_load() {
        // Round-trip via the public load path: write a manifest with one
        // epoch-0 entry to disk, load it, assert the entry survives.
        let tmp = TempDir::new().unwrap();
        let valid_hash = "a".repeat(64);
        let json = format!(
            r#"{{"skills":{{"ghost-skill":{{"source_path":"/tmp/x","source_name":"old","content_hash":"{valid_hash}","synced_at":"1970-01-01T00:00:00Z","managed":false}}}}}}"#
        );
        std::fs::write(tmp.path().join(".tome-manifest.json"), json).unwrap();

        let manifest = load(tmp.path()).expect("epoch-0 entries must NOT poison load");
        let entry = manifest
            .get("ghost-skill")
            .expect("epoch-0 entries must remain loadable");
        assert_eq!(
            entry.synced_at, "1970-01-01T00:00:00Z",
            "load must preserve the literal timestamp; the warning is informational"
        );
    }

    #[test]
    fn epoch_zero_load_does_not_warn_for_normal_entries() {
        // Mixed manifest: one epoch-0 entry, one normal entry. The normal
        // entry must NOT trigger the warning. We check this via the pure
        // formatter (already covered above) and round-trip the load to
        // verify both entries are loadable.
        let tmp = TempDir::new().unwrap();
        let valid_hash = "a".repeat(64);
        let json = format!(
            r#"{{"skills":{{
                "ghost-skill":{{"source_path":"/tmp/x","source_name":"old","content_hash":"{valid_hash}","synced_at":"1970-01-01T00:00:00Z","managed":false}},
                "fresh-skill":{{"source_path":"/tmp/y","source_name":"new","content_hash":"{valid_hash}","synced_at":"2026-05-08T07:00:00Z","managed":false}}
            }}}}"#
        );
        std::fs::write(tmp.path().join(".tome-manifest.json"), json).unwrap();

        let manifest = load(tmp.path()).expect("mixed manifest must load");
        assert_eq!(manifest.len(), 2, "both entries must be present");
        // Pure-formatter assertion mirrors what load() emits at runtime:
        let normal_name = crate::discover::SkillName::new("fresh-skill").unwrap();
        assert!(
            epoch_zero_warning(&normal_name, "2026-05-08T07:00:00Z").is_none(),
            "normal entry must not trigger warning"
        );
        let ghost_name = crate::discover::SkillName::new("ghost-skill").unwrap();
        assert!(
            epoch_zero_warning(&ghost_name, "1970-01-01T00:00:00Z").is_some(),
            "epoch-0 entry must trigger warning"
        );
    }

    #[test]
    fn hash_directory_deterministic() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("a.txt"), "hello").unwrap();
        std::fs::write(tmp.path().join("b.txt"), "world").unwrap();

        let h1 = hash_directory(tmp.path()).unwrap();
        let h2 = hash_directory(tmp.path()).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_directory_changes_with_content() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("file.txt"), "version1").unwrap();
        let h1 = hash_directory(tmp.path()).unwrap();

        std::fs::write(tmp.path().join("file.txt"), "version2").unwrap();
        let h2 = hash_directory(tmp.path()).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn hash_directory_includes_subdirs() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("sub")).unwrap();
        std::fs::write(tmp.path().join("sub/nested.txt"), "deep").unwrap();

        let h1 = hash_directory(tmp.path()).unwrap();
        assert_eq!(h1.as_str().len(), 64);
    }

    #[test]
    fn manifest_roundtrip() {
        let tmp = TempDir::new().unwrap();

        let mut manifest = Manifest::default();
        let hash = test_hash("my-skill");
        manifest.insert(
            crate::discover::SkillName::new("my-skill").unwrap(),
            SkillEntry {
                source_path: PathBuf::from("/tmp/source/my-skill"),
                ownership: SkillOwnership::Owned {
                    source: DirectoryName::new("test").unwrap(),
                },
                content_hash: hash.clone(),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: false,
            },
        );

        save(&manifest, tmp.path()).unwrap();
        let loaded = load(tmp.path()).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.get("my-skill").unwrap().content_hash, hash);
    }

    #[test]
    fn load_missing_manifest_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let manifest = load(tmp.path()).unwrap();
        assert!(manifest.is_empty());
    }

    #[test]
    fn hash_directory_different_filenames_different_hashes() {
        let tmp1 = TempDir::new().unwrap();
        let tmp2 = TempDir::new().unwrap();
        // Same content, different filenames
        std::fs::write(tmp1.path().join("file_a.txt"), "hello").unwrap();
        std::fs::write(tmp2.path().join("file_b.txt"), "hello").unwrap();
        let h1 = hash_directory(tmp1.path()).unwrap();
        let h2 = hash_directory(tmp2.path()).unwrap();
        assert_ne!(
            h1, h2,
            "different filenames should produce different hashes"
        );
    }

    #[test]
    fn load_corrupt_json_returns_error() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".tome-manifest.json"), "not valid json{{{").unwrap();
        assert!(load(tmp.path()).is_err());
    }

    #[test]
    fn now_iso8601_format() {
        let ts = now_iso8601();
        // Should match YYYY-MM-DDTHH:MM:SSZ
        assert!(ts.ends_with('Z'));
        assert_eq!(ts.len(), 20);
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[10..11], "T");
    }

    #[test]
    fn days_to_ymd_epoch() {
        // Day 0 = Jan 1, 1970
        let (y, m, d) = days_to_ymd(0);
        assert_eq!((y, m, d), (1970, 1, 1));
    }

    #[test]
    fn days_to_ymd_leap_year_century_exception() {
        // Feb 29, 2000 — leap year AND century exception (divisible by 400)
        // 2000-02-29 is day 11016 since epoch
        let (y, m, d) = days_to_ymd(11016);
        assert_eq!((y, m, d), (2000, 2, 29));
    }

    #[test]
    fn days_to_ymd_end_of_first_year() {
        // Dec 31, 1970 = day 364
        let (y, m, d) = days_to_ymd(364);
        assert_eq!((y, m, d), (1970, 12, 31));
    }

    #[test]
    fn days_to_ymd_start_of_2024() {
        // Jan 1, 2024 = day 19723
        let (y, m, d) = days_to_ymd(19723);
        assert_eq!((y, m, d), (2024, 1, 1));
    }

    #[test]
    fn now_iso8601_returns_plausible_current_date() {
        // Verify that the year from now_iso8601 is 2025 or later,
        // confirming days_to_ymd works for dates beyond 2024.
        let ts = now_iso8601();
        let year: u64 = ts[..4].parse().expect("year should be numeric");
        assert!(
            year >= 2025,
            "expected current year >= 2025, got {year} from timestamp '{ts}'"
        );
    }

    #[test]
    fn update_source_name_existing_skill() {
        let mut manifest = Manifest::default();
        manifest.insert(
            crate::discover::SkillName::new("my-skill").unwrap(),
            SkillEntry::new(
                PathBuf::from("/tmp/source/my-skill"),
                DirectoryName::new("old-source").unwrap(),
                test_hash("my-skill"),
                false,
            ),
        );

        let new_source = DirectoryName::new("new-source").unwrap();
        let updated = manifest.update_source_name("my-skill", &new_source);
        assert!(updated, "should return true for existing skill");
        assert_eq!(
            manifest.get("my-skill").unwrap().source_name(),
            Some(&new_source)
        );
    }

    #[test]
    fn update_source_name_missing_skill() {
        let mut manifest = Manifest::default();
        let new_source = DirectoryName::new("new-source").unwrap();
        let updated = manifest.update_source_name("nonexistent", &new_source);
        assert!(!updated, "should return false for missing skill");
    }

    // ---- #542 SkillOwnership migration-on-read (D-08) ----------------------
    // Old manifests carry flat `source_name` (string / null / absent) + an
    // optional `previous_source`. The five tests below mirror the pre-#542
    // migration tests against the new `SkillOwnership` enum shape: old-string
    // → Owned, null → Unowned, absent → Unowned, round-trip, and no-
    // previous_source tolerance.

    #[test]
    fn deserialize_old_shape_with_source_name_string() {
        // old `source_name: "foo"` → Owned { source: "foo" }
        let valid_hash = "a".repeat(64);
        let json = format!(
            r#"{{"source_path":"/tmp/x","source_name":"foo","content_hash":"{valid_hash}","synced_at":"2024-01-01T00:00:00Z","managed":false}}"#
        );
        let entry: SkillEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(
            entry.ownership,
            SkillOwnership::Owned {
                source: DirectoryName::new("foo").unwrap()
            }
        );
        assert_eq!(
            entry.source_name(),
            Some(&DirectoryName::new("foo").unwrap())
        );
    }

    #[test]
    fn deserialize_new_shape_with_null_source_name() {
        // `source_name: null` → Unowned { last_owner: None }
        let valid_hash = "a".repeat(64);
        let json = format!(
            r#"{{"source_path":"/tmp/x","source_name":null,"content_hash":"{valid_hash}","synced_at":"2024-01-01T00:00:00Z","managed":false}}"#
        );
        let entry: SkillEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(
            entry.ownership,
            SkillOwnership::Unowned { last_owner: None }
        );
        assert_eq!(entry.source_name(), None);
    }

    #[test]
    fn deserialize_new_shape_missing_source_name() {
        // absent `source_name` key → Unowned { last_owner: None }
        let valid_hash = "a".repeat(64);
        let json = format!(
            r#"{{"source_path":"/tmp/x","content_hash":"{valid_hash}","synced_at":"2024-01-01T00:00:00Z","managed":false}}"#
        );
        let entry: SkillEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(
            entry.ownership,
            SkillOwnership::Unowned { last_owner: None }
        );
        assert_eq!(entry.source_name(), None);
    }

    #[test]
    fn skill_entry_round_trips_through_serialize_deserialize() {
        // Asymmetric serde (Serialize on the enum shape, Deserialize via
        // SkillEntryRepr) must still round-trip to an equal SkillEntry.
        for entry in [
            SkillEntry::new(
                PathBuf::from("/tmp/owned"),
                DirectoryName::new("foo").unwrap(),
                test_hash("h1"),
                true,
            ),
            SkillEntry::new_unowned(
                PathBuf::from("/tmp/unowned"),
                test_hash("h2"),
                false,
                Some(DirectoryName::new("old-dir").unwrap()),
            ),
            SkillEntry::new_unowned(
                PathBuf::from("/tmp/unowned-no-prev"),
                test_hash("h3"),
                false,
                None,
            ),
        ] {
            let json = serde_json::to_string(&entry).unwrap();
            let back: SkillEntry = serde_json::from_str(&json).unwrap();
            assert_eq!(back.source_path, entry.source_path);
            assert_eq!(back.ownership, entry.ownership);
            assert_eq!(back.content_hash, entry.content_hash);
            assert_eq!(back.synced_at, entry.synced_at);
            assert_eq!(back.managed, entry.managed);
        }
    }

    #[test]
    fn deserialize_old_shape_without_previous_source_key() {
        // No `previous_source` key on an owned entry is tolerated; the entry
        // remains Owned (no spurious breadcrumb).
        let valid_hash = "a".repeat(64);
        let json = format!(
            r#"{{"source_path":"/tmp/x","source_name":"foo","content_hash":"{valid_hash}","synced_at":"2024-01-01T00:00:00Z","managed":false}}"#
        );
        let entry: SkillEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(
            entry.ownership,
            SkillOwnership::Owned {
                source: DirectoryName::new("foo").unwrap()
            }
        );
        assert_eq!(entry.previous_source(), None);
    }

    #[test]
    fn serialize_owned_entry_emits_tagged_owned() {
        // The new enum serializes as a TS-friendly tagged union with a
        // lowercase "kind" discriminant.
        let entry = SkillEntry::new(
            PathBuf::from("/tmp/x"),
            DirectoryName::new("foo").unwrap(),
            test_hash("h"),
            false,
        );
        let json = serde_json::to_string(&entry).unwrap();
        assert!(
            json.contains("\"kind\":\"owned\""),
            "Owned entry must serialize a tagged union, got: {json}"
        );
        assert!(
            json.contains("\"source\":\"foo\""),
            "Owned entry must carry the source directory, got: {json}"
        );
    }

    #[test]
    fn serialize_unowned_entry_emits_tagged_unowned() {
        let entry = SkillEntry::new_unowned(
            PathBuf::from("/tmp/x"),
            test_hash("h"),
            false,
            Some(DirectoryName::new("old-dir").unwrap()),
        );
        let json = serde_json::to_string(&entry).unwrap();
        assert!(
            json.contains("\"kind\":\"unowned\""),
            "Unowned entry must serialize a tagged union, got: {json}"
        );
        assert!(
            json.contains("\"last_owner\":\"old-dir\""),
            "Unowned entry with last_owner=Some must carry the breadcrumb, got: {json}"
        );
    }

    #[test]
    fn new_unowned_constructor_sets_unowned_ownership() {
        let entry = SkillEntry::new_unowned(PathBuf::from("/tmp/x"), test_hash("h"), false, None);
        assert_eq!(
            entry.ownership,
            SkillOwnership::Unowned { last_owner: None }
        );
        assert_eq!(entry.source_name(), None);
        assert_eq!(entry.previous_source(), None);
        assert_eq!(entry.source_path, PathBuf::from("/tmp/x"));
        assert_eq!(entry.content_hash, test_hash("h"));
        assert!(!entry.managed);
        assert!(!entry.synced_at.is_empty());
    }

    #[test]
    fn new_unowned_records_previous_source() {
        let entry = SkillEntry::new_unowned(
            PathBuf::from("/tmp/x"),
            test_hash("h"),
            false,
            Some(DirectoryName::new("old").unwrap()),
        );
        assert_eq!(entry.source_name(), None);
        assert_eq!(
            entry.previous_source(),
            Some(&DirectoryName::new("old").unwrap())
        );
    }

    #[test]
    fn previous_source_round_trips_through_save_load() {
        let tmp = TempDir::new().unwrap();
        let mut manifest = Manifest::default();
        manifest.insert(
            crate::discover::SkillName::new("orphan").unwrap(),
            SkillEntry::new_unowned(
                PathBuf::from("/tmp/orphan"),
                test_hash("h"),
                false,
                Some(DirectoryName::new("old-source").unwrap()),
            ),
        );
        save(&manifest, tmp.path()).unwrap();
        let loaded = load(tmp.path()).unwrap();
        assert_eq!(
            loaded.get("orphan").unwrap().previous_source(),
            Some(&DirectoryName::new("old-source").unwrap()),
        );
        assert_eq!(loaded.get("orphan").unwrap().source_name(), None);
    }

    #[test]
    fn skills_get_mut_returns_some_for_existing_entry() {
        let mut manifest = Manifest::default();
        manifest.insert(
            crate::discover::SkillName::new("my-skill").unwrap(),
            SkillEntry::new(
                PathBuf::from("/tmp/source/my-skill"),
                DirectoryName::new("src").unwrap(),
                test_hash("h"),
                false,
            ),
        );
        {
            let entry = manifest
                .skills_get_mut("my-skill")
                .expect("should be present");
            entry.ownership = SkillOwnership::Unowned { last_owner: None };
        }
        assert_eq!(manifest.get("my-skill").unwrap().source_name(), None);
    }

    #[test]
    fn skills_get_mut_returns_none_for_missing_entry() {
        let mut manifest = Manifest::default();
        assert!(manifest.skills_get_mut("nonexistent").is_none());
    }

    /// HARD-08: if the rename step of the atomic save fails, the previous
    /// on-disk manifest content must remain untouched.
    ///
    /// Mechanism: write a known content A via the happy path, then chmod
    /// the parent directory to 0o500 (read+exec, no write). Calling
    /// `save()` with content B fails at the rename step (EACCES); the
    /// original file must still parse to content A.
    ///
    /// Cross-platform: chmod-on-parent-dir is portable across macOS and
    /// Linux. Permissions are restored before the test exits so TempDir
    /// drop can clean up.
    #[cfg(unix)]
    #[test]
    fn save_preserves_previous_on_rename_failure() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = TempDir::new().unwrap();

        // Step 1: write content A via the canonical save path.
        let mut manifest_a = Manifest::default();
        manifest_a.insert(
            crate::discover::SkillName::new("alpha").unwrap(),
            SkillEntry::new(
                PathBuf::from("/tmp/alpha"),
                DirectoryName::new("src-a").unwrap(),
                test_hash("a"),
                false,
            ),
        );
        save(&manifest_a, tmp.path()).unwrap();
        let path_a = tmp.path().join(MANIFEST_FILENAME);
        let bytes_a = std::fs::read(&path_a).unwrap();

        // Step 2: chmod parent to read+exec only — rename will EACCES.
        let original_mode = std::fs::metadata(tmp.path()).unwrap().permissions().mode();
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o500)).unwrap();

        // Step 3: attempt to save a DIFFERENT content B; it must fail.
        let mut manifest_b = Manifest::default();
        manifest_b.insert(
            crate::discover::SkillName::new("beta").unwrap(),
            SkillEntry::new(
                PathBuf::from("/tmp/beta"),
                DirectoryName::new("src-b").unwrap(),
                test_hash("b"),
                false,
            ),
        );
        let result = save(&manifest_b, tmp.path());

        // Restore permissions BEFORE asserting so TempDir cleanup works
        // even if the assertion panics.
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(original_mode))
            .unwrap();

        assert!(
            result.is_err(),
            "save() must fail when rename target is unwritable"
        );

        // Step 4: re-read the manifest. It must STILL be content A.
        let bytes_after = std::fs::read(&path_a).unwrap();
        assert_eq!(
            bytes_after, bytes_a,
            "atomic-save invariant violated: original manifest content was \
             corrupted by a failed save"
        );
        let reloaded = load(tmp.path()).unwrap();
        assert!(
            reloaded.get("alpha").is_some(),
            "post-fail manifest must still hold the previous (alpha) entry"
        );
        assert!(
            reloaded.get("beta").is_none(),
            "post-fail manifest must NOT hold the new (beta) entry"
        );
    }

    // ---- OBS-07 last_synced_at header field (D-LSYNC-1) -------------------
    // Additive-compat manifest header: pre-v0.11 manifests deserialize cleanly
    // with `last_synced_at: None`. A fresh stamp survives serde round-trip.
    // Default manifests omit the key on serialize (`skip_serializing_if`).

    #[test]
    fn manifest_pre_v011_json_deserializes_with_none_last_synced_at() {
        // Pre-v0.11 manifest shape: `{"skills": {}}` with no header field.
        // Must round-trip to a Manifest with last_synced_at == None.
        let json = r#"{"skills": {}}"#;
        let manifest: Manifest =
            serde_json::from_str(json).expect("pre-v0.11 manifest must deserialize cleanly");
        assert_eq!(
            manifest.last_synced_at(),
            None,
            "pre-v0.11 manifest must produce last_synced_at == None"
        );
    }

    #[test]
    fn manifest_stamp_round_trip_preserves_timestamp() {
        let mut m = Manifest::default();
        m.stamp_last_synced_at();
        let stamped = m
            .last_synced_at()
            .expect("stamp must populate last_synced_at")
            .to_string();
        let s = serde_json::to_string(&m).expect("stamped manifest must serialize");
        let m2: Manifest = serde_json::from_str(&s).expect("stamped manifest must deserialize");
        let round_tripped = m2
            .last_synced_at()
            .expect("round-tripped manifest must still have last_synced_at");
        assert_eq!(
            round_tripped, stamped,
            "round-trip must preserve last_synced_at byte-for-byte"
        );
        // The stamp is produced via now_iso8601(); shape is "YYYY-MM-DDTHH:MM:SSZ".
        assert!(
            round_tripped.ends_with('Z') && round_tripped.len() == 20,
            "stamp must be RFC-3339 'YYYY-MM-DDTHH:MM:SSZ', got: {round_tripped}"
        );
    }

    #[test]
    fn manifest_default_skips_last_synced_at_in_json() {
        let m = Manifest::default();
        let json = serde_json::to_string(&m).unwrap();
        assert!(
            !json.contains("last_synced_at"),
            "default manifest must NOT serialize last_synced_at key, got: {json}"
        );
    }

    #[test]
    fn manifest_last_synced_at_accessor_shape() {
        let mut m = Manifest::default();
        assert_eq!(
            m.last_synced_at(),
            None,
            "default manifest must report None"
        );
        m.stamp_last_synced_at();
        let stamped = m
            .last_synced_at()
            .expect("after stamp, accessor must return Some(&str)");
        assert!(
            !stamped.is_empty(),
            "stamped value must be non-empty, got: {stamped:?}"
        );
    }

    /// HARD-08 round-trip pin: previous_source survives a full save -> load
    /// round-trip when the entry is Owned (Some) — companion to the existing
    /// Unowned-state round-trip test above. Ensures Phase 14 D-C1 schema is
    /// stable across save() failures and successes alike.
    #[test]
    fn previous_source_round_trip_through_save_load_owned_keeps_owned() {
        let tmp = TempDir::new().unwrap();
        let mut manifest = Manifest::default();
        // Owned entry with NO previous_source (the common case).
        manifest.insert(
            crate::discover::SkillName::new("kept").unwrap(),
            SkillEntry::new(
                PathBuf::from("/tmp/kept"),
                DirectoryName::new("active-source").unwrap(),
                test_hash("k"),
                false,
            ),
        );
        save(&manifest, tmp.path()).unwrap();
        let loaded = load(tmp.path()).unwrap();
        let entry = loaded.get("kept").unwrap();
        assert_eq!(
            entry.source_name(),
            Some(&DirectoryName::new("active-source").unwrap())
        );
        assert_eq!(entry.previous_source(), None);
    }
}
