//! Shared types for provider-neutral evaluation requests and observations.

use std::collections::{BTreeMap, BTreeSet};

use crate::validation::ContentHash;
use serde::{Deserialize, Serialize};

/// The side of a matched treatment/control evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Variant {
    /// The baseline run without the skill artifact.
    Control,
    /// The run with a copy of the skill artifact.
    Treatment,
}

impl Variant {
    /// Stable order used when materializing and serializing runs.
    pub const ALL: [Self; 2] = [Self::Control, Self::Treatment];
}

/// Adapter identity. Values are stable identifiers, never credential material.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunnerIdentity {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// A declared provider/model/prompt pin. It contains identifiers, never credentials.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelPin {
    pub provider: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_version: Option<String>,
}

/// Limits and isolation requirements common to both variants.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ExecutionPolicy {
    /// Maximum wall-clock duration requested for a future adapter, in milliseconds.
    pub timeout_ms: u64,
    /// Maximum normalized output size accepted from a future adapter.
    pub max_output_bytes: u64,
    /// Whether the adapter may use network access.
    pub network: NetworkPolicy,
    /// Required workspace isolation mode.
    pub isolation: IsolationMode,
    /// Named permissions requested by the suite; no permission is implied.
    pub permissions: BTreeSet<String>,
    /// Result retention defaults to hashes and summaries, not full response text.
    pub retention: RetentionPolicy,
}

impl Default for ExecutionPolicy {
    fn default() -> Self {
        Self {
            timeout_ms: 120_000,
            max_output_bytes: 1_048_576,
            network: NetworkPolicy::Disabled,
            isolation: IsolationMode::FreshWorkspace,
            permissions: BTreeSet::new(),
            retention: RetentionPolicy::SummaryOnly,
        }
    }
}

/// Network policy recorded in the suite and each run's provenance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkPolicy {
    #[default]
    Disabled,
    Enabled,
}

/// Isolation policy recorded for comparability. Only fresh workspaces are supported now.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IsolationMode {
    #[default]
    FreshWorkspace,
}

/// Controls whether raw final response text is retained in JSON result documents.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetentionPolicy {
    /// Retain a content hash and grader evidence, but omit the raw response.
    #[default]
    SummaryOnly,
    /// Explicitly retain the normalized final response in the result document.
    FullTrace,
}

/// Evidence a runner can provide to deterministic graders.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationCapability {
    FinalText,
    ArtifactManifest,
    Events,
    Activation,
}

/// Terminal state returned by an adapter. Failures remain data, not passes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalState {
    Completed,
    TimedOut,
    PermissionDenied,
    RunnerError,
    CostLimited,
    Cancelled,
}

/// Whether normalized activation evidence says the skill became active.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivationState {
    Activated,
    NotActivated,
}

/// A normalized event category; raw arguments and provider payloads are not stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    ToolCall,
    Command,
    FileWrite,
    Other,
}

/// A minimal, provider-neutral event observation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NormalizedEvent {
    pub kind: EventKind,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub succeeded: Option<bool>,
}

/// A safe, closed set of adapter failures. Free-form error messages are omitted
/// because they can accidentally contain prompts, paths, or credentials.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    AdapterError,
    InvalidObservation,
}

/// Normalized evidence returned by an adapter. Raw final text is ephemeral here;
/// result serialization applies the suite's retention policy before persistence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunObservation {
    pub terminal_state: TerminalState,
    pub capabilities: BTreeSet<ObservationCapability>,
    pub final_text: Option<String>,
    pub artifact_manifest: BTreeMap<String, ContentHash>,
    pub events: Vec<NormalizedEvent>,
    pub activation: Option<ActivationState>,
    pub model_pin: Option<ModelPin>,
    pub execution_policy: Option<ExecutionPolicy>,
    pub failure: Option<FailureKind>,
}

impl RunObservation {
    /// Create an empty completed observation for adapters and tests to populate.
    pub fn completed() -> Self {
        Self {
            terminal_state: TerminalState::Completed,
            capabilities: BTreeSet::new(),
            final_text: None,
            artifact_manifest: BTreeMap::new(),
            events: Vec::new(),
            activation: None,
            model_pin: None,
            execution_policy: None,
            failure: None,
        }
    }

    /// Create a safe adapter-failure observation without retaining its error text.
    pub fn adapter_error() -> Self {
        Self {
            terminal_state: TerminalState::RunnerError,
            failure: Some(FailureKind::AdapterError),
            ..Self::completed()
        }
    }
}
