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
    pub fn update_source_name(&mut self, skill_name: &str, new_source: &DirectoryName) -> bool {
        if let Some(entry) = self.skills.get_mut(skill_name)
            && entry.source_name.is_some()
        {
            entry.source_name = Some(new_source.clone());
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
}

/// A single skill entry in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEntry {
    /// Where this skill was originally copied from.
    pub source_path: PathBuf,
    /// Which directory config entry contributed this skill, or `None` if the
    /// skill is **Unowned** (its source was removed from `tome.toml` but the
    /// library copy is preserved per LIB-04).
    ///
    /// Old manifests with `"source_name": "foo"` parse as `Some(DirectoryName::new("foo")?)`
    /// via serde's natural `Option` handling + `DirectoryName`'s transparent
    /// validating `Deserialize`. New Unowned entries serialize without the key
    /// (per `skip_serializing_if`) and read back as `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_name: Option<DirectoryName>,
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

impl SkillEntry {
    /// Create a new `SkillEntry` for an **owned** skill (source_name known).
    /// Records the current timestamp automatically.
    pub fn new(
        source_path: PathBuf,
        source_name: DirectoryName,
        content_hash: ContentHash,
        managed: bool,
    ) -> Self {
        Self {
            source_path,
            source_name: Some(source_name),
            content_hash,
            synced_at: now_iso8601(),
            managed,
        }
    }

    /// Create a new `SkillEntry` for an **Unowned** skill — its source was
    /// removed from `tome.toml` but the library copy is preserved (per LIB-04).
    /// Records the current timestamp automatically.
    pub fn new_unowned(
        source_path: PathBuf,
        content_hash: ContentHash,
        managed: bool,
    ) -> Self {
        Self {
            source_path,
            source_name: None,
            content_hash,
            synced_at: now_iso8601(),
            managed,
        }
    }
}

/// Load the manifest from the tome home directory, or return an empty one if missing.
pub fn load(tome_home: &Path) -> Result<Manifest> {
    let path = tome_home.join(MANIFEST_FILENAME);
    if !path.exists() {
        return Ok(Manifest::default());
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read manifest {}", path.display()))?;
    let manifest: Manifest = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse manifest {}", path.display()))?;
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
                source_name: Some(DirectoryName::new("test").unwrap()),
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
            manifest.get("my-skill").unwrap().source_name,
            Some(new_source)
        );
    }

    #[test]
    fn update_source_name_missing_skill() {
        let mut manifest = Manifest::default();
        let new_source = DirectoryName::new("new-source").unwrap();
        let updated = manifest.update_source_name("nonexistent", &new_source);
        assert!(!updated, "should return false for missing skill");
    }

    #[test]
    fn deserialize_old_shape_with_source_name_string() {
        let valid_hash = "a".repeat(64);
        let json = format!(
            r#"{{"source_path":"/tmp/x","source_name":"foo","content_hash":"{valid_hash}","synced_at":"2024-01-01T00:00:00Z","managed":false}}"#
        );
        let entry: SkillEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry.source_name, Some(DirectoryName::new("foo").unwrap()));
    }

    #[test]
    fn deserialize_new_shape_with_null_source_name() {
        let valid_hash = "a".repeat(64);
        let json = format!(
            r#"{{"source_path":"/tmp/x","source_name":null,"content_hash":"{valid_hash}","synced_at":"2024-01-01T00:00:00Z","managed":false}}"#
        );
        let entry: SkillEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry.source_name, None);
    }

    #[test]
    fn deserialize_new_shape_missing_source_name() {
        let valid_hash = "a".repeat(64);
        let json = format!(
            r#"{{"source_path":"/tmp/x","content_hash":"{valid_hash}","synced_at":"2024-01-01T00:00:00Z","managed":false}}"#
        );
        let entry: SkillEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry.source_name, None);
    }

    #[test]
    fn serialize_unowned_entry_omits_source_name_key() {
        let entry = SkillEntry::new_unowned(
            PathBuf::from("/tmp/orphan"),
            test_hash("orphan"),
            false,
        );
        let json = serde_json::to_string(&entry).unwrap();
        assert!(
            !json.contains("source_name"),
            "Unowned entry must omit source_name key, got: {json}"
        );
        assert!(json.contains("\"managed\":false"));
    }

    #[test]
    fn serialize_owned_entry_preserves_string_shape() {
        let entry = SkillEntry::new(
            PathBuf::from("/tmp/x"),
            DirectoryName::new("foo").unwrap(),
            test_hash("h"),
            false,
        );
        let json = serde_json::to_string(&entry).unwrap();
        assert!(
            json.contains("\"source_name\":\"foo\""),
            "Owned entry must serialize source_name as string, got: {json}"
        );
    }

    #[test]
    fn new_unowned_constructor_sets_source_name_none() {
        let entry = SkillEntry::new_unowned(
            PathBuf::from("/tmp/x"),
            test_hash("h"),
            false,
        );
        assert_eq!(entry.source_name, None);
        assert_eq!(entry.source_path, PathBuf::from("/tmp/x"));
        assert_eq!(entry.content_hash, test_hash("h"));
        assert!(!entry.managed);
        assert!(!entry.synced_at.is_empty());
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
            entry.source_name = None;
        }
        assert_eq!(manifest.get("my-skill").unwrap().source_name, None);
    }

    #[test]
    fn skills_get_mut_returns_none_for_missing_entry() {
        let mut manifest = Manifest::default();
        assert!(manifest.skills_get_mut("nonexistent").is_none());
    }
}
