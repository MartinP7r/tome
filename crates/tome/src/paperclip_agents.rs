//! Paperclip agent desired-skill materialization.
//!
//! This module is deliberately separate from the generic sync/distribution
//! pipeline. Filesystem target deployment must never mutate Paperclip agents as
//! a side effect; callers enter this workflow explicitly through the
//! `paperclip-agents` command.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const CONFIRMATION_TOKEN_PREFIX: &str = "apply-v2-";
const CONFIRMATION_TOKEN_TTL_SECS: u64 = 15 * 60;
const CONFIRMATION_STORE_FILENAME: &str = ".paperclip-agent-confirmations.json";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub(crate) struct SkillRef(String);

impl SkillRef {
    fn new(value: impl Into<String>, field: &str) -> Result<Self> {
        let value = value.into();
        anyhow::ensure!(!value.trim().is_empty(), "{field} must not be empty");
        Ok(Self(value))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SkillRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub(crate) struct AgentId(String);

impl AgentId {
    fn new(value: impl Into<String>, field: &str) -> Result<Self> {
        let value = value.into();
        anyhow::ensure!(!value.trim().is_empty(), "{field} must not be empty");
        Ok(Self(value))
    }

    fn as_str(&self) -> &str {
        &self.0
    }

    fn api_path_segment(&self) -> String {
        encode_path_segment(self.as_str())
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CanonicalCatalog {
    revision: String,
    #[serde(default)]
    categories: BTreeMap<String, Category>,
    #[serde(default)]
    skills: BTreeMap<String, CatalogSkill>,
    #[serde(default, alias = "sets")]
    use_case_sets: BTreeMap<String, UseCaseSet>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Category {
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogSkill {
    /// Canonical Paperclip company skill key/id/unique slug to send to the API.
    company_skill: String,
    #[serde(default)]
    categories: BTreeSet<String>,
    /// Empty means target-agnostic. Non-empty must be satisfied by the
    /// selected effective constraints for the agent.
    #[serde(default)]
    constraints: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct UseCaseSet {
    #[serde(default)]
    skills: BTreeSet<String>,
    #[serde(default)]
    categories: BTreeSet<String>,
    #[serde(default)]
    exclude: BTreeSet<String>,
    /// Empty means target-agnostic. Non-empty must be a subset of the selected
    /// effective constraints for the agent.
    #[serde(default)]
    constraints: BTreeSet<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AssignmentFile {
    #[serde(default)]
    agents: Vec<AgentAssignment>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentAssignment {
    agent_id: String,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    sets: BTreeSet<String>,
    #[serde(default)]
    exclude: BTreeSet<String>,
    /// Agent-specific additions to the command-selected effective constraints.
    #[serde(default)]
    constraints: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AgentStateFile {
    #[serde(default)]
    agents: Vec<AgentStateRecord>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentStateRecord {
    agent_id: String,
    #[serde(default)]
    desired_skills: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    runtime_skills: Option<BTreeSet<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentSkillState {
    desired_skills: BTreeSet<SkillRef>,
    runtime_skills: Option<BTreeSet<SkillRef>>,
}

impl AgentSkillState {
    fn from_record(record: &AgentStateRecord) -> Result<Self> {
        Ok(Self {
            desired_skills: parse_skill_set(&record.desired_skills, "desired_skills")?,
            runtime_skills: record
                .runtime_skills
                .as_ref()
                .map(|skills| parse_skill_set(skills, "runtime_skills"))
                .transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct MaterializationPlan {
    catalog_revision: String,
    selected_constraints: BTreeSet<String>,
    agents: Vec<AgentPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct AgentPlan {
    agent_id: AgentId,
    display_name: Option<String>,
    desired_skills: BTreeSet<SkillRef>,
    reasons: BTreeMap<SkillRef, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentImpact {
    agent_id: AgentId,
    display_name: Option<String>,
    before_desired: BTreeSet<SkillRef>,
    intended_desired: BTreeSet<SkillRef>,
    add: BTreeSet<SkillRef>,
    remove: BTreeSet<SkillRef>,
    keep: BTreeSet<SkillRef>,
    before_runtime_mismatch: Option<SkillMismatch>,
    after_desired_mismatch: Option<SkillMismatch>,
    after_runtime_mismatch: Option<SkillMismatch>,
    after_runtime_unavailable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillMismatch {
    missing: BTreeSet<SkillRef>,
    extra: BTreeSet<SkillRef>,
}

impl SkillMismatch {
    fn between(expected: &BTreeSet<SkillRef>, actual: &BTreeSet<SkillRef>) -> Option<Self> {
        let missing = expected
            .difference(actual)
            .cloned()
            .collect::<BTreeSet<_>>();
        let extra = actual
            .difference(expected)
            .cloned()
            .collect::<BTreeSet<_>>();
        (!missing.is_empty() || !extra.is_empty()).then_some(Self { missing, extra })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreviewReport {
    plan: MaterializationPlan,
    confirmation_token: String,
    impacts: Vec<AgentImpact>,
}

impl PreviewReport {
    pub(crate) fn confirmation_token(&self) -> &str {
        &self.confirmation_token
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ApplyReport {
    preview: PreviewReport,
    outcomes: Vec<AgentApplyOutcome>,
}

impl ApplyReport {
    pub(crate) fn failure_messages(&self) -> Vec<String> {
        let mut failures = Vec::new();
        for outcome in &self.outcomes {
            if let Some(error) = &outcome.sync_error {
                failures.push(format!(
                    "agent '{}' desired-skill sync failed: {error}",
                    outcome.agent_id
                ));
            }
            if let Some(error) = &outcome.readback_error {
                failures.push(format!(
                    "agent '{}' read-back failed: {error}",
                    outcome.agent_id
                ));
                continue;
            }
            if let Some(impact) = &outcome.readback_impact {
                if impact.after_desired_mismatch.is_some() {
                    failures.push(format!(
                        "agent '{}' desired skills differ after read-back",
                        outcome.agent_id
                    ));
                }
                if impact.after_runtime_mismatch.is_some() {
                    failures.push(format!(
                        "agent '{}' runtime skills differ after read-back",
                        outcome.agent_id
                    ));
                }
                if impact.after_runtime_unavailable {
                    failures.push(format!(
                        "agent '{}' runtime skills were not returned on read-back",
                        outcome.agent_id
                    ));
                }
            }
        }
        failures
    }

    pub(crate) fn has_failures(&self) -> bool {
        !self.failure_messages().is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentApplyOutcome {
    agent_id: AgentId,
    display_name: Option<String>,
    preview_impact: AgentImpact,
    sync_error: Option<String>,
    readback_error: Option<String>,
    readback_impact: Option<AgentImpact>,
}

pub(crate) trait PaperclipAgentSkillBoundary {
    fn read_agent_skills(&mut self, agent_id: &AgentId) -> Result<AgentSkillState>;
    fn sync_agent_desired_skills(
        &mut self,
        agent_id: &AgentId,
        desired_skills: &[SkillRef],
    ) -> Result<()>;
}

pub(crate) trait ConfirmationTokenStore {
    fn issue(
        &mut self,
        plan: &MaterializationPlan,
        before_states: &BTreeMap<AgentId, AgentSkillState>,
    ) -> Result<String>;

    fn consume(&mut self, token: &str, plan: &MaterializationPlan) -> Result<StoredConfirmation>;
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfirmationStoreDocument {
    #[serde(default)]
    tokens: BTreeMap<String, StoredConfirmation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StoredConfirmation {
    issued_at_epoch_secs: u64,
    expires_at_epoch_secs: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    consumed_at_epoch_secs: Option<u64>,
    plan_fingerprint: String,
    before_fingerprint: String,
    agents: Vec<StoredAgentConfirmation>,
}

impl StoredConfirmation {
    fn before_states(&self) -> Result<BTreeMap<AgentId, AgentSkillState>> {
        let mut states = BTreeMap::new();
        for agent in &self.agents {
            let agent_id = AgentId::new(agent.agent_id.clone(), "confirmation agent_id")?;
            anyhow::ensure!(
                states
                    .insert(agent_id.clone(), agent.state.to_agent_state()?)
                    .is_none(),
                "confirmation token contains duplicate agent '{}'",
                agent_id
            );
        }
        Ok(states)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredAgentConfirmation {
    agent_id: String,
    state: StoredAgentSkillState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredAgentSkillState {
    desired_skills: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    runtime_skills: Option<BTreeSet<String>>,
}

impl StoredAgentSkillState {
    fn from_agent_state(state: &AgentSkillState) -> Self {
        Self {
            desired_skills: state
                .desired_skills
                .iter()
                .map(ToString::to_string)
                .collect(),
            runtime_skills: state.runtime_skills.as_ref().map(|skills| {
                skills
                    .iter()
                    .map(ToString::to_string)
                    .collect::<BTreeSet<_>>()
            }),
        }
    }

    fn to_agent_state(&self) -> Result<AgentSkillState> {
        Ok(AgentSkillState {
            desired_skills: parse_skill_set(&self.desired_skills, "confirmation desired_skills")?,
            runtime_skills: self
                .runtime_skills
                .as_ref()
                .map(|skills| parse_skill_set(skills, "confirmation runtime_skills"))
                .transpose()?,
        })
    }
}

pub(crate) struct FileConfirmationTokenStore {
    path: PathBuf,
}

impl FileConfirmationTokenStore {
    pub(crate) fn new(config_dir: &Path) -> Self {
        Self {
            path: config_dir.join(CONFIRMATION_STORE_FILENAME),
        }
    }

    fn load(&self) -> Result<ConfirmationStoreDocument> {
        if !self.path.exists() {
            return Ok(ConfirmationStoreDocument::default());
        }
        let text = std::fs::read_to_string(&self.path)
            .with_context(|| format!("failed to read {}", self.path.display()))?;
        serde_json::from_str(&text)
            .with_context(|| format!("failed to parse {}", self.path.display()))
    }

    fn save(&self, document: &ConfirmationStoreDocument) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let tmp = self.path.with_extension("json.tmp");
        let bytes = serde_json::to_vec_pretty(document)
            .context("failed to serialize Paperclip confirmation store")?;
        std::fs::write(&tmp, bytes)
            .with_context(|| format!("failed to write {}", tmp.display()))?;
        std::fs::rename(&tmp, &self.path).with_context(|| {
            format!(
                "failed to replace {} with {}",
                self.path.display(),
                tmp.display()
            )
        })?;
        Ok(())
    }
}

impl ConfirmationTokenStore for FileConfirmationTokenStore {
    fn issue(
        &mut self,
        plan: &MaterializationPlan,
        before_states: &BTreeMap<AgentId, AgentSkillState>,
    ) -> Result<String> {
        let mut document = self.load()?;
        let issued_at_epoch_secs = current_unix_secs()?;
        let expires_at_epoch_secs =
            issued_at_epoch_secs.saturating_add(CONFIRMATION_TOKEN_TTL_SECS);
        let plan_fingerprint = plan_fingerprint(plan)?;
        let agents = stored_confirmation_agents(before_states);
        let before_fingerprint = before_fingerprint(&agents)?;
        let token = loop {
            let token = format!("{CONFIRMATION_TOKEN_PREFIX}{}", random_token_hex()?);
            if !document.tokens.contains_key(&token) {
                break token;
            }
        };
        document.tokens.insert(
            token.clone(),
            StoredConfirmation {
                issued_at_epoch_secs,
                expires_at_epoch_secs,
                consumed_at_epoch_secs: None,
                plan_fingerprint,
                before_fingerprint,
                agents,
            },
        );
        self.save(&document)?;
        Ok(token)
    }

    fn consume(&mut self, token: &str, plan: &MaterializationPlan) -> Result<StoredConfirmation> {
        let mut document = self.load()?;
        let expected_plan_fingerprint = plan_fingerprint(plan)?;
        let now = current_unix_secs()?;
        let record = document
            .tokens
            .get_mut(token)
            .with_context(|| "confirmation token was not issued by preview; run preview again")?;
        anyhow::ensure!(
            record.consumed_at_epoch_secs.is_none(),
            "confirmation token has already been used; run preview again"
        );
        anyhow::ensure!(
            now <= record.expires_at_epoch_secs,
            "confirmation token has expired; run preview again"
        );
        anyhow::ensure!(
            record.plan_fingerprint == expected_plan_fingerprint,
            "confirmation token does not match this immutable plan; run preview again"
        );
        let agents = record.agents.clone();
        anyhow::ensure!(
            record.before_fingerprint == before_fingerprint(&agents)?,
            "confirmation token's before-state snapshot is invalid; run preview again"
        );
        record.consumed_at_epoch_secs = Some(now);
        let consumed = record.clone();
        self.save(&document)?;
        Ok(consumed)
    }
}

pub(crate) struct StateFileBoundary {
    states: BTreeMap<AgentId, AgentSkillState>,
}

impl StateFileBoundary {
    pub(crate) fn from_path(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let state_file: AgentStateFile = serde_json::from_str(&text)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        Self::from_state_file(&state_file)
    }

    fn from_state_file(state_file: &AgentStateFile) -> Result<Self> {
        let mut states = BTreeMap::new();
        for record in &state_file.agents {
            let agent_id = AgentId::new(record.agent_id.clone(), "state agent_id")?;
            anyhow::ensure!(
                states
                    .insert(agent_id.clone(), AgentSkillState::from_record(record)?)
                    .is_none(),
                "duplicate state entry for agent '{}'",
                agent_id
            );
        }
        Ok(Self { states })
    }
}

impl PaperclipAgentSkillBoundary for StateFileBoundary {
    fn read_agent_skills(&mut self, agent_id: &AgentId) -> Result<AgentSkillState> {
        self.states.get(agent_id).cloned().with_context(|| {
            format!("current-state is missing required state for assigned agent '{agent_id}'")
        })
    }

    fn sync_agent_desired_skills(
        &mut self,
        _agent_id: &AgentId,
        _desired_skills: &[SkillRef],
    ) -> Result<()> {
        bail!("state-file boundary is preview-only and cannot mutate Paperclip agents")
    }
}

pub(crate) struct CurlPaperclipBoundary {
    base_url: String,
    api_key: String,
}

impl CurlPaperclipBoundary {
    pub(crate) fn new(base_url: String, api_key: String) -> Result<Self> {
        anyhow::ensure!(
            !base_url.trim().is_empty(),
            "Paperclip API URL must not be empty"
        );
        anyhow::ensure!(
            !api_key.trim().is_empty(),
            "Paperclip API key must not be empty"
        );
        let base_url = normalize_api_base(&base_url);
        Ok(Self { base_url, api_key })
    }

    fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let config = self.curl_config(method, path, body)?;

        let mut child = Command::new("curl")
            .arg("--config")
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("failed to run curl for Paperclip request {method} {path}"))?;
        {
            let stdin = child.stdin.as_mut().context("failed to open curl stdin")?;
            stdin
                .write_all(config.as_bytes())
                .context("failed to write curl config")?;
        }
        let output = child.wait_with_output().with_context(|| {
            format!("curl did not finish for Paperclip request {method} {path}")
        })?;
        if !output.status.success() {
            let stderr = self.redact_api_key(&String::from_utf8_lossy(&output.stderr));
            bail!("curl failed for Paperclip request {method} {path}: {stderr}");
        }
        let text = String::from_utf8(output.stdout).context("Paperclip response was not UTF-8")?;
        let (body_text, status_text) = text
            .rsplit_once('\n')
            .context("Paperclip response did not include HTTP status")?;
        let status: u16 = status_text
            .trim()
            .parse()
            .context("Paperclip response had invalid HTTP status")?;
        if !(200..300).contains(&status) {
            let body_text = self.redact_api_key(body_text);
            bail!("Paperclip request {method} {path} failed with HTTP {status}: {body_text}");
        }
        if body_text.trim().is_empty() {
            return Ok(serde_json::Value::Null);
        }
        serde_json::from_str(body_text)
            .with_context(|| format!("failed to parse Paperclip response for {method} {path}"))
    }

    fn curl_config(
        &self,
        method: &str,
        path: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<String> {
        let url = format!("{}{}", self.base_url, path);
        let mut config = String::new();
        config.push_str("silent\n");
        config.push_str("show-error\n");
        config.push_str(&format!("request = \"{}\"\n", curl_config_escape(method)));
        config.push_str(&format!("url = \"{}\"\n", curl_config_escape(&url)));
        config.push_str(&format!(
            "header = \"{}\"\n",
            curl_config_escape(&format!("Authorization: Bearer {}", self.api_key))
        ));
        config.push_str("header = \"Content-Type: application/json\"\n");
        config.push_str("write-out = \"\\n%{http_code}\"\n");
        if let Some(body) = body {
            config.push_str(&format!(
                "data = \"{}\"\n",
                curl_config_escape(
                    &serde_json::to_string(body).context("failed to serialize Paperclip body")?
                )
            ));
        }
        Ok(config)
    }

    fn redact_api_key(&self, text: &str) -> String {
        redact_secret(text, &self.api_key)
    }
}

fn curl_config_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(c),
        }
    }
    escaped
}

impl PaperclipAgentSkillBoundary for CurlPaperclipBoundary {
    fn read_agent_skills(&mut self, agent_id: &AgentId) -> Result<AgentSkillState> {
        let value = self.request("GET", &agent_skills_path(agent_id), None)?;
        parse_agent_skill_state(agent_id, &value)
    }

    fn sync_agent_desired_skills(
        &mut self,
        agent_id: &AgentId,
        desired_skills: &[SkillRef],
    ) -> Result<()> {
        let desired = desired_skills
            .iter()
            .map(|skill| skill.as_str())
            .collect::<Vec<_>>();
        let body = serde_json::json!({
            "mode": "replace",
            "desiredSkills": desired,
        });
        self.request("POST", &agent_skills_sync_path(agent_id), Some(&body))?;
        Ok(())
    }
}

pub(crate) fn load_catalog(path: &Path) -> Result<CanonicalCatalog> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))
}

pub(crate) fn load_assignments(path: &Path) -> Result<AssignmentFile> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))
}

pub(crate) fn resolve_plan(
    catalog: &CanonicalCatalog,
    assignments: &AssignmentFile,
    selected_constraints: &[String],
) -> Result<MaterializationPlan> {
    validate_catalog(catalog)?;
    anyhow::ensure!(
        !assignments.agents.is_empty(),
        "assignments must include at least one agent"
    );
    let selected_constraints = selected_constraints
        .iter()
        .map(|value| validate_label(value, "constraint"))
        .collect::<Result<BTreeSet<_>>>()?;

    let mut seen_agents = BTreeSet::new();
    let mut agents = Vec::new();
    for assignment in &assignments.agents {
        let agent_id = AgentId::new(assignment.agent_id.clone(), "agent_id")?;
        anyhow::ensure!(
            seen_agents.insert(agent_id.clone()),
            "duplicate assignment for agent '{}'",
            agent_id
        );
        anyhow::ensure!(
            !assignment.sets.is_empty(),
            "agent '{}' must select at least one use-case set",
            agent_id
        );

        let agent_constraints = assignment
            .constraints
            .iter()
            .map(|value| validate_label(value, "agent constraint"))
            .collect::<Result<BTreeSet<_>>>()?;
        let effective_constraints = selected_constraints
            .union(&agent_constraints)
            .cloned()
            .collect::<BTreeSet<_>>();

        let mut candidates: BTreeSet<String> = BTreeSet::new();
        let mut excluded: BTreeSet<String> = assignment.exclude.clone();
        let mut reasons: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

        for set_id in &assignment.sets {
            validate_label(set_id, "use-case set id")?;
            let set = catalog
                .use_case_sets
                .get(set_id)
                .with_context(|| format!("use-case set '{set_id}' is not in catalog"))?;
            ensure_constraints_satisfied(
                &effective_constraints,
                &set.constraints,
                &format!("use-case set '{set_id}'"),
            )?;
            for skill_id in &set.skills {
                ensure_catalog_skill(catalog, skill_id)?;
                candidates.insert(skill_id.clone());
                reasons
                    .entry(skill_id.clone())
                    .or_default()
                    .insert(format!("set:{set_id}:skill"));
            }
            for category_id in &set.categories {
                ensure_catalog_category(catalog, category_id)?;
                for (skill_id, skill) in &catalog.skills {
                    if skill.categories.contains(category_id) {
                        candidates.insert(skill_id.clone());
                        reasons
                            .entry(skill_id.clone())
                            .or_default()
                            .insert(format!("set:{set_id}:category:{category_id}"));
                    }
                }
            }
            excluded.extend(set.exclude.iter().cloned());
        }

        for excluded_id in &excluded {
            ensure_catalog_skill(catalog, excluded_id)?;
        }
        for excluded_id in excluded {
            candidates.remove(&excluded_id);
            reasons.remove(&excluded_id);
        }

        let mut desired_skills = BTreeSet::new();
        let mut rendered_reasons = BTreeMap::new();
        for skill_id in candidates {
            let skill = catalog
                .skills
                .get(&skill_id)
                .expect("candidate skills were validated");
            ensure_constraints_satisfied(
                &effective_constraints,
                &skill.constraints,
                &format!("skill '{skill_id}'"),
            )?;
            let paperclip_skill = SkillRef::new(skill.company_skill.clone(), "company_skill")?;
            desired_skills.insert(paperclip_skill.clone());
            rendered_reasons.insert(
                paperclip_skill,
                reasons
                    .remove(&skill_id)
                    .unwrap_or_default()
                    .into_iter()
                    .collect(),
            );
        }

        agents.push(AgentPlan {
            agent_id,
            display_name: assignment.display_name.clone(),
            desired_skills,
            reasons: rendered_reasons,
        });
    }
    agents.sort_by(|a, b| a.agent_id.cmp(&b.agent_id));
    Ok(MaterializationPlan {
        catalog_revision: catalog.revision.clone(),
        selected_constraints,
        agents,
    })
}

pub(crate) fn preview_with_boundary(
    plan: MaterializationPlan,
    boundary: &mut dyn PaperclipAgentSkillBoundary,
    confirmation_store: &mut dyn ConfirmationTokenStore,
) -> Result<PreviewReport> {
    let mut before_states = BTreeMap::new();
    for agent in &plan.agents {
        let before = boundary.read_agent_skills(&agent.agent_id)?;
        before_states.insert(agent.agent_id.clone(), before);
    }
    preview_from_before_states(plan, before_states, confirmation_store)
}

pub(crate) fn apply_with_boundary(
    plan: MaterializationPlan,
    boundary: &mut dyn PaperclipAgentSkillBoundary,
    confirm_token: &str,
    confirmation_store: &mut dyn ConfirmationTokenStore,
) -> Result<ApplyReport> {
    let confirmation = confirmation_store.consume(confirm_token, &plan)?;
    let confirmed_before_states = confirmation.before_states()?;
    ensure_plan_agents_match_before_states(&plan, &confirmed_before_states)?;

    let mut current_before_states = BTreeMap::new();
    for agent in &plan.agents {
        let current = boundary.read_agent_skills(&agent.agent_id)?;
        let confirmed = confirmed_before_states
            .get(&agent.agent_id)
            .expect("confirmed before states were checked against the plan");
        anyhow::ensure!(
            current == *confirmed,
            "agent '{}' current skills changed since preview; run preview again before applying",
            agent.agent_id
        );
        current_before_states.insert(agent.agent_id.clone(), current);
    }

    let preview = preview_report_from_before_states(
        plan.clone(),
        confirm_token.to_string(),
        current_before_states.clone(),
    )?;

    let mut outcomes = Vec::new();
    for agent in &plan.agents {
        let desired = agent.desired_skills.iter().cloned().collect::<Vec<_>>();
        let sync_error = boundary
            .sync_agent_desired_skills(&agent.agent_id, &desired)
            .map_err(user_error)
            .err();
        let preview_impact = preview
            .impacts
            .iter()
            .find(|impact| impact.agent_id == agent.agent_id)
            .expect("preview was built from the plan agents")
            .clone();
        outcomes.push(AgentApplyOutcome {
            agent_id: agent.agent_id.clone(),
            display_name: agent.display_name.clone(),
            preview_impact,
            sync_error,
            readback_error: None,
            readback_impact: None,
        });
    }

    for outcome in &mut outcomes {
        let agent = plan
            .agents
            .iter()
            .find(|agent| agent.agent_id == outcome.agent_id)
            .expect("outcome was built from the plan agents");
        let before = current_before_states
            .get(&agent.agent_id)
            .expect("current before state was read for every plan agent");
        match boundary.read_agent_skills(&agent.agent_id) {
            Ok(after) => {
                outcome.readback_impact = Some(impact_for(agent, before, Some(&after)));
            }
            Err(error) => {
                outcome.readback_error = Some(user_error(error));
            }
        }
    }

    Ok(ApplyReport { preview, outcomes })
}

pub(crate) fn render_preview(report: &PreviewReport) -> String {
    let mut out = String::new();
    push_header(&mut out, &report.plan, Some(report.confirmation_token()));
    out.push_str("No Paperclip agent skills were changed.\n");
    render_impacts(&mut out, &report.impacts, false);
    out
}

pub(crate) fn render_apply(report: &ApplyReport) -> String {
    let mut out = String::new();
    push_header(
        &mut out,
        &report.preview.plan,
        Some(report.preview.confirmation_token()),
    );
    out.push_str("Attempted Paperclip agent desired-skill synchronization and read-back.\n");
    render_apply_outcomes(&mut out, &report.outcomes);
    out
}

fn push_header(out: &mut String, plan: &MaterializationPlan, confirmation_token: Option<&str>) {
    out.push_str("Paperclip agent skill materialization\n");
    out.push_str(&format!("Catalog revision: {}\n", plan.catalog_revision));
    let constraints = if plan.selected_constraints.is_empty() {
        "(none)".to_string()
    } else {
        plan.selected_constraints
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    };
    out.push_str(&format!("Effective constraints: {constraints}\n"));
    if let Some(token) = confirmation_token {
        out.push_str(&format!("Confirmation token: {token}\n"));
    }
    out.push('\n');
}

fn render_apply_outcomes(out: &mut String, outcomes: &[AgentApplyOutcome]) {
    for outcome in outcomes {
        let name = outcome
            .display_name
            .as_deref()
            .map(|name| format!("{name} ({})", outcome.agent_id))
            .unwrap_or_else(|| outcome.agent_id.to_string());
        out.push_str(&format!("Agent {name}\n"));
        match &outcome.sync_error {
            Some(error) => out.push_str(&format!("  sync: failed: {error}\n")),
            None => out.push_str("  sync: succeeded\n"),
        }
        match &outcome.readback_error {
            Some(error) => out.push_str(&format!("  read-back: failed: {error}\n")),
            None => out.push_str("  read-back: succeeded\n"),
        }
        let impact = outcome
            .readback_impact
            .as_ref()
            .unwrap_or(&outcome.preview_impact);
        render_impact_body(out, impact, outcome.readback_impact.is_some());
    }
}

fn render_impacts(out: &mut String, impacts: &[AgentImpact], include_readback: bool) {
    for impact in impacts {
        render_single_impact(out, impact, include_readback);
    }
}

fn render_single_impact(out: &mut String, impact: &AgentImpact, include_readback: bool) {
    let name = impact
        .display_name
        .as_deref()
        .map(|name| format!("{name} ({})", impact.agent_id))
        .unwrap_or_else(|| impact.agent_id.to_string());
    out.push_str(&format!("Agent {name}\n"));
    render_impact_body(out, impact, include_readback);
}

fn render_impact_body(out: &mut String, impact: &AgentImpact, include_readback: bool) {
    out.push_str(&format!(
        "  before desired: {}\n",
        render_set(&impact.before_desired)
    ));
    out.push_str(&format!(
        "  intended desired: {}\n",
        render_set(&impact.intended_desired)
    ));
    out.push_str(&format!("  add: {}\n", render_set(&impact.add)));
    out.push_str(&format!("  remove: {}\n", render_set(&impact.remove)));
    out.push_str(&format!("  keep: {}\n", render_set(&impact.keep)));
    if let Some(mismatch) = &impact.before_runtime_mismatch {
        out.push_str(&format!(
            "  before desired/runtime mismatch: {}\n",
            render_mismatch(mismatch)
        ));
    }
    if include_readback {
        if let Some(mismatch) = &impact.after_desired_mismatch {
            out.push_str(&format!(
                "  read-back desired mismatch: {}\n",
                render_mismatch(mismatch)
            ));
        }
        if let Some(mismatch) = &impact.after_runtime_mismatch {
            out.push_str(&format!(
                "  read-back runtime mismatch: {}\n",
                render_mismatch(mismatch)
            ));
        }
        if impact.after_runtime_unavailable {
            out.push_str("  read-back runtime mismatch: runtime skills unavailable\n");
        }
    }
}

fn render_set(set: &BTreeSet<SkillRef>) -> String {
    if set.is_empty() {
        return "(none)".to_string();
    }
    set.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_mismatch(mismatch: &SkillMismatch) -> String {
    format!(
        "missing [{}], extra [{}]",
        render_set(&mismatch.missing),
        render_set(&mismatch.extra)
    )
}

fn impact_for(
    agent: &AgentPlan,
    before: &AgentSkillState,
    after: Option<&AgentSkillState>,
) -> AgentImpact {
    let add = agent
        .desired_skills
        .difference(&before.desired_skills)
        .cloned()
        .collect();
    let remove = before
        .desired_skills
        .difference(&agent.desired_skills)
        .cloned()
        .collect();
    let keep = before
        .desired_skills
        .intersection(&agent.desired_skills)
        .cloned()
        .collect();
    let before_runtime_mismatch = before
        .runtime_skills
        .as_ref()
        .and_then(|runtime| SkillMismatch::between(&before.desired_skills, runtime));
    let (after_desired_mismatch, after_runtime_mismatch, after_runtime_unavailable) =
        if let Some(after) = after {
            (
                SkillMismatch::between(&agent.desired_skills, &after.desired_skills),
                after
                    .runtime_skills
                    .as_ref()
                    .and_then(|runtime| SkillMismatch::between(&agent.desired_skills, runtime)),
                after.runtime_skills.is_none(),
            )
        } else {
            (None, None, false)
        };

    AgentImpact {
        agent_id: agent.agent_id.clone(),
        display_name: agent.display_name.clone(),
        before_desired: before.desired_skills.clone(),
        intended_desired: agent.desired_skills.clone(),
        add,
        remove,
        keep,
        before_runtime_mismatch,
        after_desired_mismatch,
        after_runtime_mismatch,
        after_runtime_unavailable,
    }
}

fn validate_catalog(catalog: &CanonicalCatalog) -> Result<()> {
    validate_label(&catalog.revision, "catalog revision")?;
    anyhow::ensure!(
        !catalog.skills.is_empty(),
        "catalog must include at least one skill"
    );
    anyhow::ensure!(
        !catalog.use_case_sets.is_empty(),
        "catalog must include at least one use-case set"
    );

    let mut company_keys = BTreeSet::new();
    for (skill_id, skill) in &catalog.skills {
        validate_label(skill_id, "skill id")?;
        SkillRef::new(skill.company_skill.clone(), "company_skill")?;
        anyhow::ensure!(
            company_keys.insert(skill.company_skill.clone()),
            "company skill '{}' is assigned to more than one catalog skill",
            skill.company_skill
        );
        for category in &skill.categories {
            ensure_catalog_category(catalog, category)?;
        }
        for constraint in &skill.constraints {
            validate_label(constraint, "skill constraint")?;
        }
    }
    for (category_id, category) in &catalog.categories {
        validate_label(category_id, "category id")?;
        if let Some(description) = &category.description {
            anyhow::ensure!(
                !description.trim().is_empty(),
                "category '{category_id}' description must not be empty when present"
            );
        }
    }
    for (set_id, set) in &catalog.use_case_sets {
        validate_label(set_id, "use-case set id")?;
        anyhow::ensure!(
            !set.skills.is_empty() || !set.categories.is_empty(),
            "use-case set '{set_id}' must include skills or categories"
        );
        for skill_id in &set.skills {
            ensure_catalog_skill(catalog, skill_id)?;
        }
        for skill_id in &set.exclude {
            ensure_catalog_skill(catalog, skill_id)?;
        }
        for category_id in &set.categories {
            ensure_catalog_category(catalog, category_id)?;
        }
        for constraint in &set.constraints {
            validate_label(constraint, "set constraint")?;
        }
    }
    Ok(())
}

fn validate_label(value: &str, field: &str) -> Result<String> {
    anyhow::ensure!(!value.trim().is_empty(), "{field} must not be empty");
    anyhow::ensure!(
        !value.chars().any(char::is_whitespace),
        "{field} '{value}' must not contain whitespace"
    );
    Ok(value.to_string())
}

fn ensure_catalog_skill(catalog: &CanonicalCatalog, skill_id: &str) -> Result<()> {
    anyhow::ensure!(
        catalog.skills.contains_key(skill_id),
        "catalog skill '{skill_id}' is not defined"
    );
    Ok(())
}

fn ensure_catalog_category(catalog: &CanonicalCatalog, category_id: &str) -> Result<()> {
    anyhow::ensure!(
        catalog.categories.contains_key(category_id),
        "catalog category '{category_id}' is not defined"
    );
    Ok(())
}

fn ensure_constraints_satisfied(
    selected: &BTreeSet<String>,
    required: &BTreeSet<String>,
    label: &str,
) -> Result<()> {
    let missing = required
        .difference(selected)
        .cloned()
        .collect::<BTreeSet<_>>();
    anyhow::ensure!(
        missing.is_empty(),
        "{label} requires missing effective constraints: {}",
        missing.into_iter().collect::<Vec<_>>().join(", ")
    );
    Ok(())
}

fn parse_skill_set(values: &BTreeSet<String>, field: &str) -> Result<BTreeSet<SkillRef>> {
    values
        .iter()
        .map(|value| SkillRef::new(value.clone(), field))
        .collect()
}

fn preview_from_before_states(
    plan: MaterializationPlan,
    before_states: BTreeMap<AgentId, AgentSkillState>,
    confirmation_store: &mut dyn ConfirmationTokenStore,
) -> Result<PreviewReport> {
    ensure_plan_agents_match_before_states(&plan, &before_states)?;
    let confirmation_token = confirmation_store.issue(&plan, &before_states)?;
    preview_report_from_before_states(plan, confirmation_token, before_states)
}

fn preview_report_from_before_states(
    plan: MaterializationPlan,
    confirmation_token: String,
    before_states: BTreeMap<AgentId, AgentSkillState>,
) -> Result<PreviewReport> {
    ensure_plan_agents_match_before_states(&plan, &before_states)?;
    let impacts = plan
        .agents
        .iter()
        .map(|agent| {
            let before = before_states
                .get(&agent.agent_id)
                .expect("before states were checked against the plan");
            impact_for(agent, before, None)
        })
        .collect();
    Ok(PreviewReport {
        plan,
        confirmation_token,
        impacts,
    })
}

fn ensure_plan_agents_match_before_states(
    plan: &MaterializationPlan,
    before_states: &BTreeMap<AgentId, AgentSkillState>,
) -> Result<()> {
    for agent in &plan.agents {
        anyhow::ensure!(
            before_states.contains_key(&agent.agent_id),
            "confirmation snapshot is missing agent '{}'",
            agent.agent_id
        );
    }
    for agent_id in before_states.keys() {
        anyhow::ensure!(
            plan.agents.iter().any(|agent| agent.agent_id == *agent_id),
            "confirmation snapshot contains unexpected agent '{agent_id}'"
        );
    }
    Ok(())
}

fn stored_confirmation_agents(
    before_states: &BTreeMap<AgentId, AgentSkillState>,
) -> Vec<StoredAgentConfirmation> {
    before_states
        .iter()
        .map(|(agent_id, state)| StoredAgentConfirmation {
            agent_id: agent_id.to_string(),
            state: StoredAgentSkillState::from_agent_state(state),
        })
        .collect()
}

fn plan_fingerprint(plan: &MaterializationPlan) -> Result<String> {
    serde_json_fingerprint(plan, "plan")
}

fn before_fingerprint(agents: &[StoredAgentConfirmation]) -> Result<String> {
    serde_json_fingerprint(agents, "before-state snapshot")
}

fn serde_json_fingerprint<T: Serialize + ?Sized>(value: &T, label: &str) -> Result<String> {
    let bytes =
        serde_json::to_vec(value).with_context(|| format!("failed to serialize {label}"))?;
    Ok(hex_digest(&bytes))
}

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn random_token_hex() -> Result<String> {
    let mut bytes = [0_u8; 16];
    let urandom_result =
        std::fs::File::open("/dev/urandom").and_then(|mut file| file.read_exact(&mut bytes));
    if urandom_result.is_ok() {
        return Ok(bytes.iter().map(|b| format!("{b:02x}")).collect());
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let fallback = format!("{now}:{}:{:p}", std::process::id(), &bytes);
    Ok(hex_digest(fallback.as_bytes())[..32].to_string())
}

fn current_unix_secs() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| anyhow!("system clock is before UNIX epoch: {error}"))?
        .as_secs())
}

fn user_error(error: anyhow::Error) -> String {
    format!("{error:#}")
}

fn redact_secret(text: &str, secret: &str) -> String {
    if secret.is_empty() {
        return text.to_string();
    }
    text.replace(secret, "***")
}

fn agent_skills_path(agent_id: &AgentId) -> String {
    format!("/api/agents/{}/skills", agent_id.api_path_segment())
}

fn agent_skills_sync_path(agent_id: &AgentId) -> String {
    format!("/api/agents/{}/skills/sync", agent_id.api_path_segment())
}

fn encode_path_segment(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(byte as char);
        } else {
            use std::fmt::Write as _;

            write!(&mut encoded, "%{byte:02X}").expect("writing to String cannot fail");
        }
    }
    encoded
}

fn normalize_api_base(input: &str) -> String {
    let mut value = input.trim_end_matches('/').to_string();
    if value.ends_with("/api") {
        value.truncate(value.len() - "/api".len());
    }
    value
}

fn parse_agent_skill_state(
    agent_id: &AgentId,
    value: &serde_json::Value,
) -> Result<AgentSkillState> {
    let desired_value = find_array(value, &["desiredSkills"])
        .or_else(|| find_array(value, &["desired_skills"]))
        .or_else(|| find_array(value, &["skills", "desired"]))
        .or_else(|| find_array(value, &["data", "desiredSkills"]))
        .or_else(|| find_array(value, &["agent", "desiredSkills"]))
        .with_context(|| {
            format!("Paperclip read-back for agent '{agent_id}' did not include desired skills")
        })?;
    let runtime_value = find_array(value, &["runtimeSkills"])
        .or_else(|| find_array(value, &["runtime_skills"]))
        .or_else(|| find_array(value, &["effectiveSkills"]))
        .or_else(|| find_array(value, &["activeSkills"]))
        .or_else(|| find_array(value, &["skills", "runtime"]))
        .or_else(|| find_array(value, &["data", "runtimeSkills"]))
        .or_else(|| find_array(value, &["agent", "runtimeSkills"]));

    Ok(AgentSkillState {
        desired_skills: parse_json_skill_array(desired_value, "desiredSkills")?,
        runtime_skills: runtime_value
            .map(|runtime| parse_json_skill_array(runtime, "runtimeSkills"))
            .transpose()?,
    })
}

fn find_array<'a>(
    value: &'a serde_json::Value,
    path: &[&str],
) -> Option<&'a Vec<serde_json::Value>> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_array()
}

fn parse_json_skill_array(values: &[serde_json::Value], field: &str) -> Result<BTreeSet<SkillRef>> {
    values
        .iter()
        .map(|value| {
            if let Some(text) = value.as_str() {
                return SkillRef::new(text.to_string(), field);
            }
            for key in ["key", "skillKey", "companySkillKey", "slug", "id", "name"] {
                if let Some(text) = value.get(key).and_then(|v| v.as_str()) {
                    return SkillRef::new(text.to_string(), field);
                }
            }
            bail!("{field} item did not contain a skill key")
        })
        .collect()
}

#[cfg(test)]
pub(crate) mod testing {
    use super::*;

    #[derive(Default)]
    pub(crate) struct MockPaperclipBoundary {
        pub(crate) states: BTreeMap<AgentId, Vec<AgentSkillState>>,
        pub(crate) syncs: Vec<(AgentId, Vec<SkillRef>)>,
        pub(crate) sync_failures: BTreeSet<AgentId>,
    }

    impl MockPaperclipBoundary {
        pub(crate) fn push_state(&mut self, agent_id: &str, state: AgentSkillState) {
            self.states
                .entry(AgentId::new(agent_id, "agent_id").unwrap())
                .or_default()
                .push(state);
        }

        pub(crate) fn fail_sync(&mut self, agent_id: &str) {
            self.sync_failures
                .insert(AgentId::new(agent_id, "agent_id").unwrap());
        }
    }

    impl PaperclipAgentSkillBoundary for MockPaperclipBoundary {
        fn read_agent_skills(&mut self, agent_id: &AgentId) -> Result<AgentSkillState> {
            let states = self
                .states
                .get_mut(agent_id)
                .with_context(|| format!("no mock state for agent '{agent_id}'"))?;
            anyhow::ensure!(
                !states.is_empty(),
                "no remaining mock state for agent '{agent_id}'"
            );
            Ok(states.remove(0))
        }

        fn sync_agent_desired_skills(
            &mut self,
            agent_id: &AgentId,
            desired_skills: &[SkillRef],
        ) -> Result<()> {
            self.syncs.push((agent_id.clone(), desired_skills.to_vec()));
            anyhow::ensure!(
                !self.sync_failures.contains(agent_id),
                "mock sync failure for agent '{agent_id}'"
            );
            Ok(())
        }
    }

    pub(crate) struct MemoryConfirmationTokenStore {
        pub(crate) tokens: BTreeMap<String, StoredConfirmation>,
        pub(crate) now: u64,
        next: u64,
    }

    impl Default for MemoryConfirmationTokenStore {
        fn default() -> Self {
            Self {
                tokens: BTreeMap::new(),
                now: 1_000,
                next: 0,
            }
        }
    }

    impl MemoryConfirmationTokenStore {
        pub(crate) fn expire_token(&mut self, token: &str) {
            self.tokens
                .get_mut(token)
                .expect("test token should exist")
                .expires_at_epoch_secs = self.now.saturating_sub(1);
        }
    }

    impl ConfirmationTokenStore for MemoryConfirmationTokenStore {
        fn issue(
            &mut self,
            plan: &MaterializationPlan,
            before_states: &BTreeMap<AgentId, AgentSkillState>,
        ) -> Result<String> {
            let token = format!("{CONFIRMATION_TOKEN_PREFIX}test-{}", self.next);
            self.next += 1;
            let agents = stored_confirmation_agents(before_states);
            self.tokens.insert(
                token.clone(),
                StoredConfirmation {
                    issued_at_epoch_secs: self.now,
                    expires_at_epoch_secs: self.now.saturating_add(CONFIRMATION_TOKEN_TTL_SECS),
                    consumed_at_epoch_secs: None,
                    plan_fingerprint: plan_fingerprint(plan)?,
                    before_fingerprint: before_fingerprint(&agents)?,
                    agents,
                },
            );
            Ok(token)
        }

        fn consume(
            &mut self,
            token: &str,
            plan: &MaterializationPlan,
        ) -> Result<StoredConfirmation> {
            let expected_plan_fingerprint = plan_fingerprint(plan)?;
            let record = self.tokens.get_mut(token).with_context(
                || "confirmation token was not issued by preview; run preview again",
            )?;
            anyhow::ensure!(
                record.consumed_at_epoch_secs.is_none(),
                "confirmation token has already been used; run preview again"
            );
            anyhow::ensure!(
                self.now <= record.expires_at_epoch_secs,
                "confirmation token has expired; run preview again"
            );
            anyhow::ensure!(
                record.plan_fingerprint == expected_plan_fingerprint,
                "confirmation token does not match this immutable plan; run preview again"
            );
            let agents = record.agents.clone();
            anyhow::ensure!(
                record.before_fingerprint == before_fingerprint(&agents)?,
                "confirmation token's before-state snapshot is invalid; run preview again"
            );
            record.consumed_at_epoch_secs = Some(self.now);
            Ok(record.clone())
        }
    }

    pub(crate) fn state(desired: &[&str], runtime: Option<&[&str]>) -> AgentSkillState {
        AgentSkillState {
            desired_skills: desired
                .iter()
                .map(|skill| SkillRef::new(*skill, "desired").unwrap())
                .collect(),
            runtime_skills: runtime.map(|skills| {
                skills
                    .iter()
                    .map(|skill| SkillRef::new(*skill, "runtime").unwrap())
                    .collect()
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{MemoryConfirmationTokenStore, MockPaperclipBoundary, state};
    use super::*;

    fn catalog() -> CanonicalCatalog {
        toml::from_str(
            r#"
revision = "catalog-2026-10-02"

[categories.engineering]
description = "Engineering work"

[categories.review]
description = "Review work"

[skills.rust]
company_skill = "company/rust"
categories = ["engineering"]
constraints = ["codex"]

[skills.review]
company_skill = "company/review"
categories = ["review"]

[skills.legacy]
company_skill = "company/legacy"
categories = ["engineering"]

[use_case_sets.founding-engineer]
skills = ["review"]
categories = ["engineering"]
exclude = ["legacy"]
constraints = ["paperclip-agent"]
"#,
        )
        .unwrap()
    }

    fn assignments() -> AssignmentFile {
        toml::from_str(
            r#"
[[agents]]
agent_id = "agent-a"
display_name = "FoundingEngineer"
sets = ["founding-engineer"]
constraints = ["codex"]
"#,
        )
        .unwrap()
    }

    fn two_agent_assignments() -> AssignmentFile {
        toml::from_str(
            r#"
[[agents]]
agent_id = "agent-a"
display_name = "FoundingEngineer"
sets = ["founding-engineer"]
constraints = ["codex"]

[[agents]]
agent_id = "agent-b"
display_name = "Reviewer"
sets = ["founding-engineer"]
constraints = ["codex"]
"#,
        )
        .unwrap()
    }

    fn plan() -> MaterializationPlan {
        resolve_plan(
            &catalog(),
            &assignments(),
            &[String::from("paperclip-agent")],
        )
        .unwrap()
    }

    fn preview_token_for(
        plan: MaterializationPlan,
        before: AgentSkillState,
        store: &mut MemoryConfirmationTokenStore,
    ) -> String {
        let mut boundary = MockPaperclipBoundary::default();
        boundary.push_state("agent-a", before);
        preview_with_boundary(plan, &mut boundary, store)
            .unwrap()
            .confirmation_token()
            .to_string()
    }

    #[test]
    fn resolves_sets_categories_exclusions_and_constraints_deterministically() {
        let plan = resolve_plan(
            &catalog(),
            &assignments(),
            &[String::from("paperclip-agent")],
        )
        .unwrap();

        assert_eq!(plan.catalog_revision, "catalog-2026-10-02");
        assert_eq!(plan.selected_constraints.len(), 1);
        let agent = &plan.agents[0];
        assert_eq!(agent.agent_id.as_str(), "agent-a");
        assert_eq!(
            agent
                .desired_skills
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec!["company/review", "company/rust"]
        );
        assert!(
            agent
                .reasons
                .keys()
                .any(|skill| skill.as_str() == "company/rust")
        );
        assert_eq!(plan.agents.len(), 1);
    }

    #[test]
    fn missing_effective_constraint_rejects_before_preview_or_apply() {
        let err = resolve_plan(&catalog(), &assignments(), &[]).unwrap_err();
        assert!(format!("{err:#}").contains("missing effective constraints"));
    }

    #[test]
    fn preview_reads_only_and_renders_before_after_impact() {
        let plan = resolve_plan(
            &catalog(),
            &assignments(),
            &[String::from("paperclip-agent")],
        )
        .unwrap();
        let mut boundary = MockPaperclipBoundary::default();
        boundary.push_state(
            "agent-a",
            state(&["company/old", "company/review"], Some(&["company/old"])),
        );
        let mut store = MemoryConfirmationTokenStore::default();

        let report = preview_with_boundary(plan, &mut boundary, &mut store).unwrap();

        assert!(boundary.syncs.is_empty());
        assert!(
            report
                .confirmation_token()
                .starts_with(CONFIRMATION_TOKEN_PREFIX)
        );
        let text = render_preview(&report);
        assert!(text.contains("No Paperclip agent skills were changed."));
        assert!(text.contains("add: company/rust"));
        assert!(text.contains("remove: company/old"));
        assert!(text.contains("before desired/runtime mismatch"));
    }

    #[test]
    fn apply_requires_matching_confirmation_token_before_syncing() {
        let plan = plan();
        let mut boundary = MockPaperclipBoundary::default();
        let mut store = MemoryConfirmationTokenStore::default();

        let err = apply_with_boundary(plan, &mut boundary, "apply-wrong", &mut store).unwrap_err();
        let error_text = format!("{err:#}");

        assert!(error_text.contains("confirmation token was not issued by preview"));
        assert!(!error_text.contains(CONFIRMATION_TOKEN_PREFIX));
        assert!(boundary.syncs.is_empty());
    }

    #[test]
    fn apply_syncs_then_reads_back_every_agent() {
        let plan = plan();
        let mut store = MemoryConfirmationTokenStore::default();
        let token = preview_token_for(
            plan.clone(),
            state(&["company/review"], Some(&["company/review"])),
            &mut store,
        );
        let mut boundary = MockPaperclipBoundary::default();
        boundary.push_state(
            "agent-a",
            state(&["company/review"], Some(&["company/review"])),
        );
        boundary.push_state(
            "agent-a",
            state(
                &["company/review", "company/rust"],
                Some(&["company/review", "company/rust"]),
            ),
        );

        let report = apply_with_boundary(plan, &mut boundary, &token, &mut store).unwrap();

        assert_eq!(boundary.syncs.len(), 1);
        assert_eq!(report.outcomes.len(), 1);
        assert!(!report.has_failures());
        assert!(
            render_apply(&report)
                .contains("Attempted Paperclip agent desired-skill synchronization")
        );
    }

    #[test]
    fn apply_surfaces_runtime_mismatch_after_readback() {
        let plan = plan();
        let mut store = MemoryConfirmationTokenStore::default();
        let token = preview_token_for(
            plan.clone(),
            state(&["company/review"], Some(&["company/review"])),
            &mut store,
        );
        let mut boundary = MockPaperclipBoundary::default();
        boundary.push_state(
            "agent-a",
            state(&["company/review"], Some(&["company/review"])),
        );
        boundary.push_state(
            "agent-a",
            state(
                &["company/review", "company/rust"],
                Some(&["company/review"]),
            ),
        );

        let report = apply_with_boundary(plan, &mut boundary, &token, &mut store).unwrap();

        assert!(
            report
                .failure_messages()
                .iter()
                .any(|message| message.contains("runtime skills differ"))
        );
    }

    #[test]
    fn state_file_preview_requires_every_assigned_agent_snapshot() {
        let plan = plan();
        let state_file = AgentStateFile::default();
        let mut boundary = StateFileBoundary::from_state_file(&state_file).unwrap();
        let mut store = MemoryConfirmationTokenStore::default();

        let err = preview_with_boundary(plan, &mut boundary, &mut store).unwrap_err();

        assert!(format!("{err:#}").contains("current-state is missing required state"));
    }

    #[test]
    fn apply_rejects_token_bound_to_different_plan() {
        let original_plan = plan();
        let mut store = MemoryConfirmationTokenStore::default();
        let token = preview_token_for(
            original_plan,
            state(&["company/review"], Some(&["company/review"])),
            &mut store,
        );
        let mut changed_plan = plan();
        changed_plan.catalog_revision = "catalog-2026-10-03".to_string();
        let mut boundary = MockPaperclipBoundary::default();

        let err = apply_with_boundary(changed_plan, &mut boundary, &token, &mut store).unwrap_err();

        assert!(format!("{err:#}").contains("does not match this immutable plan"));
        assert!(boundary.syncs.is_empty());
    }

    #[test]
    fn apply_rejects_before_state_drift_before_syncing() {
        let plan = plan();
        let mut store = MemoryConfirmationTokenStore::default();
        let token = preview_token_for(
            plan.clone(),
            state(&["company/review"], Some(&["company/review"])),
            &mut store,
        );
        let mut boundary = MockPaperclipBoundary::default();
        boundary.push_state(
            "agent-a",
            state(
                &["company/other", "company/review"],
                Some(&["company/other", "company/review"]),
            ),
        );

        let err = apply_with_boundary(plan, &mut boundary, &token, &mut store).unwrap_err();

        assert!(format!("{err:#}").contains("current skills changed since preview"));
        assert!(boundary.syncs.is_empty());
    }

    #[test]
    fn apply_rejects_stale_confirmation_token() {
        let plan = plan();
        let mut store = MemoryConfirmationTokenStore::default();
        let token = preview_token_for(
            plan.clone(),
            state(&["company/review"], Some(&["company/review"])),
            &mut store,
        );
        store.expire_token(&token);
        let mut boundary = MockPaperclipBoundary::default();

        let err = apply_with_boundary(plan, &mut boundary, &token, &mut store).unwrap_err();

        assert!(format!("{err:#}").contains("confirmation token has expired"));
        assert!(boundary.syncs.is_empty());
    }

    #[test]
    fn apply_rejects_confirmation_token_replay() {
        let plan = plan();
        let mut store = MemoryConfirmationTokenStore::default();
        let token = preview_token_for(
            plan.clone(),
            state(&["company/review"], Some(&["company/review"])),
            &mut store,
        );
        let mut first_boundary = MockPaperclipBoundary::default();
        first_boundary.push_state(
            "agent-a",
            state(&["company/review"], Some(&["company/review"])),
        );
        first_boundary.push_state(
            "agent-a",
            state(
                &["company/review", "company/rust"],
                Some(&["company/review", "company/rust"]),
            ),
        );
        apply_with_boundary(plan.clone(), &mut first_boundary, &token, &mut store).unwrap();

        let mut second_boundary = MockPaperclipBoundary::default();
        let err = apply_with_boundary(plan, &mut second_boundary, &token, &mut store).unwrap_err();

        assert!(format!("{err:#}").contains("confirmation token has already been used"));
        assert!(second_boundary.syncs.is_empty());
    }

    #[test]
    fn apply_collects_multi_agent_partial_failures_and_readbacks() {
        let plan = resolve_plan(
            &catalog(),
            &two_agent_assignments(),
            &[String::from("paperclip-agent")],
        )
        .unwrap();
        let mut preview_boundary = MockPaperclipBoundary::default();
        preview_boundary.push_state("agent-a", state(&["company/review"], None));
        preview_boundary.push_state("agent-b", state(&["company/review"], None));
        let mut store = MemoryConfirmationTokenStore::default();
        let token = preview_with_boundary(plan.clone(), &mut preview_boundary, &mut store)
            .unwrap()
            .confirmation_token()
            .to_string();

        let mut boundary = MockPaperclipBoundary::default();
        boundary.push_state("agent-a", state(&["company/review"], None));
        boundary.push_state("agent-b", state(&["company/review"], None));
        boundary.fail_sync("agent-a");
        boundary.push_state(
            "agent-a",
            state(
                &["company/review", "company/rust"],
                Some(&["company/review", "company/rust"]),
            ),
        );

        let report = apply_with_boundary(plan, &mut boundary, &token, &mut store).unwrap();
        let failures = report.failure_messages().join("\n");
        let rendered = render_apply(&report);

        assert_eq!(boundary.syncs.len(), 2);
        assert_eq!(report.outcomes.len(), 2);
        assert!(failures.contains("desired-skill sync failed"));
        assert!(failures.contains("read-back failed"));
        assert!(rendered.contains("Agent FoundingEngineer (agent-a)"));
        assert!(rendered.contains("Agent Reviewer (agent-b)"));
    }

    #[test]
    fn parses_flexible_paperclip_skill_response() {
        let value = serde_json::json!({
            "data": {
                "desiredSkills": [
                    { "key": "company/review" },
                    "company/rust"
                ],
                "runtimeSkills": [
                    { "companySkillKey": "company/review" },
                    { "slug": "company/rust" }
                ]
            }
        });

        let state =
            parse_agent_skill_state(&AgentId::new("agent-a", "agent_id").unwrap(), &value).unwrap();

        assert_eq!(
            state
                .desired_skills
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec!["company/review", "company/rust"]
        );
        assert_eq!(state.runtime_skills.unwrap(), state.desired_skills);
    }

    #[test]
    fn normalizes_api_base_without_guessing_company_scope() {
        assert_eq!(
            normalize_api_base("https://paperclip.example/api/"),
            "https://paperclip.example"
        );
        assert_eq!(
            normalize_api_base("https://paperclip.example/root"),
            "https://paperclip.example/root"
        );
    }

    #[test]
    fn agent_api_paths_percent_encode_agent_id_segment() {
        let agent_id = AgentId::new("agent/a?b#c", "agent_id").unwrap();

        assert_eq!(
            agent_skills_path(&agent_id),
            "/api/agents/agent%2Fa%3Fb%23c/skills"
        );
        assert_eq!(
            agent_skills_sync_path(&agent_id),
            "/api/agents/agent%2Fa%3Fb%23c/skills/sync"
        );
    }

    #[test]
    fn curl_request_config_uses_real_bearer_token_but_redacts_diagnostics() {
        let boundary = CurlPaperclipBoundary::new(
            "https://paperclip.example/api".to_string(),
            "real-secret-token".to_string(),
        )
        .unwrap();

        let config = boundary
            .curl_config("GET", "/api/agents/agent-a/skills", None)
            .unwrap();
        let redacted = boundary.redact_api_key(
            "curl failed with Authorization: Bearer real-secret-token in diagnostic text",
        );

        assert!(config.contains("Authorization: Bearer real-secret-token"));
        assert!(!config.contains("Authorization: Bearer ***"));
        assert!(!redacted.contains("real-secret-token"));
        assert!(redacted.contains("Authorization: Bearer ***"));
    }

    #[test]
    fn curl_config_escape_handles_secret_and_json_characters() {
        assert_eq!(
            curl_config_escape("Bearer a\"b\\c\n"),
            "Bearer a\\\"b\\\\c\\n"
        );
    }
}
