//! Shared test fixtures, helpers, and assertions for the per-domain
//! `cli_*.rs` integration test files (HARD-13).
//!
//! Each `tests/cli_*.rs` file is its own compilation unit, so any helper
//! a single file doesn't use compiles to a dead-code warning. We mark the
//! whole module `#[allow(dead_code)]` rather than maintaining per-helper
//! attributes — cargo's idiomatic `tests/common/mod.rs` pattern.
//!
//! Helper scope:
//!
//! - **`tome()`**, **`snapshot_settings`** — used by every domain.
//! - **`write_config`**, **`write_config_with_target`**, **`create_skill`** —
//!   bare-bones fixture builders for sync / list / status / doctor / lint.
//! - **`TestEnv`** + **`TestEnvBuilder`** — richer fixture with sources,
//!   targets, machine.toml, and lockfile pre-population. Used by sync,
//!   reassign, fork, status, doctor, lifecycle, edge, eject, lint, backup.
//! - **`Phase14Fixture`** + helpers — pre-staged Unowned / Owned manifest
//!   fixtures used by the remove-skill, reassign, status, and doctor tests
//!   that target the Phase 14 contracts.
//!
//! Single-use helpers (e.g. `git_init` for sync's git-commit tests,
//! `remove_test_env` for cli_remove, `reassign_test_env` for cli_reassign,
//! `parse_generated_config` / `assert_config_roundtrips` for cli_init,
//! and `V09Fixture` / `build_v09_fixture` for cli_migrate_library) live
//! with their consumer rather than here.

#![allow(dead_code)]

use assert_cmd::{Command, cargo_bin_cmd};
use assert_fs::TempDir;
use insta::Settings;
use std::path::{Path, PathBuf};

pub fn tome() -> Command {
    cargo_bin_cmd!("tome")
}

/// Create insta Settings with path redaction for the given tmpdir.
pub fn snapshot_settings(tmp: &TempDir) -> Settings {
    let mut settings = Settings::clone_current();
    let tmp_str = tmp.path().display().to_string();
    // Escape regex metacharacters in the tmpdir path
    let escaped = tmp_str
        .chars()
        .flat_map(|c| {
            if r"\.+*?()|[]{}^$-".contains(c) {
                vec!['\\', c]
            } else {
                vec![c]
            }
        })
        .collect::<String>();
    settings.add_filter(&escaped, "[TMPDIR]");
    settings.add_filter(r" +\n", "\n");
    settings.set_snapshot_path("snapshots");
    settings
}

pub fn write_config(dir: &std::path::Path, sources_toml: &str) -> std::path::PathBuf {
    let config_path = dir.join("config.toml");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    std::fs::write(
        &config_path,
        format!(
            "library_dir = \"{}\"\n{}",
            library_dir.display(),
            sources_toml
        ),
    )
    .unwrap();
    config_path
}

pub fn write_config_with_target(
    dir: &std::path::Path,
    sources_toml: &str,
    target_dir: &std::path::Path,
) -> std::path::PathBuf {
    let config_path = dir.join("config.toml");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    std::fs::write(
        &config_path,
        format!(
            "library_dir = \"{}\"\n{}\n[directories.test-target]\npath = \"{}\"\ntype = \"directory\"\nrole = \"target\"\n",
            library_dir.display(),
            sources_toml,
            target_dir.display()
        ),
    )
    .unwrap();
    config_path
}

pub fn create_skill(dir: &std::path::Path, name: &str) {
    let skill_dir = dir.join(name);
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        format!("---\nname: {name}\n---\n# {name}\nA test skill."),
    )
    .unwrap();
}

// === TestEnv Builder ===

pub struct TestEnv {
    pub tmp: TempDir,
    pub config_path: PathBuf,
    pub machine_path: Option<PathBuf>,
    pub library_dir: PathBuf,
    pub source_dirs: Vec<(String, PathBuf)>,
    pub target_dirs: Vec<(String, PathBuf)>,
}

pub struct TestEnvBuilder {
    sources: Vec<(String, String)>,
    targets: Vec<String>,
    skills: Vec<(String, String, Option<String>)>,
    managed_skills: Vec<(String, String, String, String)>,
    disabled_skills: Vec<String>,
    disabled_targets: Vec<String>,
    lockfile_content: Option<String>,
}

impl TestEnvBuilder {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            targets: Vec::new(),
            skills: Vec::new(),
            managed_skills: Vec::new(),
            disabled_skills: Vec::new(),
            disabled_targets: Vec::new(),
            lockfile_content: None,
        }
    }

    pub fn source(mut self, name: &str, source_type: &str) -> Self {
        self.sources
            .push((name.to_string(), source_type.to_string()));
        self
    }

    pub fn target(mut self, name: &str) -> Self {
        self.targets.push(name.to_string());
        self
    }

    pub fn skill(mut self, name: &str, source: &str) -> Self {
        self.skills
            .push((name.to_string(), source.to_string(), None));
        self
    }

    pub fn skill_with_content(mut self, name: &str, source: &str, content: &str) -> Self {
        self.skills.push((
            name.to_string(),
            source.to_string(),
            Some(content.to_string()),
        ));
        self
    }

    pub fn managed_skill(
        mut self,
        name: &str,
        source: &str,
        registry: &str,
        version: &str,
    ) -> Self {
        self.managed_skills.push((
            name.to_string(),
            source.to_string(),
            registry.to_string(),
            version.to_string(),
        ));
        self
    }

    pub fn disable_skill(mut self, name: &str) -> Self {
        self.disabled_skills.push(name.to_string());
        self
    }

    pub fn disable_target(mut self, name: &str) -> Self {
        self.disabled_targets.push(name.to_string());
        self
    }

    pub fn lockfile(mut self, json: &str) -> Self {
        self.lockfile_content = Some(json.to_string());
        self
    }

    pub fn build(self) -> TestEnv {
        let tmp = TempDir::new().unwrap();
        let library_dir = tmp.path().join("library");
        std::fs::create_dir_all(&library_dir).unwrap();

        let mut source_dirs = Vec::new();
        let mut target_dirs = Vec::new();
        let mut config_toml = format!("library_dir = \"{}\"\n\n", library_dir.display());

        // Create sources
        for (name, source_type) in &self.sources {
            let source_dir = tmp.path().join("sources").join(name);
            std::fs::create_dir_all(&source_dir).unwrap();

            if source_type == "claude-plugins" {
                // Build installed_plugins.json v2 format
                let mut plugins_map = serde_json::Map::new();
                for (skill_name, skill_source, registry, version) in &self.managed_skills {
                    if skill_source == name {
                        let install_dir = source_dir.join("installs").join(skill_name);
                        let skills_subdir = install_dir.join("skills").join(skill_name);
                        std::fs::create_dir_all(&skills_subdir).unwrap();
                        std::fs::write(
                            skills_subdir.join("SKILL.md"),
                            format!(
                                "---\nname: {skill_name}\n---\n# {skill_name}\nA managed skill."
                            ),
                        )
                        .unwrap();
                        let record = serde_json::json!({
                            "installPath": install_dir.display().to_string(),
                            "version": version
                        });
                        plugins_map
                            .entry(registry.clone())
                            .or_insert_with(|| serde_json::json!([]))
                            .as_array_mut()
                            .unwrap()
                            .push(record);
                    }
                }
                let json = serde_json::json!({
                    "version": 2,
                    "plugins": plugins_map
                });
                std::fs::write(
                    source_dir.join("installed_plugins.json"),
                    serde_json::to_string_pretty(&json).unwrap(),
                )
                .unwrap();

                config_toml.push_str(&format!(
                    "[directories.{name}]\npath = \"{}\"\ntype = \"claude-plugins\"\n\n",
                    source_dir.display()
                ));
            } else {
                // Directory source — create skills
                for (skill_name, skill_source, content) in &self.skills {
                    if skill_source == name {
                        let skill_dir = source_dir.join(skill_name);
                        std::fs::create_dir_all(&skill_dir).unwrap();
                        let skill_content = content.clone().unwrap_or_else(|| {
                            format!("---\nname: {skill_name}\n---\n# {skill_name}\nA test skill.")
                        });
                        std::fs::write(skill_dir.join("SKILL.md"), skill_content).unwrap();
                    }
                }

                config_toml.push_str(&format!(
                    "[directories.{name}]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n\n",
                    source_dir.display()
                ));
            }

            source_dirs.push((name.clone(), source_dir));
        }

        // Create targets
        for name in &self.targets {
            let target_dir = tmp.path().join("targets").join(name);
            std::fs::create_dir_all(&target_dir).unwrap();
            config_toml.push_str(&format!(
                "[directories.{name}]\npath = \"{}\"\ntype = \"directory\"\nrole = \"target\"\n\n",
                target_dir.display()
            ));
            target_dirs.push((name.clone(), target_dir));
        }

        // Write config
        let config_path = tmp.path().join("config.toml");
        std::fs::write(&config_path, &config_toml).unwrap();

        // Write machine prefs if needed
        let machine_path = if !self.disabled_skills.is_empty() || !self.disabled_targets.is_empty()
        {
            let path = tmp.path().join("machine.toml");
            let mut content = String::new();
            if !self.disabled_skills.is_empty() {
                let items: Vec<String> = self
                    .disabled_skills
                    .iter()
                    .map(|s| format!("\"{s}\""))
                    .collect();
                content.push_str(&format!("disabled = [{}]\n", items.join(", ")));
            }
            if !self.disabled_targets.is_empty() {
                let items: Vec<String> = self
                    .disabled_targets
                    .iter()
                    .map(|s| format!("\"{s}\""))
                    .collect();
                content.push_str(&format!("disabled_directories = [{}]\n", items.join(", ")));
            }
            std::fs::write(&path, content).unwrap();
            Some(path)
        } else {
            None
        };

        // Write lockfile if provided
        if let Some(lockfile) = &self.lockfile_content {
            std::fs::write(tmp.path().join("tome.lock"), lockfile).unwrap();
        }

        TestEnv {
            tmp,
            config_path,
            machine_path,
            library_dir,
            source_dirs,
            target_dirs,
        }
    }
}

impl Default for TestEnvBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TestEnv {
    pub fn cmd(&self) -> Command {
        let mut cmd = cargo_bin_cmd!("tome");
        cmd.args(["--config", self.config_path.to_str().unwrap()]);
        cmd.env("NO_COLOR", "1");
        cmd
    }

    pub fn cmd_with_machine(&self) -> Command {
        let mut cmd = self.cmd();
        if let Some(ref machine_path) = self.machine_path {
            cmd.args(["--machine", machine_path.to_str().unwrap()]);
        }
        cmd
    }

    pub fn library_dir(&self) -> &Path {
        &self.library_dir
    }

    pub fn source_dir(&self, name: &str) -> &Path {
        &self
            .source_dirs
            .iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("source '{name}' not found"))
            .1
    }

    pub fn target_dir(&self, name: &str) -> &Path {
        &self
            .target_dirs
            .iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("target '{name}' not found"))
            .1
    }

    pub fn tome_home(&self) -> &Path {
        self.tmp.path()
    }

    pub fn snapshot_settings(&self) -> Settings {
        snapshot_settings(&self.tmp)
    }

    pub fn lockfile_path(&self) -> PathBuf {
        self.tome_home().join("tome.lock")
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.tome_home().join(".tome-manifest.json")
    }

    pub fn add_skill(&self, name: &str, source: &str) {
        let source_dir = self.source_dir(source);
        let skill_dir = source_dir.join(name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            format!("---\nname: {name}\n---\n# {name}\nA test skill."),
        )
        .unwrap();
    }

    pub fn modify_skill(&self, name: &str, source: &str, content: &str) {
        let source_dir = self.source_dir(source);
        std::fs::write(source_dir.join(name).join("SKILL.md"), content).unwrap();
    }

    pub fn remove_skill(&self, name: &str, source: &str) {
        let source_dir = self.source_dir(source);
        std::fs::remove_dir_all(source_dir.join(name)).unwrap();
    }
}

// ===========================================================================
// Phase 14 fixture — pre-staged Unowned / Owned manifest skills.
//
// These helpers seed `.tome-manifest.json` directly so individual tests can
// stage Unowned skills (`source_name = None`) without first having to sync,
// then remove a directory, then re-sync. The skills they describe are real
// directories on disk inside the library_dir so commands that touch
// filesystem state (remove skill, reassign content-hash check) operate on
// actual data — only the manifest provenance is fabricated.
//
// Used by: cli_remove.rs (remove skill), cli_reassign.rs, cli_status.rs,
// cli_doctor.rs.
// ===========================================================================

/// Phase 14 fixture builder. Holds tome_home + config_path + library_dir
/// and the on-disk locations of pre-staged skills so individual tests
/// can assert filesystem-level state changes.
pub struct Phase14Fixture {
    /// Held to keep the TempDir alive for the duration of the test.
    pub tmp: TempDir,
    pub tome_home: PathBuf,
    pub config_path: PathBuf,
    pub library_dir: PathBuf,
    pub machine_path: PathBuf,
    /// Optional pre-staged target dir (for reassign / distribution-symlink tests).
    pub target_dir: Option<PathBuf>,
}

impl Phase14Fixture {
    pub fn cmd(&self) -> Command {
        let mut cmd = cargo_bin_cmd!("tome");
        cmd.args(["--tome-home", self.tome_home.to_str().unwrap()]);
        cmd.args(["--config", self.config_path.to_str().unwrap()]);
        cmd.args(["--machine", self.machine_path.to_str().unwrap()]);
        cmd.env("NO_COLOR", "1");
        cmd
    }

    pub fn manifest_value(&self) -> serde_json::Value {
        let raw = std::fs::read_to_string(self.tome_home.join(".tome-manifest.json")).unwrap();
        serde_json::from_str(&raw).unwrap()
    }
}

/// Build a manifest entry value. `source_name = None` produces an Unowned
/// entry (the key is omitted, matching the `skip_serializing_if` shape on
/// `SkillEntry`). When `previous_source` is set, it's emitted regardless of
/// owned-ness.
pub fn phase14_manifest_entry(
    source_path: &Path,
    source_name: Option<&str>,
    previous_source: Option<&str>,
    content_hash: &str,
) -> serde_json::Value {
    let mut entry = serde_json::json!({
        "source_path": source_path.to_string_lossy(),
        "content_hash": content_hash,
        "synced_at": "2026-05-07T00:00:00Z",
        "managed": false,
    });
    if let Some(name) = source_name {
        entry["source_name"] = serde_json::Value::String(name.to_string());
    }
    if let Some(prev) = previous_source {
        entry["previous_source"] = serde_json::Value::String(prev.to_string());
    }
    entry
}

/// Write a skill directory inside `library_dir/<name>/` containing a SKILL.md
/// with the given body, and return the SHA-256 content hash recorded by
/// `tome::hash_directory` so manifest fixtures stay consistent.
pub fn phase14_write_library_skill(library_dir: &Path, name: &str, body: &str) -> String {
    let dir = library_dir.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {name}\n---\n# {name}\n{body}"),
    )
    .unwrap();
    tome::hash_directory(&dir).unwrap().as_str().to_string()
}

/// Build a Phase 14 fixture with pre-staged Unowned skills.
///
/// `unowned_skills` = list of `(skill_name, previous_source_dir_name)` tuples.
/// Each is materialized as a real directory in `library_dir/<name>/` and an
/// Unowned manifest entry with `previous_source = Some(<dir>)`.
///
/// `synced_dirs` = list of `(dir_name, role)` tuples for `[directories.*]`
/// entries (role = "source" / "synced" / "target" / "managed"). Each gets a
/// real on-disk path under `tmp/<name>/`.
///
/// `owned_skills` = list of `(skill_name, source_dir_name)` Owned entries
/// (manifest entry with `source_name = Some(...)`, no `previous_source`).
pub fn phase14_build_fixture(
    synced_dirs: &[(&str, &str)],
    owned_skills: &[(&str, &str)],
    unowned_skills: &[(&str, &str)],
) -> Phase14Fixture {
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join("tome_home");
    let library_dir = tome_home.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();

    let mut config_toml = format!("library_dir = \"{}\"\n\n", library_dir.display());

    let mut first_target_dir: Option<PathBuf> = None;
    let mut dir_paths: std::collections::HashMap<String, PathBuf> =
        std::collections::HashMap::new();
    for (name, role) in synced_dirs {
        let dir_path = tmp.path().join(format!("dir-{name}"));
        std::fs::create_dir_all(&dir_path).unwrap();
        dir_paths.insert(name.to_string(), dir_path.clone());
        if first_target_dir.is_none() && (*role == "target" || *role == "synced") {
            first_target_dir = Some(dir_path.clone());
        }
        config_toml.push_str(&format!(
            "[directories.{name}]\npath = \"{}\"\ntype = \"directory\"\nrole = \"{role}\"\n\n",
            dir_path.display()
        ));
    }

    let mut skills_obj = serde_json::Map::new();
    for (name, source) in owned_skills {
        let body = format!("owned skill {name}");
        let content_hash = phase14_write_library_skill(&library_dir, name, &body);
        let source_path = dir_paths
            .get(*source)
            .map(|p| p.join(name))
            .unwrap_or_else(|| std::path::PathBuf::from(format!("/tmp/orig-{name}")));
        skills_obj.insert(
            name.to_string(),
            phase14_manifest_entry(&source_path, Some(source), None, &content_hash),
        );
    }
    for (name, previous) in unowned_skills {
        let body = format!("unowned skill {name}");
        let content_hash = phase14_write_library_skill(&library_dir, name, &body);
        // Use a synthetic source_path the dir no longer covers (Unowned skills
        // were materialised by an earlier sync; their source_path lives at the
        // old location which may no longer exist).
        let source_path = std::path::PathBuf::from(format!("/tmp/old/{previous}/{name}"));
        skills_obj.insert(
            name.to_string(),
            phase14_manifest_entry(&source_path, None, Some(previous), &content_hash),
        );
    }
    let manifest = serde_json::json!({ "skills": skills_obj });

    let config_path = tome_home.join("tome.toml");
    std::fs::write(&config_path, &config_toml).unwrap();

    let manifest_path = tome_home.join(".tome-manifest.json");
    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let machine_path = tmp.path().join("machine.toml");
    std::fs::write(&machine_path, "").unwrap();

    Phase14Fixture {
        tmp,
        tome_home,
        config_path,
        library_dir,
        machine_path,
        target_dir: first_target_dir,
    }
}
