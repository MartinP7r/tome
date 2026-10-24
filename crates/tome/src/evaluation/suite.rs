//! Versioned TOML evaluation-suite format and validation.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::validation::{ContentHash, validate_identifier};

use super::contract::{
    ActivationState, EventKind, ExecutionPolicy, MAX_ARTIFACT_PATH_BYTES, MAX_EVENT_NAME_BYTES,
    MAX_GRADERS_PER_CASE, ModelPin, ObservationCapability, Variant, validate_model_pin,
    validate_safe_token,
};

/// Current author-authored suite schema version.
pub const SUITE_SCHEMA_VERSION: u32 = 1;
/// Bound repetitions so an untrusted suite cannot schedule unbounded work later.
pub const MAX_REPETITIONS: u32 = 100;
/// Bound the number of runs produced by a single suite.
pub(crate) const MAX_RUNS: usize = 10_000;

/// A validated provider-neutral evaluation suite.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationSuite {
    pub schema_version: u32,
    pub suite_id: String,
    pub artifact: ArtifactIdentity,
    #[serde(default = "default_repetitions")]
    pub repetitions: u32,
    #[serde(default)]
    pub policy: ExecutionPolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_pin: Option<ModelPin>,
    pub cases: Vec<EvaluationCase>,
}

fn default_repetitions() -> u32 {
    1
}

/// Immutable identity for the exact artifact under evaluation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactIdentity {
    pub name: String,
    pub sha256: ContentHash,
}

/// One prompt/fixture scenario and its deterministic graders.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationCase {
    pub id: String,
    pub prompt: String,
    /// Relative path from the suite directory to this case's fixture directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixture: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub graders: Vec<GraderSpec>,
}

/// A deterministic grader declaration. `kind` is the TOML discriminator.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GraderSpec {
    TextContains {
        id: String,
        needle: String,
        #[serde(default = "default_weight")]
        weight: u32,
    },
    TextRegex {
        id: String,
        pattern: String,
        #[serde(default = "default_weight")]
        weight: u32,
    },
    ArtifactExists {
        id: String,
        path: String,
        expected_present: bool,
        #[serde(default = "default_weight")]
        weight: u32,
    },
    ArtifactSha256 {
        id: String,
        path: String,
        expected_sha256: ContentHash,
        #[serde(default = "default_weight")]
        weight: u32,
    },
    EventCount {
        id: String,
        event_kind: EventKind,
        name: String,
        expected_count: usize,
        #[serde(default = "default_weight")]
        weight: u32,
    },
    EventOrder {
        id: String,
        sequence: Vec<EventSelector>,
        #[serde(default = "default_weight")]
        weight: u32,
    },
    ArtifactAllowlist {
        id: String,
        allowed_paths: Vec<String>,
        #[serde(default = "default_weight")]
        weight: u32,
    },
    Activation {
        id: String,
        #[serde(default)]
        treatment: Option<ActivationState>,
        #[serde(default)]
        control: Option<ActivationState>,
        #[serde(default = "default_weight")]
        weight: u32,
    },
}

fn default_weight() -> u32 {
    1
}

/// A selector used by event-occurrence and event-order graders.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventSelector {
    pub event_kind: EventKind,
    pub name: String,
}

impl GraderSpec {
    /// Stable grader identifier.
    pub fn id(&self) -> &str {
        match self {
            Self::TextContains { id, .. }
            | Self::TextRegex { id, .. }
            | Self::ArtifactExists { id, .. }
            | Self::ArtifactSha256 { id, .. }
            | Self::EventCount { id, .. }
            | Self::EventOrder { id, .. }
            | Self::ArtifactAllowlist { id, .. }
            | Self::Activation { id, .. } => id,
        }
    }

    /// Weight used by the score when this grader is score-bearing.
    pub fn weight(&self) -> u32 {
        match self {
            Self::TextContains { weight, .. }
            | Self::TextRegex { weight, .. }
            | Self::ArtifactExists { weight, .. }
            | Self::ArtifactSha256 { weight, .. }
            | Self::EventCount { weight, .. }
            | Self::EventOrder { weight, .. }
            | Self::ArtifactAllowlist { weight, .. }
            | Self::Activation { weight, .. } => *weight,
        }
    }

    /// Whether this grader contributes to the score for a variant.
    /// Positive activation is diagnostic-only; negative routing remains scoreable.
    pub fn score_bearing_for(&self, variant: Variant) -> bool {
        match self {
            Self::Activation {
                treatment, control, ..
            } => match match variant {
                Variant::Treatment => treatment,
                Variant::Control => control,
            } {
                Some(ActivationState::NotActivated) => true,
                Some(ActivationState::Activated) | None => false,
            },
            _ => true,
        }
    }

    /// Evidence channel required by this grader.
    pub fn required_capability(&self) -> ObservationCapability {
        match self {
            Self::TextContains { .. } | Self::TextRegex { .. } => ObservationCapability::FinalText,
            Self::ArtifactExists { .. }
            | Self::ArtifactSha256 { .. }
            | Self::ArtifactAllowlist { .. } => ObservationCapability::ArtifactManifest,
            Self::EventCount { .. } | Self::EventOrder { .. } => ObservationCapability::Events,
            Self::Activation { .. } => ObservationCapability::Activation,
        }
    }

    pub(crate) fn validate(&self) -> Result<()> {
        validate_safe_token(self.id(), "grader id", 128, false)?;
        ensure!(
            self.weight() > 0,
            "grader '{}' weight must be positive",
            self.id()
        );

        match self {
            Self::TextContains { needle, .. } => {
                ensure!(
                    !needle.is_empty(),
                    "grader '{}' needle cannot be empty",
                    self.id()
                );
            }
            Self::TextRegex { pattern, .. } => {
                Regex::new(pattern)
                    .with_context(|| format!("grader '{}' has an invalid regex", self.id()))?;
            }
            Self::ArtifactExists { path, .. } | Self::ArtifactSha256 { path, .. } => {
                validate_relative_path(path, "grader artifact path")?;
            }
            Self::EventCount {
                name,
                expected_count,
                ..
            } => {
                validate_safe_token(name, "grader event name", MAX_EVENT_NAME_BYTES, false)?;
                ensure!(
                    *expected_count <= super::contract::MAX_GRADER_EVIDENCE_REFS,
                    "grader '{}' expected event count exceeds evidence limit",
                    self.id()
                );
            }
            Self::EventOrder { sequence, .. } => {
                ensure!(
                    !sequence.is_empty(),
                    "grader '{}' event sequence cannot be empty",
                    self.id()
                );
                ensure!(
                    sequence.len() <= super::contract::MAX_GRADER_EVIDENCE_REFS,
                    "grader '{}' event sequence exceeds evidence limit",
                    self.id()
                );
                for selector in sequence {
                    validate_safe_token(
                        &selector.name,
                        "grader event name",
                        MAX_EVENT_NAME_BYTES,
                        false,
                    )?;
                }
            }
            Self::ArtifactAllowlist { allowed_paths, .. } => {
                for path in allowed_paths {
                    validate_relative_path(path, "allowlisted artifact path")?;
                }
            }
            Self::Activation {
                treatment, control, ..
            } => {
                ensure!(
                    treatment.is_some() || control.is_some(),
                    "grader '{}' must specify an activation expectation",
                    self.id()
                );
            }
        }
        Ok(())
    }
}

/// Parse TOML and validate the complete versioned suite before it can be used.
pub fn parse_suite_toml(input: &str) -> Result<EvaluationSuite> {
    let suite: EvaluationSuite =
        toml::from_str(input).context("failed to parse evaluation suite TOML")?;
    suite.validate()?;
    Ok(suite)
}

/// Read and validate a suite TOML file. Fixture paths remain relative to its parent directory.
pub fn load_suite_file(path: &Path) -> Result<EvaluationSuite> {
    let input = fs::read_to_string(path)
        .with_context(|| format!("failed to read evaluation suite '{}'", path.display()))?;
    parse_suite_toml(&input)
        .with_context(|| format!("invalid evaluation suite '{}'", path.display()))
}

impl EvaluationSuite {
    /// Validate a suite built programmatically or deserialized from another format.
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.schema_version == SUITE_SCHEMA_VERSION,
            "unsupported evaluation suite schema_version {}; supported version is {}",
            self.schema_version,
            SUITE_SCHEMA_VERSION
        );
        validate_identifier(&self.suite_id, "evaluation suite id")?;
        validate_identifier(&self.artifact.name, "artifact name")?;
        ensure!(
            (1..=MAX_REPETITIONS).contains(&self.repetitions),
            "repetitions must be between 1 and {MAX_REPETITIONS}"
        );
        self.policy.validate()?;
        if let Some(pin) = &self.model_pin {
            validate_model_pin(pin)?;
        }
        ensure!(
            !self.cases.is_empty(),
            "evaluation suite must contain at least one case"
        );
        let run_count = self
            .cases
            .len()
            .checked_mul(self.repetitions as usize)
            .and_then(|count| count.checked_mul(Variant::ALL.len()))
            .context("evaluation suite run count overflowed")?;
        ensure!(
            run_count <= MAX_RUNS,
            "evaluation suite exceeds the {MAX_RUNS}-run limit"
        );

        let mut case_ids = BTreeSet::new();
        for case in &self.cases {
            validate_identifier(&case.id, "evaluation case id")?;
            ensure!(
                case_ids.insert(case.id.clone()),
                "duplicate evaluation case id '{}'",
                case.id
            );
            ensure!(
                !case.prompt.trim().is_empty(),
                "case '{}' prompt cannot be empty",
                case.id
            );
            if let Some(fixture) = &case.fixture {
                validate_relative_path(fixture, "fixture path")?;
            }
            for tag in &case.tags {
                ensure!(
                    !tag.trim().is_empty() && tag.trim() == tag,
                    "case tags must be non-empty and trimmed"
                );
            }
            ensure!(
                !case.graders.is_empty(),
                "case '{}' must define at least one grader",
                case.id
            );
            ensure!(
                case.graders.len() <= MAX_GRADERS_PER_CASE,
                "case '{}' exceeds the {MAX_GRADERS_PER_CASE}-grader limit",
                case.id
            );
            let mut grader_ids = BTreeSet::new();
            for grader in &case.graders {
                grader.validate()?;
                ensure!(
                    grader_ids.insert(grader.id().to_string()),
                    "duplicate grader id '{}' in case '{}'",
                    grader.id(),
                    case.id
                );
            }
        }
        Ok(())
    }
}

/// Reject absolute, empty, dot, parent, and platform-specific paths.
pub fn validate_relative_path(value: &str, label: &str) -> Result<()> {
    ensure!(!value.is_empty(), "{label} cannot be empty");
    ensure!(
        value.len() <= MAX_ARTIFACT_PATH_BYTES,
        "{label} exceeds {MAX_ARTIFACT_PATH_BYTES} bytes"
    );
    ensure!(
        !value.starts_with('/')
            && !value.as_bytes().contains(&92)
            && !value.as_bytes().contains(&0),
        "{label} must be a portable relative path: '{value}'"
    );
    ensure!(
        !value.contains(':'),
        "{label} must be a portable relative path: '{value}'"
    );
    ensure!(
        !Path::new(value).is_absolute(),
        "{label} must be relative: '{value}'"
    );
    for component in value.split('/') {
        ensure!(
            !component.is_empty() && component != "." && component != "..",
            "{label} contains an unsafe path component: '{value}'"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const MINIMAL_SUITE: &str = r#"
schema_version = 1
suite_id = "answer-check"
repetitions = 2
artifact = { name = "example-skill", sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" }

[[cases]]
id = "answers-question"
prompt = "What is 6 times 7?"

[[cases.graders]]
id = "contains-answer"
kind = "text_contains"
needle = "42"
weight = 1
"#;

    #[test]
    fn parses_and_roundtrips_minimal_versioned_suite() {
        let suite = parse_suite_toml(MINIMAL_SUITE).unwrap();
        assert_eq!(suite.schema_version, 1);
        assert_eq!(suite.suite_id, "answer-check");
        assert_eq!(suite.repetitions, 2);
        let encoded = toml::to_string(&suite).unwrap();
        assert_eq!(parse_suite_toml(&encoded).unwrap(), suite);
    }

    #[test]
    fn loads_suite_from_an_on_disk_toml_file() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("suite.toml");
        fs::write(&path, MINIMAL_SUITE).unwrap();
        let suite = load_suite_file(&path).unwrap();
        assert_eq!(suite.suite_id, "answer-check");
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let input = MINIMAL_SUITE.replace("schema_version = 1", "schema_version = 2");
        assert!(
            parse_suite_toml(&input)
                .unwrap_err()
                .to_string()
                .contains("unsupported")
        );
    }

    #[test]
    fn rejects_malformed_artifact_hash() {
        let input = MINIMAL_SUITE.replace(&"a".repeat(64), "not-a-hash");
        assert!(parse_suite_toml(&input).is_err());
    }

    #[test]
    fn rejects_duplicate_case_ids() {
        let input = r#"
schema_version = 1
suite_id = "answer-check"
artifact = { name = "example-skill", sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" }
[[cases]]
id = "same"
prompt = "first"
[[cases.graders]]
id = "g1"
kind = "text_contains"
needle = "one"
[[cases]]
id = "same"
prompt = "second"
[[cases.graders]]
id = "g2"
kind = "text_contains"
needle = "two"
"#;
        assert!(
            parse_suite_toml(input)
                .unwrap_err()
                .to_string()
                .contains("duplicate evaluation case id")
        );
    }

    #[test]
    fn rejects_unsafe_fixture_paths() {
        for path in [
            "../outside",
            "/absolute",
            "fixtures/../secret",
            "fixtures\\outside",
            "C:/outside",
        ] {
            assert!(
                validate_relative_path(path, "fixture path").is_err(),
                "accepted {path:?}"
            );
        }
    }

    #[test]
    fn rejects_invalid_regex_and_duplicate_grader_ids() {
        let bad_regex = MINIMAL_SUITE
            .replace(r#"kind = "text_contains""#, r#"kind = "text_regex""#)
            .replace(r#"needle = "42""#, r#"pattern = "[""#);
        assert!(parse_suite_toml(&bad_regex).is_err());

        let duplicate = r#"
schema_version = 1
suite_id = "answer-check"
artifact = { name = "example-skill", sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" }
[[cases]]
id = "case"
prompt = "prompt"
[[cases.graders]]
id = "same"
kind = "text_contains"
needle = "one"
[[cases.graders]]
id = "same"
kind = "text_contains"
needle = "two"
"#;
        assert!(
            parse_suite_toml(duplicate)
                .unwrap_err()
                .to_string()
                .contains("duplicate grader id")
        );
    }

    #[test]
    fn rejects_unknown_grader_fields_instead_of_ignoring_typos() {
        let input = MINIMAL_SUITE.replace(r#"needle = "42""#, "needle = \"42\"\nweigth = 9");
        assert!(parse_suite_toml(&input).is_err());
    }
}
