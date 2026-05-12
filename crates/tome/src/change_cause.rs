//! `ChangeCause` — typed reason a skill was re-emitted by consolidate or
//! distribute. OBS-04 (Phase 18) locks the four user-facing strings.
//!
//! Greppability matters: `grep "cause=hash changed" sync-output.txt` is
//! the user's debugging workflow. Renaming any string is a BREAKING change
//! to the OBS-04 contract.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeCause {
    HashChanged,
    PreviouslyFailed,
    NewlyAdded,
    DirectoryNowAllowed,
}

impl ChangeCause {
    /// POLISH-04 exhaustiveness sentinel — compile fails if a new variant
    /// is added without updating ChangeCause::ALL. Mirrors LogLevel::ALL
    /// (cli.rs:28) and MigrationFailureKind::ALL (migration_v010.rs:53).
    #[allow(dead_code)]
    pub const ALL: [Self; 4] = [
        Self::HashChanged,
        Self::PreviouslyFailed,
        Self::NewlyAdded,
        Self::DirectoryNowAllowed,
    ];
}

#[allow(dead_code)]
fn _change_cause_exhaustiveness(c: ChangeCause) {
    match c {
        ChangeCause::HashChanged => {}
        ChangeCause::PreviouslyFailed => {}
        ChangeCause::NewlyAdded => {}
        ChangeCause::DirectoryNowAllowed => {}
    }
}

const _: () = assert!(
    ChangeCause::ALL.len() == 4,
    "ChangeCause::ALL must list every variant",
);

impl fmt::Display for ChangeCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::HashChanged => "hash changed",
            Self::PreviouslyFailed => "previously failed",
            Self::NewlyAdded => "newly added",
            Self::DirectoryNowAllowed => "directory now allowed",
        };
        f.write_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_strings_match_obs04_vocabulary() {
        // Hard-pin the OBS-04 grep vocabulary. Renaming is BREAKING.
        assert_eq!(ChangeCause::HashChanged.to_string(), "hash changed");
        assert_eq!(
            ChangeCause::PreviouslyFailed.to_string(),
            "previously failed"
        );
        assert_eq!(ChangeCause::NewlyAdded.to_string(), "newly added");
        assert_eq!(
            ChangeCause::DirectoryNowAllowed.to_string(),
            "directory now allowed"
        );
    }

    #[test]
    fn all_array_has_length_four() {
        assert_eq!(ChangeCause::ALL.len(), 4);
    }
}
