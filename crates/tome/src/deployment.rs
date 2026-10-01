//! Create-only target copy deployments with external records.
//!
//! This module is the first MCO-144 slice: it can inspect a resolved
//! `(skill, target)` route, render a create-only plan, materialize an absent
//! destination as a real directory copy, and persist ownership state outside the
//! target tree. It intentionally does not refresh, remove, repair, or migrate
//! existing artifacts.

use std::ffi::{CStr, CString, OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind};
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
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
#[cfg(test)]
type BeforeActivateHook = std::sync::Arc<dyn Fn(&DeploymentPlan) + Send + Sync>;

#[derive(Clone)]
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
    #[cfg(test)]
    before_activate: Option<BeforeActivateHook>,
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
        #[cfg(test)]
        before_activate: None,
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

            self.run_before_activate_hook();
            match activate_staging_no_replace(&staging_path, &self.target_path).with_context(
                || {
                    format!(
                        "failed to activate staging dir {} -> {}",
                        staging_path.display(),
                        self.target_path.display()
                    )
                },
            )? {
                ActivateOutcome::Activated => {}
                ActivateOutcome::DestinationExists => {
                    return Ok(DeploymentApplyResult::skipped(DeploymentState::Foreign));
                }
            }
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

        if !activated {
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

    fn run_before_activate_hook(&self) {
        #[cfg(test)]
        if let Some(hook) = &self.before_activate {
            hook(self);
        }
    }

    #[cfg(test)]
    fn with_before_activate(
        mut self,
        hook: impl Fn(&DeploymentPlan) + Send + Sync + 'static,
    ) -> Self {
        self.before_activate = Some(std::sync::Arc::new(hook));
        self
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActivateOutcome {
    Activated,
    DestinationExists,
}

fn activate_staging_no_replace(staging_path: &Path, target_path: &Path) -> Result<ActivateOutcome> {
    let staging = cstring_from_path(staging_path)?;
    let target = cstring_from_path(target_path)?;

    match rename_no_replace(staging.as_c_str(), target.as_c_str()) {
        Ok(()) => Ok(ActivateOutcome::Activated),
        Err(error) if is_destination_exists_error(&error) => Ok(ActivateOutcome::DestinationExists),
        Err(error) => Err(error).context("no-replace rename failed"),
    }
}

#[cfg(target_os = "linux")]
fn rename_no_replace(staging: &CStr, target: &CStr) -> io::Result<()> {
    let rc = unsafe {
        // SAFETY: both C strings are NUL-terminated, live for the call, and are
        // passed with AT_FDCWD so libc does not retain their pointers.
        libc::renameat2(
            libc::AT_FDCWD,
            staging.as_ptr(),
            libc::AT_FDCWD,
            target.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    syscall_unit_result(rc)
}

#[cfg(target_os = "macos")]
fn rename_no_replace(staging: &CStr, target: &CStr) -> io::Result<()> {
    let rc = unsafe {
        // SAFETY: both C strings are NUL-terminated, live for the call, and are
        // passed with AT_FDCWD so libc does not retain their pointers.
        libc::renameatx_np(
            libc::AT_FDCWD,
            staging.as_ptr(),
            libc::AT_FDCWD,
            target.as_ptr(),
            libc::RENAME_EXCL,
        )
    };
    syscall_unit_result(rc)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn rename_no_replace(staging: &CStr, target: &CStr) -> io::Result<()> {
    let _ = (staging, target);
    Err(io::Error::new(
        ErrorKind::Unsupported,
        "no-replace deployment activation is unsupported on this platform",
    ))
}

fn syscall_unit_result(rc: i32) -> io::Result<()> {
    if rc == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

fn is_destination_exists_error(error: &io::Error) -> bool {
    error.kind() == ErrorKind::AlreadyExists
        || error.raw_os_error() == Some(libc::EEXIST)
        || error.raw_os_error() == Some(libc::ENOTEMPTY)
}

fn copy_regular_tree(source: &Path, destination: &Path) -> Result<()> {
    copy_regular_tree_inner(source, destination, &mut |_| Ok(()))
}

#[cfg(test)]
fn copy_regular_tree_with_hook(
    source: &Path,
    destination: &Path,
    mut before_open: impl FnMut(&Path) -> Result<()>,
) -> Result<()> {
    copy_regular_tree_inner(source, destination, &mut before_open)
}

fn copy_regular_tree_inner(
    source: &Path,
    destination: &Path,
    before_open: &mut dyn FnMut(&Path) -> Result<()>,
) -> Result<()> {
    let source_dir = SourceDir::open_root(source)?;
    copy_regular_dir_entries(&source_dir, source, destination, before_open)
}

struct SourceDir {
    file: File,
}

impl SourceDir {
    fn open_root(path: &Path) -> Result<Self> {
        let path_c = cstring_from_path(path)?;
        let fd = unsafe {
            // SAFETY: path_c is a valid, NUL-terminated path for this call.
            libc::open(
                path_c.as_ptr(),
                libc::O_RDONLY | libc::O_CLOEXEC | libc::O_DIRECTORY | libc::O_NOFOLLOW,
            )
        };
        let file = file_from_fd(fd)
            .with_context(|| format!("failed to open source directory {}", path.display()))?;
        let stat = fstat_file(&file)
            .with_context(|| format!("failed to inspect source directory {}", path.display()))?;
        if !stat_is_dir(&stat) {
            bail!("canonical root is not a real directory: {}", path.display());
        }
        Ok(Self { file })
    }

    fn open_child_dir(&self, name: &OsStr, expected: &libc::stat, path: &Path) -> Result<Self> {
        let name_c = cstring_from_os_str(name)?;
        let fd = unsafe {
            // SAFETY: name_c is a single path component opened relative to self.file.
            libc::openat(
                self.file.as_raw_fd(),
                name_c.as_ptr(),
                libc::O_RDONLY | libc::O_CLOEXEC | libc::O_DIRECTORY | libc::O_NOFOLLOW,
            )
        };
        let file = file_from_fd(fd)
            .with_context(|| format!("failed to open directory {}", path.display()))?;
        let actual =
            fstat_file(&file).with_context(|| format!("failed to inspect {}", path.display()))?;
        if !stat_is_dir(&actual) || !same_identity(expected, &actual) {
            bail!("source directory changed while staging: {}", path.display());
        }
        Ok(Self { file })
    }
}

struct DirStream {
    ptr: *mut libc::DIR,
}

impl Drop for DirStream {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: ptr came from fdopendir and is owned by this DirStream.
            libc::closedir(self.ptr);
        }
    }
}

#[derive(Clone)]
struct SourceEntry {
    name: OsString,
    stat: libc::stat,
}

fn copy_regular_dir_entries(
    source_dir: &SourceDir,
    source_path: &Path,
    destination: &Path,
    before_open: &mut dyn FnMut(&Path) -> Result<()>,
) -> Result<()> {
    let mut entries = read_dir_entries(source_dir, source_path)?;
    entries.sort_by(|a, b| {
        a.name
            .as_os_str()
            .as_bytes()
            .cmp(b.name.as_os_str().as_bytes())
    });

    for entry in entries {
        let source_child = source_path.join(&entry.name);
        let target_child = destination.join(&entry.name);
        if stat_is_dir(&entry.stat) {
            before_open(&source_child)?;
            let child_dir = source_dir.open_child_dir(&entry.name, &entry.stat, &source_child)?;
            fs::create_dir(&target_child).with_context(|| {
                format!("failed to create directory {}", target_child.display())
            })?;
            copy_regular_dir_entries(&child_dir, &source_child, &target_child, before_open)?;
            fs::set_permissions(
                &target_child,
                fs::Permissions::from_mode(permission_bits(&entry.stat)),
            )
            .with_context(|| {
                format!(
                    "failed to set permissions on directory {}",
                    target_child.display()
                )
            })?;
        } else if stat_is_regular(&entry.stat) {
            before_open(&source_child)?;
            copy_regular_file(
                source_dir,
                &entry.name,
                &entry.stat,
                &source_child,
                &target_child,
            )?;
        } else {
            bail!(
                "deployment tree contains symlink or special file: {}",
                source_child.display()
            );
        }
    }
    Ok(())
}

fn read_dir_entries(source_dir: &SourceDir, source_path: &Path) -> Result<Vec<SourceEntry>> {
    let duplicated_fd = unsafe {
        // SAFETY: dup only borrows the fd number and returns a new owned fd.
        libc::dup(source_dir.file.as_raw_fd())
    };
    if duplicated_fd < 0 {
        return Err(io::Error::last_os_error())
            .with_context(|| format!("failed to duplicate fd for {}", source_path.display()));
    }
    let dir_ptr = unsafe {
        // SAFETY: duplicated_fd is a valid owned directory fd; fdopendir takes ownership.
        libc::fdopendir(duplicated_fd)
    };
    if dir_ptr.is_null() {
        let error = io::Error::last_os_error();
        unsafe {
            // SAFETY: fdopendir failed, so duplicated_fd is still owned here.
            libc::close(duplicated_fd);
        }
        return Err(error).with_context(|| format!("failed to read {}", source_path.display()));
    }
    let stream = DirStream { ptr: dir_ptr };
    let mut entries = Vec::new();

    loop {
        clear_errno();
        let dirent = unsafe {
            // SAFETY: stream.ptr is a live DIR* owned for the duration of this loop.
            libc::readdir(stream.ptr)
        };
        if dirent.is_null() {
            let errno = current_errno();
            if errno == 0 {
                break;
            }
            return Err(io::Error::from_raw_os_error(errno))
                .with_context(|| format!("failed to read {}", source_path.display()));
        }

        let name = unsafe {
            // SAFETY: readdir returned a non-null dirent whose d_name is NUL-terminated.
            CStr::from_ptr((*dirent).d_name.as_ptr())
        };
        let bytes = name.to_bytes();
        if bytes == b"." || bytes == b".." {
            continue;
        }
        let name_os = OsStr::from_bytes(bytes).to_os_string();
        let stat = fstatat_nofollow(source_dir.file.as_raw_fd(), &name_os).with_context(|| {
            format!("failed to inspect {}", source_path.join(&name_os).display())
        })?;
        entries.push(SourceEntry {
            name: name_os,
            stat,
        });
    }

    Ok(entries)
}

fn copy_regular_file(
    source_dir: &SourceDir,
    name: &OsStr,
    expected: &libc::stat,
    source_path: &Path,
    target_path: &Path,
) -> Result<()> {
    let name_c = cstring_from_os_str(name)?;
    let fd = unsafe {
        // SAFETY: name_c is a single path component opened relative to source_dir.file.
        libc::openat(
            source_dir.file.as_raw_fd(),
            name_c.as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
        )
    };
    let mut source_file =
        file_from_fd(fd).with_context(|| format!("failed to open {}", source_path.display()))?;
    let actual = fstat_file(&source_file)
        .with_context(|| format!("failed to inspect {}", source_path.display()))?;
    if !stat_is_regular(&actual) || !same_identity(expected, &actual) {
        bail!(
            "source file changed while staging: {}",
            source_path.display()
        );
    }

    let mut target_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(permission_bits(expected))
        .open(target_path)
        .with_context(|| format!("failed to create {}", target_path.display()))?;
    io::copy(&mut source_file, &mut target_file).with_context(|| {
        format!(
            "failed to copy {} -> {}",
            source_path.display(),
            target_path.display()
        )
    })?;
    fs::set_permissions(
        target_path,
        fs::Permissions::from_mode(permission_bits(expected)),
    )
    .with_context(|| format!("failed to set permissions on {}", target_path.display()))?;
    Ok(())
}

fn file_from_fd(fd: RawFd) -> Result<File, io::Error> {
    if fd < 0 {
        Err(io::Error::last_os_error())
    } else {
        let file = unsafe {
            // SAFETY: fd is newly owned by this function when non-negative.
            File::from_raw_fd(fd)
        };
        Ok(file)
    }
}

fn fstat_file(file: &File) -> Result<libc::stat, io::Error> {
    let mut stat = MaybeUninit::<libc::stat>::uninit();
    let rc = unsafe {
        // SAFETY: stat points to writable memory and file.as_raw_fd() is valid.
        libc::fstat(file.as_raw_fd(), stat.as_mut_ptr())
    };
    if rc == 0 {
        Ok(unsafe {
            // SAFETY: fstat initialized stat on success.
            stat.assume_init()
        })
    } else {
        Err(io::Error::last_os_error())
    }
}

fn fstatat_nofollow(dir_fd: RawFd, name: &OsStr) -> Result<libc::stat, io::Error> {
    let name_c = cstring_from_os_str(name).map_err(io::Error::other)?;
    let mut stat = MaybeUninit::<libc::stat>::uninit();
    let rc = unsafe {
        // SAFETY: name_c is NUL-terminated and stat points to writable memory.
        libc::fstatat(
            dir_fd,
            name_c.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if rc == 0 {
        Ok(unsafe {
            // SAFETY: fstatat initialized stat on success.
            stat.assume_init()
        })
    } else {
        Err(io::Error::last_os_error())
    }
}

fn stat_is_dir(stat: &libc::stat) -> bool {
    stat.st_mode & libc::S_IFMT == libc::S_IFDIR
}

fn stat_is_regular(stat: &libc::stat) -> bool {
    stat.st_mode & libc::S_IFMT == libc::S_IFREG
}

fn same_identity(left: &libc::stat, right: &libc::stat) -> bool {
    left.st_dev == right.st_dev && left.st_ino == right.st_ino
}

#[cfg(target_os = "linux")]
fn permission_bits(stat: &libc::stat) -> u32 {
    stat.st_mode & 0o777
}

#[cfg(not(target_os = "linux"))]
fn permission_bits(stat: &libc::stat) -> u32 {
    (stat.st_mode & 0o777) as u32
}

fn cstring_from_path(path: &Path) -> Result<CString> {
    cstring_from_os_str(path.as_os_str())
        .with_context(|| format!("path contains an interior NUL byte: {}", path.display()))
}

fn cstring_from_os_str(value: &OsStr) -> Result<CString> {
    CString::new(value.as_bytes()).context("path component contains an interior NUL byte")
}

fn clear_errno() {
    unsafe {
        // SAFETY: errno is thread-local on supported Unix targets.
        *errno_location() = 0;
    }
}

fn current_errno() -> i32 {
    unsafe {
        // SAFETY: errno is thread-local on supported Unix targets.
        *errno_location()
    }
}

#[cfg(target_os = "linux")]
unsafe fn errno_location() -> *mut libc::c_int {
    unsafe { libc::__errno_location() }
}

#[cfg(target_os = "macos")]
unsafe fn errno_location() -> *mut libc::c_int {
    unsafe { libc::__error() }
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

    #[test]
    fn activation_race_preserves_concurrently_created_empty_target() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let library = tmp.path().join("library");
        let target = tmp.path().join("target");
        fs::create_dir_all(&library).unwrap();
        fs::create_dir_all(&target).unwrap();
        let canonical = create_skill(&library, "skill-a");
        let raced_target = target.join("skill-a");

        let plan =
            plan(&config_dir, &target, &canonical, "skill-a").with_before_activate(move |_| {
                fs::create_dir(&raced_target).unwrap();
            });
        let lock_path = transition_path(&config_dir, &plan.target_name, &plan.skill);

        let result = plan.apply(false).unwrap();
        assert_eq!(result.state, DeploymentState::Foreign);
        assert!(result.skipped);
        assert!(target.join("skill-a").is_dir());
        assert!(
            fs::read_dir(target.join("skill-a"))
                .unwrap()
                .next()
                .is_none(),
            "the concurrently-created empty directory must not be replaced"
        );
        assert!(!plan.record_path().exists());
        assert!(!lock_path.exists());
        assert!(
            fs::read_dir(&target)
                .unwrap()
                .filter_map(|entry| entry.ok())
                .all(|entry| !entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".tome-stage-")),
            "abandoned staging directory should be cleaned up"
        );
    }

    #[test]
    fn copy_rejects_file_replaced_with_symlink_before_open() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("source");
        let destination = tmp.path().join("destination");
        fs::create_dir_all(&destination).unwrap();
        let canonical = create_skill(&source, "skill-a");
        let outside = tmp.path().join("outside.txt");
        fs::write(&outside, "outside").unwrap();

        let err = copy_regular_tree_with_hook(&canonical, &destination, |path| {
            if path.file_name() == Some(OsStr::new("SKILL.md")) {
                fs::remove_file(path).unwrap();
                std::os::unix::fs::symlink(&outside, path).unwrap();
            }
            Ok(())
        })
        .unwrap_err();

        assert!(
            format!("{err:#}").contains("failed to open")
                || format!("{err:#}").contains("source file changed while staging")
        );
        assert!(!destination.join("SKILL.md").exists());
    }

    #[test]
    fn copy_rejects_file_replaced_with_fifo_before_open() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("source");
        let destination = tmp.path().join("destination");
        fs::create_dir_all(&destination).unwrap();
        let canonical = create_skill(&source, "skill-a");

        let err = copy_regular_tree_with_hook(&canonical, &destination, |path| {
            if path.file_name() == Some(OsStr::new("SKILL.md")) {
                fs::remove_file(path).unwrap();
                let status = std::process::Command::new("mkfifo")
                    .arg(path)
                    .status()
                    .unwrap();
                assert!(status.success(), "mkfifo should create a FIFO fixture");
            }
            Ok(())
        })
        .unwrap_err();

        assert!(format!("{err:#}").contains("source file changed while staging"));
        assert!(!destination.join("SKILL.md").exists());
    }
}
