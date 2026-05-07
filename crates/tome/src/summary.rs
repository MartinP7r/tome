//! Shared skill-summary type for `tome status` and `tome doctor` Unowned
//! section rendering (UNOWN-03 / D-D3).
//!
//! Both `status::StatusReport` and `doctor::DoctorReport` carry
//! `unowned: Vec<SkillSummary>` fields. The type is intentionally
//! display-shaped: `previous_source` is the clean directory name when
//! present (D-C1), `source_path_display` is the always-populated
//! `collapse_home`-rendered fallback (D-C2). JSON output presents both
//! so consumers can pick whichever is more informative.

use crate::discover::SkillName;
use crate::manifest::SkillEntry;

/// One row of the Unowned section in `tome status` and `tome doctor`.
/// Per D-D3 in the Phase 14 CONTEXT.md.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SkillSummary {
    /// Skill name as displayed.
    pub name: String,
    /// Last directory that owned this skill, captured at transition time
    /// (D-C1). `None` for entries that became Unowned before Phase 14
    /// landed — consumers fall back to `source_path_display` (D-C2).
    pub previous_source: Option<String>,
    /// `paths::collapse_home`-rendered `source_path` from the manifest.
    /// Always populated; serves as the D-C2 fallback when
    /// `previous_source` is `None`, and as supplementary info otherwise.
    pub source_path_display: String,
    /// ISO 8601 timestamp from the manifest (preserved across Owned→Unowned
    /// transition per Phase 11 manifest semantics).
    pub synced_at: String,
    /// Mirrors `SkillEntry::managed`. Display-only; consumers may want
    /// to surface "originally a managed plugin" for context.
    pub managed: bool,
}

impl SkillSummary {
    /// Build a summary from a manifest entry and its name. No filesystem
    /// I/O — purely a projection of `SkillEntry` fields.
    //
    // dead_code allow: consumed in 14-06 (status) and 14-07 (doctor) within
    // this wave-set; remove this attr when those plans land.
    #[allow(dead_code)]
    pub fn from_entry(name: &SkillName, entry: &SkillEntry) -> Self {
        Self {
            name: name.as_str().to_string(),
            previous_source: entry
                .previous_source
                .as_ref()
                .map(|d| d.as_str().to_string()),
            source_path_display: crate::paths::collapse_home(&entry.source_path),
            synced_at: entry.synced_at.clone(),
            managed: entry.managed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DirectoryName;
    use crate::manifest::SkillEntry;
    use crate::validation::test_hash;
    use std::path::PathBuf;

    fn unowned_entry_with_previous(previous: Option<&str>) -> SkillEntry {
        SkillEntry::new_unowned(
            PathBuf::from("/tmp/orphan-skill"),
            test_hash("h"),
            false,
            previous.map(|s| DirectoryName::new(s).unwrap()),
        )
    }

    #[test]
    fn from_entry_populates_previous_source_when_present() {
        let entry = unowned_entry_with_previous(Some("removed-source"));
        let name = SkillName::new("orphan-skill").unwrap();
        let summary = SkillSummary::from_entry(&name, &entry);
        assert_eq!(summary.name, "orphan-skill");
        assert_eq!(summary.previous_source, Some("removed-source".to_string()));
        assert!(!summary.managed);
    }

    #[test]
    fn from_entry_falls_back_when_previous_source_missing() {
        // D-C2 fallback case: an Unowned entry that became Unowned
        // before Phase 14 landed has previous_source = None. Consumers
        // render source_path_display.
        let entry = unowned_entry_with_previous(None);
        let name = SkillName::new("legacy-orphan").unwrap();
        let summary = SkillSummary::from_entry(&name, &entry);
        assert_eq!(summary.previous_source, None);
        assert!(
            !summary.source_path_display.is_empty(),
            "source_path_display must always be populated for D-C2 fallback"
        );
    }

    #[test]
    fn json_shape_includes_all_keys() {
        let entry = unowned_entry_with_previous(Some("foo"));
        let name = SkillName::new("bar").unwrap();
        let summary = SkillSummary::from_entry(&name, &entry);
        let value = serde_json::to_value(&summary).unwrap();
        let obj = value
            .as_object()
            .expect("SkillSummary serializes to JSON object");
        for key in [
            "name",
            "previous_source",
            "source_path_display",
            "synced_at",
            "managed",
        ] {
            assert!(
                obj.contains_key(key),
                "SkillSummary JSON must contain key '{key}', got: {value}"
            );
        }
        assert_eq!(obj["name"], "bar");
        assert_eq!(obj["previous_source"], "foo");
        assert_eq!(obj["managed"], false);
    }

    #[test]
    fn json_previous_source_serializes_as_null_when_none() {
        // Stable JSON shape: consumers should always see the key with
        // an explicit null when previous_source is absent. NO skip_serializing_if.
        let entry = unowned_entry_with_previous(None);
        let name = SkillName::new("legacy").unwrap();
        let summary = SkillSummary::from_entry(&name, &entry);
        let value = serde_json::to_value(&summary).unwrap();
        assert!(
            value["previous_source"].is_null(),
            "previous_source must serialize as null (not omitted) when None: {value}"
        );
    }
}
