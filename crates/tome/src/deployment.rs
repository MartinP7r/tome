//! Create-only target copy deployments with external records.
//!
//! This module is the first MCO-144 slice: it can inspect a resolved
//! `(skill, target)` route, render a create-only plan, materialize an absent
//! destination as a real directory copy, and persist ownership state outside the
//! target tree. It intentionally does not refresh, remove, repair, or migrate
//! existing artifacts.

use std::fs::{self, OpenOptions};
use std::io::ErrorKind;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::config::DirectoryName;
use crate::discover::SkillName;
use crate::manifest;
use crate::validation::ContentHash;

const DEPLOYMENT_SCHEMA_VERSION: u32 = 1;
const DEPLOYMENT_ROOT: &str = "deployments";
const DEPLOYMENT_VERSION_DIR: &str = "v1";

/// Target materialization mode. The first supported mode is a real copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum MaterializationMode {
    Copy,
}

/// Symlink-aware read-only state for a selected route before mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum DeploymentState {
    Absent,
    Foreign,
    Healthy,
    Drifted,
    LegacySymlink,
    Unavailable,
    Disabled,
    StaleRecord,
    Interrupted,
    InvalidCanonical,
    Busy,
}

/// Stable filesystem identity for target roots and deployed directories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FileIdentity {
    dev: u64,
    ino: u64,
}

impl FileIdentity {
    fn from_metadata(metadata: &fs::Metadata) -> Self {
        Self {
            dev: metadata.dev(),
            ino: metadata.ino(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DeploymentRecord {
    schema_version: u32,
    skill_id: SkillName,
    canonical_hash: ContentHash,
    target_name: DirectoryName,
    target_root: PathBuf,
    target_root_identity: FileIdentity,
    target_path: PathBuf,
    materialization: MaterializationMode,
    observed_target_hash: ContentHash,
    observed_target_identity: FileIdentity,
    observed_at: String,
    ownership: DeploymentOwnership,
    transition: TransitionSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeploymentOwnership {
    owner: String,
    selected_profile: Option<DirectoryName>,
    source_name: Option<DirectoryName>,
    managed_source: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransitionSummary {
    transition_id: String,
    completed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TransitionRecord {
    schema_version: u32,
    transition_id: String,
    skill_id: SkillName,
    target_name: DirectoryName,
    target_root: PathBuf,
    target_root_identity: FileIdentity,
    target_path: PathBuf,
    staging_path: PathBuf,
    canonical_hash: ContentHash,
    phase: TransitionPhase,
    owner_pid: u32,
    created_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum TransitionPhase {
    Staging,
    ActivatedPendingRecord,
}

/// Immutable create-only plan produced by read-only inspection.
#[derive(Debug, Clone)]
pub(crate) struct DeploymentPlan {
    pub(crate) state: DeploymentState,
    action: DeploymentAction,
    config_dir: PathBuf,
    selected_profile: Option<DirectoryName>,
    target_name: DirectoryName,
    target_root: PathBuf,
    target_root_identity: Option<FileIdentity>,
    target_path: PathBuf,
    canonical_path: PathBuf,
    canonical_hash: ContentHash,
    skill: SkillName,
    source_name: Option<DirectoryName>,
    managed_source: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeploymentAction {
    Create,
    Skip,
}

/// Result of applying a deployment plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DeploymentApplyResult {
    pub(crate) state: DeploymentState,
    pub(crate) changed: bool,
    pub(crate) unchanged: bool,
    pub(crate) skipped: bool,
}

impl DeploymentApplyResult {
    fn changed(state: DeploymentState) -> Self {
        Self {
            state,
            changed: true,
            unchanged: false,
            skipped: false,
        }
    }

    fn unchanged(state: DeploymentState) -> Self {
        Self {
            state,
            changed: false,
            unchanged: true,
            skipped: false,
        }
    }

    fn skipped(state: DeploymentState) -> Self {
        Self {
            state,
            changed: false,
            unchanged: false,
            skipped: true,
        }
    }
}

/// Provenance data copied from the library manifest into deployment records.
#[derive(Debug, Clone, Default)]
pub(crate) struct DeploymentProvenance {
    pub(crate) source_name: Option<DirectoryName>,
    pub(crate) managed_source: bool,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn plan_create_only(
    config_dir: &Path,
    selected_profile: Option<&DirectoryName>,
    target_name: &DirectoryName,
    target_root: &Path,
    skill: &SkillName,
    canonical_path: &Path,
    canonical_hash: &ContentHash,
    provenance: DeploymentProvenance,
) -> Result<DeploymentPlan> {
    let target_path = target_root.join(skill.as_str());
    let base = DeploymentPlan {
        state: DeploymentState::InvalidCanonical,
        action: DeploymentAction::Skip,
        config_dir: config_dir.to_path_buf(),
        selected_profile: selected_profile.cloned(),
        target_name: target_name.clone(),
        target_root: target_root.to_path_buf(),
        target_root_identity: None,
        target_path,
        canonical_path: canonical_path.to_path_buf(),
        canonical_hash: canonical_hash.clone(),
        skill: skill.clone(),
        source_name: provenance.source_name,
        managed_source: provenance.managed_source,
    };

    if validate_regular_tree(canonical_path).is_err() {
        return Ok(base);
    }

    let root_metadata = match fs::symlink_metadata(target_root) {
        Ok(metadata) if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() => {
            metadata
        }
        Ok(_) => return Ok(base.with_state(DeploymentState::Unavailable)),
        Err(e) if e.kind() == ErrorKind::NotFound => {
            return Ok(base.with_state(DeploymentState::Unavailable));
        }
        Err(e) => {
            return Err(e).with_context(|| {
                format!("failed to inspect target root {}", target_root.display())
            });
        }
    };
    let root_identity = FileIdentity::from_metadata(&root_metadata);
    let canonical_root = fs::canonicalize(target_root).with_context(|| {
        format!(
            "failed to canonicalize target root {}",
            target_root.display()
        )
    })?;
    let target_path = canonical_root.join(skill.as_str());

    let mut plan = DeploymentPlan {
        target_root: canonical_root,
        target_root_identity: Some(root_identity),
        target_path,
        ..base
    };

    if transition_path(config_dir, target_name, skill).exists() {
        return Ok(plan.with_state(DeploymentState::Busy));
    }

    let record = load_record(config_dir, target_name, skill)?;
    let destination_metadata = match fs::symlink_metadata(&plan.target_path) {
        Ok(metadata) => Some(metadata),
        Err(e) if e.kind() == ErrorKind::NotFound => None,
        Err(e) => {
            return Err(e).with_context(|| {
                format!(
                    "failed to inspect target path {}",
                    plan.target_path.display()
                )
            });
        }
    };

    match (destination_metadata, record) {
        (None, None) => {
            plan.state = DeploymentState::Absent;
            plan.action = DeploymentAction::Create;
            Ok(plan)
        }
        (None, Some(_)) => Ok(plan.with_state(DeploymentState::StaleRecord)),
        (Some(metadata), None) if metadata.file_type().is_symlink() => {
            Ok(plan.with_state(DeploymentState::LegacySymlink))
        }
        (Some(_), None) => Ok(plan.with_state(DeploymentState::Foreign)),
        (Some(metadata), Some(record)) => {
            if !record_matches_plan(&record, &plan) {
                return Ok(plan.with_state(DeploymentState::Foreign));
            }
            if metadata.file_type().is_symlink() {
                return Ok(plan.with_state(DeploymentState::LegacySymlink));
            }
            if !metadata.file_type().is_dir() {
                return Ok(plan.with_state(DeploymentState::Foreign));
            }
            if FileIdentity::from_metadata(&metadata) != record.observed_target_identity {
                return Ok(plan.with_state(DeploymentState::Drifted));
            }
            if validate_regular_tree(&plan.target_path).is_err() {
                return Ok(plan.with_state(DeploymentState::Drifted));
            }
            let observed_hash = manifest::hash_directory(&plan.target_path).with_context(|| {
                format!(
                    "failed to hash deployed target {}",
                    plan.target_path.display()
                )
            })?;
            if observed_hash == record.observed_target_hash
                && observed_hash == record.canonical_hash
                && observed_hash == plan.canonical_hash
            {
                Ok(plan.with_state(DeploymentState::Healthy))
            } else {
                Ok(plan.with_state(DeploymentState::Drifted))
            }
        }
    }
}

impl DeploymentPlan {
    fn with_state(mut self, state: DeploymentState) -> Self {
        self.state = state;
        self.action = DeploymentAction::Skip;
        self
    }

    pub(crate) fn apply(&self, dry_run: bool) -> Result<DeploymentApplyResult> {
        match self.action {
            DeploymentAction::Skip => {
                if self.state == DeploymentState::Healthy {
                    Ok(DeploymentApplyResult::unchanged(self.state))
                } else {
                    Ok(DeploymentApplyResult::skipped(self.state))
                }
            }
            DeploymentAction::Create if dry_run => Ok(DeploymentApplyResult::changed(self.state)),
            DeploymentAction::Create => self.apply_create(),
        }
    }

    fn apply_create(&self) -> Result<DeploymentApplyResult> {
        let mut transition = match self.acquire_transition_lock()? {
            Some(transition) => transition,
            None => return Ok(DeploymentApplyResult::skipped(DeploymentState::Busy)),
        };
        let mut activated = false;
        let mut staging_path = transition.staging_path.clone();

        let result = (|| -> Result<DeploymentApplyResult> {
            self.recheck_root_identity()?;
            if self.target_path_exists()? {
                return Ok(DeploymentApplyResult::skipped(DeploymentState::Foreign));
            }

            fs::create_dir(&staging_path).with_context(|| {
                format!("failed to create staging dir {}", staging_path.display())
            })?;
            copy_regular_tree(&self.canonical_path, &staging_path)?;
            let staged_hash = manifest::hash_directory(&staging_path).with_context(|| {
                format!("failed to hash staging dir {}", staging_path.display())
            })?;
            if staged_hash != self.canonical_hash {
                bail!(
                    "staged hash {} does not match canonical hash {} for {}",
                    staged_hash,
                    self.canonical_hash,
                    self.skill
                );
            }

            self.ensure_transition_lock(&transition)?;
            self.recheck_root_identity()?;
            if self.target_path_exists()? {
                return Ok(DeploymentApplyResult::skipped(DeploymentState::Foreign));
            }

            fs::rename(&staging_path, &self.target_path).with_context(|| {
                format!(
                    "failed to activate staging dir {} -> {}",
                    staging_path.display(),
                    self.target_path.display()
                )
            })?;
            activated = true;
            staging_path = self.target_path.clone();
            transition.phase = TransitionPhase::ActivatedPendingRecord;
            save_transition_record(&self.lock_path(), &transition)?;

            validate_regular_tree(&self.target_path)?;
            let observed_hash = manifest::hash_directory(&self.target_path).with_context(|| {
                format!("failed to hash target dir {}", self.target_path.display())
            })?;
            let observed_metadata = fs::symlink_metadata(&self.target_path).with_context(|| {
                format!(
                    "failed to inspect target dir {}",
                    self.target_path.display()
                )
            })?;
            let record = DeploymentRecord {
                schema_version: DEPLOYMENT_SCHEMA_VERSION,
                skill_id: self.skill.clone(),
                canonical_hash: self.canonical_hash.clone(),
                target_name: self.target_name.clone(),
                target_root: self.target_root.clone(),
                target_root_identity: self
                    .target_root_identity
                    .context("BUG: create plan missing root identity")?,
                target_path: self.target_path.clone(),
                materialization: MaterializationMode::Copy,
                observed_target_hash: observed_hash,
                observed_target_identity: FileIdentity::from_metadata(&observed_metadata),
                observed_at: manifest::now_iso8601(),
                ownership: DeploymentOwnership {
                    owner: "tome".to_string(),
                    selected_profile: self.selected_profile.clone(),
                    source_name: self.source_name.clone(),
                    managed_source: self.managed_source,
                },
                transition: TransitionSummary {
                    transition_id: transition.transition_id.clone(),
                    completed_at: manifest::now_iso8601(),
                },
            };
            save_record(&self.record_path(), &record)?;
            fs::remove_file(self.lock_path()).with_context(|| {
                format!(
                    "failed to remove transition lock {}",
                    self.lock_path().display()
                )
            })?;
            Ok(DeploymentApplyResult::changed(DeploymentState::Healthy))
        })();

        if result.is_err() && !activated {
            let _ = fs::remove_dir_all(&staging_path);
            let _ = fs::remove_file(self.lock_path());
        }

        result
    }

    fn acquire_transition_lock(&self) -> Result<Option<TransitionRecord>> {
        let root_identity = self
            .target_root_identity
            .context("BUG: create plan missing root identity")?;
        let transition_id = new_transition_id();
        let staging_path = self.target_root.join(format!(
            ".tome-stage-{}-{transition_id}",
            self.skill.as_str()
        ));
        let transition = TransitionRecord {
            schema_version: DEPLOYMENT_SCHEMA_VERSION,
            transition_id,
            skill_id: self.skill.clone(),
            target_name: self.target_name.clone(),
            target_root: self.target_root.clone(),
            target_root_identity: root_identity,
            target_path: self.target_path.clone(),
            staging_path,
            canonical_hash: self.canonical_hash.clone(),
            phase: TransitionPhase::Staging,
            owner_pid: std::process::id(),
            created_at: manifest::now_iso8601(),
        };
        let lock_path = self.lock_path();
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create deployment record dir {}",
                    parent.display()
                )
            })?;
        }
        let content = serde_json::to_vec_pretty(&transition)
            .context("failed to serialize deployment transition")?;
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(mut file) => {
                use std::io::Write;
                file.write_all(&content).with_context(|| {
                    format!("failed to write transition lock {}", lock_path.display())
                })?;
                Ok(Some(transition))
            }
            Err(e) if e.kind() == ErrorKind::AlreadyExists => Ok(None),
            Err(e) => Err(e).with_context(|| {
                format!("failed to create transition lock {}", lock_path.display())
            }),
        }
    }

    fn ensure_transition_lock(&self, expected: &TransitionRecord) -> Result<()> {
        let current = load_transition_record(&self.lock_path())?;
        if current.transition_id != expected.transition_id {
            bail!(
                "transition lock changed for {} while staging",
                self.target_path.display()
            );
        }
        Ok(())
    }

    fn recheck_root_identity(&self) -> Result<()> {
        let expected = self
            .target_root_identity
            .context("BUG: create plan missing root identity")?;
        let metadata = fs::symlink_metadata(&self.target_root).with_context(|| {
            format!(
                "failed to recheck target root {}",
                self.target_root.display()
            )
        })?;
        if !metadata.file_type().is_dir()
            || metadata.file_type().is_symlink()
            || FileIdentity::from_metadata(&metadata) != expected
        {
            bail!(
                "target root identity changed before deployment: {}",
                self.target_root.display()
            );
        }
        Ok(())
    }

    fn target_path_exists(&self) -> Result<bool> {
        match fs::symlink_metadata(&self.target_path) {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
            Err(e) => {
                Err(e).with_context(|| format!("failed to inspect {}", self.target_path.display()))
            }
        }
    }

    pub(crate) fn record_path(&self) -> PathBuf {
        record_path(&self.config_dir, &self.target_name, &self.skill)
    }

    fn lock_path(&self) -> PathBuf {
        transition_path(&self.config_dir, &self.target_name, &self.skill)
    }
}

fn record_matches_plan(record: &DeploymentRecord, plan: &DeploymentPlan) -> bool {
    record.schema_version == DEPLOYMENT_SCHEMA_VERSION
        && record.skill_id == plan.skill
        && record.target_name == plan.target_name
        && record.target_root == plan.target_root
        && Some(record.target_root_identity) == plan.target_root_identity
        && record.target_path == plan.target_path
        && record.materialization == MaterializationMode::Copy
}

pub(crate) fn record_path(
    config_dir: &Path,
    target_name: &DirectoryName,
    skill: &SkillName,
) -> PathBuf {
    deployment_dir(config_dir, target_name).join(format!("{}.json", skill.as_str()))
}

fn transition_path(config_dir: &Path, target_name: &DirectoryName, skill: &SkillName) -> PathBuf {
    deployment_dir(config_dir, target_name).join(format!("{}.lock.json", skill.as_str()))
}

fn deployment_dir(config_dir: &Path, target_name: &DirectoryName) -> PathBuf {
    config_dir
        .join(DEPLOYMENT_ROOT)
        .join(DEPLOYMENT_VERSION_DIR)
        .join(target_name.as_str())
}

fn load_record(
    config_dir: &Path,
    target_name: &DirectoryName,
    skill: &SkillName,
) -> Result<Option<DeploymentRecord>> {
    let path = record_path(config_dir, target_name, skill);
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("failed to read deployment record {}", path.display()))?;
    let record: DeploymentRecord = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse deployment record {}", path.display()))?;
    Ok(Some(record))
}

fn save_record(path: &Path, record: &DeploymentRecord) -> Result<()> {
    atomic_write_json(path, record, "deployment record")
}

fn save_transition_record(path: &Path, record: &TransitionRecord) -> Result<()> {
    atomic_write_json(path, record, "deployment transition")
}

fn load_transition_record(path: &Path) -> Result<TransitionRecord> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read deployment transition {}", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("failed to parse deployment transition {}", path.display()))
}

fn atomic_write_json<T: Serialize>(path: &Path, value: &T, label: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let tmp = path.with_extension(format!("tmp-{}", new_transition_id()));
    let content = serde_json::to_string_pretty(value)
        .with_context(|| format!("failed to serialize {label}"))?;
    fs::write(&tmp, content)
        .with_context(|| format!("failed to write temporary {label} {}", tmp.display()))?;
    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(e)
            .with_context(|| format!("failed to rename {} -> {}", tmp.display(), path.display()));
    }
    Ok(())
}

fn validate_regular_tree(root: &Path) -> Result<()> {
    let root_metadata = fs::symlink_metadata(root)
        .with_context(|| format!("failed to inspect canonical root {}", root.display()))?;
    if !root_metadata.file_type().is_dir() || root_metadata.file_type().is_symlink() {
        bail!("canonical root is not a real directory: {}", root.display());
    }
    for entry in WalkDir::new(root).follow_links(false).into_iter() {
        let entry = entry.with_context(|| format!("failed to walk {}", root.display()))?;
        let metadata = fs::symlink_metadata(entry.path())
            .with_context(|| format!("failed to inspect {}", entry.path().display()))?;
        let ty = metadata.file_type();
        if ty.is_dir() || ty.is_file() {
            continue;
        }
        bail!(
            "deployment tree contains symlink or special file: {}",
            entry.path().display()
        );
    }
    Ok(())
}

fn copy_regular_tree(source: &Path, destination: &Path) -> Result<()> {
    validate_regular_tree(source)?;
    let mut entries = Vec::new();
    for entry in WalkDir::new(source).follow_links(false).into_iter() {
        let entry = entry.with_context(|| format!("failed to walk {}", source.display()))?;
        entries.push(entry.path().to_path_buf());
    }
    entries.sort();

    for path in entries {
        let rel = path.strip_prefix(source).with_context(|| {
            format!(
                "BUG: WalkDir yielded path {} not under {}",
                path.display(),
                source.display()
            )
        })?;
        if rel.as_os_str().is_empty() {
            continue;
        }
        let target = destination.join(rel);
        let metadata = fs::symlink_metadata(&path)
            .with_context(|| format!("failed to inspect {}", path.display()))?;
        let ty = metadata.file_type();
        if ty.is_dir() {
            fs::create_dir(&target)
                .with_context(|| format!("failed to create directory {}", target.display()))?;
            fs::set_permissions(&target, metadata.permissions()).with_context(|| {
                format!(
                    "failed to set permissions on directory {}",
                    target.display()
                )
            })?;
        } else if ty.is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create directory {}", parent.display()))?;
            }
            fs::copy(&path, &target).with_context(|| {
                format!("failed to copy {} -> {}", path.display(), target.display())
            })?;
            fs::set_permissions(&target, fs::Permissions::from_mode(metadata.mode() & 0o777))
                .with_context(|| format!("failed to set permissions on {}", target.display()))?;
        } else {
            bail!(
                "deployment tree contains symlink or special file: {}",
                path.display()
            );
        }
    }
    Ok(())
}

fn new_transition_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    format!("{}-{nanos}", std::process::id())
}

pub(crate) fn transition_findings(config_dir: &Path) -> Result<Vec<String>> {
    let root = config_dir
        .join(DEPLOYMENT_ROOT)
        .join(DEPLOYMENT_VERSION_DIR);
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut findings = Vec::new();
    for target in fs::read_dir(&root)
        .with_context(|| format!("failed to read deployment records {}", root.display()))?
    {
        let target =
            target.with_context(|| format!("failed to read entry in {}", root.display()))?;
        if !target
            .file_type()
            .with_context(|| format!("failed to inspect {}", target.path().display()))?
            .is_dir()
        {
            continue;
        }
        for entry in fs::read_dir(target.path()).with_context(|| {
            format!(
                "failed to read deployment target {}",
                target.path().display()
            )
        })? {
            let entry = entry
                .with_context(|| format!("failed to read entry in {}", target.path().display()))?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json")
                || !path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|name| name.ends_with(".lock.json"))
            {
                continue;
            }
            let message = match load_transition_record(&path) {
                Ok(record) => format!(
                    "interrupted deployment transition for '{}' in target '{}': phase {:?}, staging {}",
                    record.skill_id,
                    record.target_name,
                    record.phase,
                    record.staging_path.display()
                ),
                Err(e) => format!(
                    "interrupted deployment transition is unreadable at {}: {e:#}",
                    path.display()
                ),
            };
            findings.push(message);
        }
    }
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_skill(root: &Path, name: &str) -> PathBuf {
        let skill = root.join(name);
        fs::create_dir_all(&skill).unwrap();
        fs::write(skill.join("SKILL.md"), format!("# {name}\n")).unwrap();
        skill
    }

    fn plan(
        config_dir: &Path,
        target_root: &Path,
        canonical_path: &Path,
        skill: &str,
    ) -> DeploymentPlan {
        let skill = SkillName::new(skill).unwrap();
        let hash = manifest::hash_directory(canonical_path).unwrap();
        plan_create_only(
            config_dir,
            Some(&DirectoryName::new("workstation").unwrap()),
            &DirectoryName::new("codex").unwrap(),
            target_root,
            &skill,
            canonical_path,
            &hash,
            DeploymentProvenance::default(),
        )
        .unwrap()
    }

    #[test]
    fn create_copy_is_independent_from_canonical_pool() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let library = tmp.path().join("library");
        let target = tmp.path().join("target");
        fs::create_dir_all(&library).unwrap();
        fs::create_dir_all(&target).unwrap();
        let canonical = create_skill(&library, "skill-a");

        let plan = plan(&config_dir, &target, &canonical, "skill-a");
        assert_eq!(plan.state, DeploymentState::Absent);
        let result = plan.apply(false).unwrap();
        assert!(result.changed);
        assert!(target.join("skill-a/SKILL.md").is_file());
        assert!(plan.record_path().is_file());

        fs::remove_dir_all(&library).unwrap();
        let deployed = fs::read_to_string(target.join("skill-a/SKILL.md")).unwrap();
        assert_eq!(deployed, "# skill-a\n");
    }

    #[test]
    fn dry_run_create_writes_nothing() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let library = tmp.path().join("library");
        let target = tmp.path().join("target");
        fs::create_dir_all(&library).unwrap();
        fs::create_dir_all(&target).unwrap();
        let canonical = create_skill(&library, "skill-a");

        let plan = plan(&config_dir, &target, &canonical, "skill-a");
        let result = plan.apply(true).unwrap();
        assert!(result.changed);
        assert!(!target.join("skill-a").exists());
        assert!(!config_dir.join(DEPLOYMENT_ROOT).exists());
        assert!(fs::read_dir(&target).unwrap().next().is_none());
    }

    #[test]
    fn existing_target_artifact_is_preserved_as_foreign() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let library = tmp.path().join("library");
        let target = tmp.path().join("target");
        fs::create_dir_all(&library).unwrap();
        fs::create_dir_all(target.join("skill-a")).unwrap();
        fs::write(target.join("skill-a/SKILL.md"), "foreign").unwrap();
        let canonical = create_skill(&library, "skill-a");

        let plan = plan(&config_dir, &target, &canonical, "skill-a");
        assert_eq!(plan.state, DeploymentState::Foreign);
        let result = plan.apply(false).unwrap();
        assert!(result.skipped);
        assert_eq!(
            fs::read_to_string(target.join("skill-a/SKILL.md")).unwrap(),
            "foreign"
        );
        assert!(!plan.record_path().exists());
    }

    #[test]
    fn missing_target_root_is_unavailable_and_not_created() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let library = tmp.path().join("library");
        let target = tmp.path().join("missing-target");
        fs::create_dir_all(&library).unwrap();
        let canonical = create_skill(&library, "skill-a");

        let plan = plan(&config_dir, &target, &canonical, "skill-a");
        assert_eq!(plan.state, DeploymentState::Unavailable);
        let result = plan.apply(false).unwrap();
        assert!(result.skipped);
        assert!(!target.exists());
    }

    #[test]
    fn canonical_symlink_is_invalid_and_not_copied() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let library = tmp.path().join("library");
        let target = tmp.path().join("target");
        fs::create_dir_all(&library).unwrap();
        fs::create_dir_all(&target).unwrap();
        let canonical = create_skill(&library, "skill-a");
        std::os::unix::fs::symlink("/tmp/not-deployed", canonical.join("link")).unwrap();

        let hash = manifest::hash_directory(&canonical).unwrap();
        let plan = plan_create_only(
            &config_dir,
            None,
            &DirectoryName::new("codex").unwrap(),
            &target,
            &SkillName::new("skill-a").unwrap(),
            &canonical,
            &hash,
            DeploymentProvenance::default(),
        )
        .unwrap();
        assert_eq!(plan.state, DeploymentState::InvalidCanonical);
        assert!(plan.apply(false).unwrap().skipped);
        assert!(!target.join("skill-a").exists());
    }

    #[test]
    fn canonical_special_file_is_invalid_and_not_copied() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let library = tmp.path().join("library");
        let target = tmp.path().join("target");
        fs::create_dir_all(&library).unwrap();
        fs::create_dir_all(&target).unwrap();
        let canonical = create_skill(&library, "skill-a");
        let fifo = canonical.join("fifo");
        let status = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap();
        assert!(
            status.success(),
            "mkfifo should create a special-file fixture"
        );

        let hash = manifest::hash_directory(&canonical).unwrap();
        let plan = plan_create_only(
            &config_dir,
            None,
            &DirectoryName::new("codex").unwrap(),
            &target,
            &SkillName::new("skill-a").unwrap(),
            &canonical,
            &hash,
            DeploymentProvenance::default(),
        )
        .unwrap();
        assert_eq!(plan.state, DeploymentState::InvalidCanonical);
        assert!(plan.apply(false).unwrap().skipped);
        assert!(!target.join("skill-a").exists());
    }

    #[test]
    fn held_transition_lock_reports_busy() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let library = tmp.path().join("library");
        let target = tmp.path().join("target");
        fs::create_dir_all(&library).unwrap();
        fs::create_dir_all(&target).unwrap();
        let canonical = create_skill(&library, "skill-a");
        let first = plan(&config_dir, &target, &canonical, "skill-a");
        let transition = first.acquire_transition_lock().unwrap().unwrap();
        assert!(transition_path(&config_dir, &first.target_name, &first.skill).exists());

        let second = plan(&config_dir, &target, &canonical, "skill-a");
        assert_eq!(second.state, DeploymentState::Busy);
        assert!(second.apply(false).unwrap().skipped);

        fs::remove_file(transition_path(
            &config_dir,
            &first.target_name,
            &first.skill,
        ))
        .unwrap();
        assert_eq!(transition.skill_id.as_str(), "skill-a");
    }

    #[test]
    fn transition_findings_report_interrupted_locks() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let library = tmp.path().join("library");
        let target = tmp.path().join("target");
        fs::create_dir_all(&library).unwrap();
        fs::create_dir_all(&target).unwrap();
        let canonical = create_skill(&library, "skill-a");
        let plan = plan(&config_dir, &target, &canonical, "skill-a");
        let _transition = plan.acquire_transition_lock().unwrap().unwrap();

        let findings = transition_findings(&config_dir).unwrap();
        assert_eq!(findings.len(), 1);
        assert!(findings[0].contains("interrupted deployment transition"));
        assert!(findings[0].contains("skill-a"));
    }
}
