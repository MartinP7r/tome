//! Shared validation logic for identifier types (skill names, target names)
//! and the `ContentHash` newtype for compile-time hash safety.

use anyhow::Result;
/// Validate an identifier string used for skill or target names.
///
/// Rejects empty names, `.`/`..`, whitespace-only or leading/trailing whitespace,
/// and names containing path separators (`/` or `\`).
///
/// The `kind` parameter is used in error messages (e.g. "skill name", "target name").
pub(crate) fn validate_identifier(name: &str, kind: &str) -> Result<()> {
    anyhow::ensure!(!name.is_empty(), "{kind} cannot be empty");
    anyhow::ensure!(name != "." && name != "..", "{kind} cannot be '.' or '..'");
    anyhow::ensure!(
        !name.chars().all(|c| c.is_whitespace()) && name.trim() == name,
        "{kind} cannot be whitespace-only or have leading/trailing whitespace: '{name}'"
    );
    anyhow::ensure!(
        !name.contains('/') && !name.contains('\\'),
        "{kind} contains path separator: '{name}'"
    );
    Ok(())
}

/// A validated SHA-256 content hash (64 hex characters).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(transparent)]
pub struct ContentHash(String);

impl ContentHash {
    pub fn new(hash: impl Into<String>) -> Result<Self> {
        let hash = hash.into();
        if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            anyhow::bail!("invalid content hash: expected 64 hex characters, got '{hash}'");
        }
        // Normalize to lowercase
        Ok(Self(hash.to_ascii_lowercase()))
    }

    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ContentHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> serde::Deserialize<'de> for ContentHash {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        ContentHash::new(s).map_err(serde::de::Error::custom)
    }
}

/// Create a test ContentHash by hashing a seed string with SHA-256.
#[cfg(test)]
pub fn test_hash(seed: &str) -> ContentHash {
    use sha2::{Digest, Sha256};
    let hash = format!("{:x}", Sha256::digest(seed.as_bytes()));
    ContentHash::new(hash).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty() {
        assert!(validate_identifier("", "test name").is_err());
    }

    #[test]
    fn rejects_dots() {
        assert!(validate_identifier(".", "test name").is_err());
        assert!(validate_identifier("..", "test name").is_err());
    }

    #[test]
    fn rejects_whitespace() {
        assert!(validate_identifier("  ", "test name").is_err());
        assert!(validate_identifier(" leading", "test name").is_err());
        assert!(validate_identifier("trailing ", "test name").is_err());
    }

    #[test]
    fn rejects_path_separators() {
        assert!(validate_identifier("foo/bar", "test name").is_err());
        assert!(validate_identifier("foo\\bar", "test name").is_err());
    }

    #[test]
    fn accepts_valid() {
        assert!(validate_identifier("my-skill-123", "test name").is_ok());
        assert!(validate_identifier("My_Skill", "test name").is_ok());
    }

    // -- ContentHash tests --

    #[test]
    fn content_hash_valid() {
        let hash = "a".repeat(64);
        let ch = ContentHash::new(hash).unwrap();
        assert_eq!(ch.as_str().len(), 64);
    }

    #[test]
    fn content_hash_too_short() {
        assert!(ContentHash::new("abc123").is_err());
    }

    #[test]
    fn content_hash_too_long() {
        let hash = "a".repeat(65);
        assert!(ContentHash::new(hash).is_err());
    }

    #[test]
    fn content_hash_non_hex() {
        let hash = "g".repeat(64);
        assert!(ContentHash::new(hash).is_err());
    }

    #[test]
    fn content_hash_normalizes_uppercase() {
        let hash = "A".repeat(64);
        let ch = ContentHash::new(hash).unwrap();
        assert_eq!(ch.as_str(), "a".repeat(64));
    }

    #[test]
    fn content_hash_display() {
        let hash = "b".repeat(64);
        let ch = ContentHash::new(hash.clone()).unwrap();
        assert_eq!(format!("{ch}"), hash);
    }

    #[test]
    fn content_hash_serde_roundtrip() {
        let ch = test_hash("test-seed");
        let json = serde_json::to_string(&ch).unwrap();
        let parsed: ContentHash = serde_json::from_str(&json).unwrap();
        assert_eq!(ch, parsed);
    }

    #[test]
    fn content_hash_deserialize_rejects_invalid() {
        let result: std::result::Result<ContentHash, _> = serde_json::from_str(r#""not-a-hash""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_hash_helper_produces_valid_hash() {
        let h = test_hash("anything");
        assert_eq!(h.as_str().len(), 64);
        assert!(h.as_str().chars().all(|c| c.is_ascii_hexdigit()));
    }
}
