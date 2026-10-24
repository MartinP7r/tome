//! Deterministic test adapter and workspace provider; no subprocesses or model calls.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use super::contract::{ObservationCapability, RunObservation, RunnerIdentity, Variant};
use super::runner::{RunKey, RunRequest, RunnerAdapter, WorkspaceProvider};

/// A map-backed adapter for deterministic unit and integration tests.
#[derive(Clone, Debug)]
pub struct DeterministicFakeAdapter {
    identity: RunnerIdentity,
    capabilities: BTreeSet<ObservationCapability>,
    observations: BTreeMap<RunKey, RunObservation>,
    invocations: Vec<RunKey>,
    write_marker: bool,
}

impl DeterministicFakeAdapter {
    pub fn new(
        identity: RunnerIdentity,
        capabilities: impl IntoIterator<Item = ObservationCapability>,
    ) -> Self {
        Self {
            identity,
            capabilities: capabilities.into_iter().collect(),
            observations: BTreeMap::new(),
            invocations: Vec::new(),
            write_marker: false,
        }
    }

    pub fn insert_observation(&mut self, key: RunKey, observation: RunObservation) {
        self.observations.insert(key, observation);
    }

    /// Write a marker into the disposable workspace to test source isolation.
    pub fn set_write_marker(&mut self, enabled: bool) {
        self.write_marker = enabled;
    }

    pub fn invocations(&self) -> &[RunKey] {
        &self.invocations
    }
}

impl RunnerAdapter for DeterministicFakeAdapter {
    fn identity(&self) -> &RunnerIdentity {
        &self.identity
    }

    fn capabilities(&self) -> &BTreeSet<ObservationCapability> {
        &self.capabilities
    }

    fn run(&mut self, request: &RunRequest, workspace: &Path) -> Result<RunObservation> {
        let key = request.key();
        self.invocations.push(key.clone());
        if self.write_marker {
            fs::write(workspace.join("fake-adapter-marker"), b"disposable")
                .context("failed to write fake adapter marker")?;
        }
        let Some(mut observation) = self.observations.get(&key).cloned() else {
            bail!("no deterministic fake observation for run key");
        };
        if observation.model_pin.is_none() {
            observation.model_pin = request.model_pin.clone();
        }
        if observation.execution_policy.is_none() {
            observation.execution_policy = Some(request.policy.clone());
        }
        Ok(observation)
    }
}

/// Creates unique empty run directories below a caller-owned temporary root.
#[derive(Clone, Debug)]
pub struct FreshWorkspaceProvider {
    root: PathBuf,
}

impl FreshWorkspaceProvider {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl WorkspaceProvider for FreshWorkspaceProvider {
    fn workspace_path(&mut self, key: &RunKey) -> Result<PathBuf> {
        let variant = match key.variant {
            Variant::Control => "control",
            Variant::Treatment => "treatment",
        };
        let name = format!("{}-{variant}-{}", key.case_id, key.repetition);
        Ok(self.root.join(name))
    }
}
