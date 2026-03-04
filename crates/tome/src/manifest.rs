//! Library manifest — tracks provenance and content hashes for each skill in the library.
//!
//! The manifest file (`.tome-manifest.json`) lives at the library root and records where each
//! skill was copied from, its content hash, and when it was last synced. This enables idempotent
//! copy-based consolidation: unchanged skills are skipped, modified skills are re-copied.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

const MANIFEST_FILENAME: &str = ".tome-manifest.json";

/// The library manifest, tracking all skills and their provenance.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Manifest {
    pub skills: BTreeMap<String, SkillEntry>,
}

/// A single skill entry in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEntry {
    /// Where this skill was originally copied from.
    pub source_path: PathBuf,
    /// Which source config entry contributed this skill.
    pub source_name: String,
    /// SHA-256 hex digest of the directory contents.
    pub content_hash: String,
    /// ISO 8601 timestamp of when this skill was last synced.
    pub synced_at: String,
}

/// Load the manifest from the library directory, or return an empty one if missing.
pub fn load(library_dir: &Path) -> Result<Manifest> {
    let path = library_dir.join(MANIFEST_FILENAME);
    if !path.exists() {
        return Ok(Manifest::default());
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read manifest {}", path.display()))?;
    let manifest: Manifest = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse manifest {}", path.display()))?;
    Ok(manifest)
}

/// Save the manifest to the library directory.
///
/// Uses a write-to-temp-then-rename pattern so the manifest file is never left in a partially
/// written (corrupted) state if the process is killed mid-write. `rename` is atomic on POSIX
/// filesystems when source and destination are on the same filesystem.
pub fn save(manifest: &Manifest, library_dir: &Path) -> Result<()> {
    let path = library_dir.join(MANIFEST_FILENAME);
    let tmp_path = library_dir.join(".tome-manifest.tmp");
    let content = serde_json::to_string_pretty(manifest).context("failed to serialize manifest")?;
    std::fs::write(&tmp_path, &content)
        .with_context(|| format!("failed to write temporary manifest {}", tmp_path.display()))?;
    std::fs::rename(&tmp_path, &path).with_context(|| {
        format!(
            "failed to rename manifest {} -> {}",
            tmp_path.display(),
            path.display()
        )
    })
}

/// Compute a deterministic SHA-256 hash of a directory's contents.
///
/// Walks all files in sorted order by relative path, hashing each file's
/// relative path and content into a single digest.
pub fn hash_directory(dir: &Path) -> Result<String> {
    let mut entries: Vec<(String, PathBuf)> = Vec::new();

    for entry in WalkDir::new(dir).follow_links(false).into_iter() {
        let entry = entry.with_context(|| format!("failed to walk directory {}", dir.display()))?;
        if entry.file_type().is_file() {
            let rel = entry
                .path()
                .strip_prefix(dir)
                .unwrap_or(entry.path())
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

    Ok(format!("{:x}", hasher.finalize()))
}

/// Get the current timestamp as an ISO 8601 string (UTC, second precision).
pub fn now_iso8601() -> String {
    // Use std::time for a simple UTC timestamp without pulling in chrono
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
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
        assert!(!h1.is_empty());
    }

    #[test]
    fn manifest_roundtrip() {
        let tmp = TempDir::new().unwrap();

        let mut manifest = Manifest::default();
        manifest.skills.insert(
            "my-skill".to_string(),
            SkillEntry {
                source_path: PathBuf::from("/tmp/source/my-skill"),
                source_name: "test".to_string(),
                content_hash: "abc123".to_string(),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
            },
        );

        save(&manifest, tmp.path()).unwrap();
        let loaded = load(tmp.path()).unwrap();
        assert_eq!(loaded.skills.len(), 1);
        assert_eq!(loaded.skills["my-skill"].content_hash, "abc123");
    }

    #[test]
    fn load_missing_manifest_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let manifest = load(tmp.path()).unwrap();
        assert!(manifest.skills.is_empty());
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
}
