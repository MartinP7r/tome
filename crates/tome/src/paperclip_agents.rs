//! Paperclip agent desired-skill materialization.
//!
//! This module is deliberately separate from the generic sync/distribution
//! pipeline. Filesystem target deployment must never mutate Paperclip agents as
//! a side effect; callers enter this workflow explicitly through the
//! `paperclip-agents` command.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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
    fn empty() -> Self {
        Self {
            desired_skills: BTreeSet::new(),
            runtime_skills: Some(BTreeSet::new()),
        }
    }

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
    confirmation_token: String,
}

impl MaterializationPlan {
    pub(crate) fn confirmation_token(&self) -> &str {
        &self.confirmation_token
    }
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
    impacts: Vec<AgentImpact>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ApplyReport {
    preview: PreviewReport,
    readback_impacts: Vec<AgentImpact>,
}

pub(crate) trait PaperclipAgentSkillBoundary {
    fn read_agent_skills(&mut self, agent_id: &AgentId) -> Result<AgentSkillState>;
    fn sync_agent_desired_skills(
        &mut self,
        agent_id: &AgentId,
        desired_skills: &[SkillRef],
    ) -> Result<()>;
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
        Ok(self
            .states
            .get(agent_id)
            .cloned()
            .unwrap_or_else(AgentSkillState::empty))
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
            let stderr = String::from_utf8_lossy(&output.stderr);
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
            bail!("Paperclip request {method} {path} failed with HTTP {status}: {body_text}");
        }
        if body_text.trim().is_empty() {
            return Ok(serde_json::Value::Null);
        }
        serde_json::from_str(body_text)
            .with_context(|| format!("failed to parse Paperclip response for {method} {path}"))
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
    let mut plan = MaterializationPlan {
        catalog_revision: catalog.revision.clone(),
        selected_constraints,
        agents,
        confirmation_token: String::new(),
    };
    plan.confirmation_token = confirmation_token(&plan)?;
    Ok(plan)
}

pub(crate) fn preview_with_boundary(
    plan: MaterializationPlan,
    boundary: &mut dyn PaperclipAgentSkillBoundary,
) -> Result<PreviewReport> {
    let mut impacts = Vec::new();
    for agent in &plan.agents {
        let before = boundary.read_agent_skills(&agent.agent_id)?;
        impacts.push(impact_for(agent, &before, None));
    }
    Ok(PreviewReport { plan, impacts })
}

pub(crate) fn apply_with_boundary(
    plan: MaterializationPlan,
    boundary: &mut dyn PaperclipAgentSkillBoundary,
    confirm_token: &str,
) -> Result<ApplyReport> {
    anyhow::ensure!(
        confirm_token == plan.confirmation_token(),
        "confirmation token mismatch; run preview again and pass the exact token it printed"
    );
    let preview = preview_with_boundary(plan.clone(), boundary)?;

    for agent in &plan.agents {
        let desired = agent.desired_skills.iter().cloned().collect::<Vec<_>>();
        boundary.sync_agent_desired_skills(&agent.agent_id, &desired)?;
    }

    let mut readback_impacts = Vec::new();
    let mut failures = Vec::new();
    for agent in &plan.agents {
        let after = boundary.read_agent_skills(&agent.agent_id)?;
        let before = AgentSkillState::empty();
        let impact = impact_for(agent, &before, Some(&after));
        if impact.after_desired_mismatch.is_some() {
            failures.push(format!(
                "agent '{}' desired skills differ after read-back",
                agent.agent_id
            ));
        }
        if impact.after_runtime_mismatch.is_some() {
            failures.push(format!(
                "agent '{}' runtime skills differ after read-back",
                agent.agent_id
            ));
        }
        if impact.after_runtime_unavailable {
            failures.push(format!(
                "agent '{}' runtime skills were not returned on read-back",
                agent.agent_id
            ));
        }
        readback_impacts.push(impact);
    }

    let report = ApplyReport {
        preview,
        readback_impacts,
    };
    if !failures.is_empty() {
        bail!(
            "Paperclip read-back verification failed: {}",
            failures.join("; ")
        );
    }
    Ok(report)
}

pub(crate) fn render_preview(report: &PreviewReport) -> String {
    let mut out = String::new();
    push_header(&mut out, &report.plan);
    out.push_str("No Paperclip agent skills were changed.\n");
    render_impacts(&mut out, &report.impacts, false);
    out
}

pub(crate) fn render_apply(report: &ApplyReport) -> String {
    let mut out = String::new();
    push_header(&mut out, &report.preview.plan);
    out.push_str("Synchronized Paperclip agent desired-skill sets and read them back.\n");
    render_impacts(&mut out, &report.readback_impacts, true);
    out
}

fn push_header(out: &mut String, plan: &MaterializationPlan) {
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
    out.push_str(&format!(
        "Confirmation token: {}\n\n",
        plan.confirmation_token()
    ));
}

fn render_impacts(out: &mut String, impacts: &[AgentImpact], include_readback: bool) {
    for impact in impacts {
        let name = impact
            .display_name
            .as_deref()
            .map(|name| format!("{name} ({})", impact.agent_id))
            .unwrap_or_else(|| impact.agent_id.to_string());
        out.push_str(&format!("Agent {name}\n"));
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

fn confirmation_token(plan: &MaterializationPlan) -> Result<String> {
    #[derive(Serialize)]
    struct TokenAgent<'a> {
        agent_id: &'a AgentId,
        desired_skills: Vec<&'a SkillRef>,
    }

    #[derive(Serialize)]
    struct TokenInput<'a> {
        catalog_revision: &'a str,
        selected_constraints: Vec<&'a String>,
        agents: Vec<TokenAgent<'a>>,
    }

    let input = TokenInput {
        catalog_revision: &plan.catalog_revision,
        selected_constraints: plan.selected_constraints.iter().collect(),
        agents: plan
            .agents
            .iter()
            .map(|agent| TokenAgent {
                agent_id: &agent.agent_id,
                desired_skills: agent.desired_skills.iter().collect(),
            })
            .collect(),
    };
    let bytes =
        serde_json::to_vec(&input).context("failed to serialize confirmation token input")?;
    let digest = Sha256::digest(&bytes);
    let hex = digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    Ok(format!("apply-{}", &hex[..12]))
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
    }

    impl MockPaperclipBoundary {
        pub(crate) fn push_state(&mut self, agent_id: &str, state: AgentSkillState) {
            self.states
                .entry(AgentId::new(agent_id, "agent_id").unwrap())
                .or_default()
                .push(state);
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
            Ok(())
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
    use super::testing::{MockPaperclipBoundary, state};
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
        assert!(plan.confirmation_token.starts_with("apply-"));
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

        let report = preview_with_boundary(plan, &mut boundary).unwrap();

        assert!(boundary.syncs.is_empty());
        let text = render_preview(&report);
        assert!(text.contains("No Paperclip agent skills were changed."));
        assert!(text.contains("add: company/rust"));
        assert!(text.contains("remove: company/old"));
        assert!(text.contains("before desired/runtime mismatch"));
    }

    #[test]
    fn apply_requires_matching_confirmation_token_before_syncing() {
        let plan = resolve_plan(
            &catalog(),
            &assignments(),
            &[String::from("paperclip-agent")],
        )
        .unwrap();
        let expected_token = plan.confirmation_token().to_string();
        let mut boundary = MockPaperclipBoundary::default();

        let err = apply_with_boundary(plan, &mut boundary, "apply-wrong").unwrap_err();
        let error_text = format!("{err:#}");

        assert!(error_text.contains("confirmation token mismatch"));
        assert!(!error_text.contains(&expected_token));
        assert!(boundary.syncs.is_empty());
    }

    #[test]
    fn apply_syncs_then_reads_back_every_agent() {
        let plan = resolve_plan(
            &catalog(),
            &assignments(),
            &[String::from("paperclip-agent")],
        )
        .unwrap();
        let token = plan.confirmation_token().to_string();
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

        let report = apply_with_boundary(plan, &mut boundary, &token).unwrap();

        assert_eq!(boundary.syncs.len(), 1);
        assert_eq!(report.readback_impacts.len(), 1);
        assert!(render_apply(&report).contains("Synchronized Paperclip agent desired-skill sets"));
    }

    #[test]
    fn apply_surfaces_runtime_mismatch_after_readback() {
        let plan = resolve_plan(
            &catalog(),
            &assignments(),
            &[String::from("paperclip-agent")],
        )
        .unwrap();
        let token = plan.confirmation_token().to_string();
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

        let err = apply_with_boundary(plan, &mut boundary, &token).unwrap_err();

        assert!(format!("{err:#}").contains("runtime skills differ"));
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
    fn curl_config_escape_handles_secret_and_json_characters() {
        assert_eq!(
            curl_config_escape("Bearer a\"b\\c\n"),
            "Bearer a\\\"b\\\\c\\n"
        );
    }
}
