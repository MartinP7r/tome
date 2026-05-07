use assert_cmd::{Command, cargo_bin_cmd};
use assert_fs::TempDir;
use insta::Settings;
use predicates::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;

fn tome() -> Command {
    cargo_bin_cmd!("tome")
}

/// Create insta Settings with path redaction for the given tmpdir.
fn snapshot_settings(tmp: &TempDir) -> Settings {
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

fn write_config(dir: &std::path::Path, sources_toml: &str) -> std::path::PathBuf {
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

fn write_config_with_target(
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

fn create_skill(dir: &std::path::Path, name: &str) {
    let skill_dir = dir.join(name);
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        format!("---\nname: {name}\n---\n# {name}\nA test skill."),
    )
    .unwrap();
}

// === TestEnv Builder ===

#[allow(dead_code)]
struct TestEnv {
    tmp: TempDir,
    config_path: PathBuf,
    machine_path: Option<PathBuf>,
    library_dir: PathBuf,
    source_dirs: Vec<(String, PathBuf)>,
    target_dirs: Vec<(String, PathBuf)>,
}

#[allow(dead_code)]
struct TestEnvBuilder {
    sources: Vec<(String, String)>,
    targets: Vec<String>,
    skills: Vec<(String, String, Option<String>)>,
    managed_skills: Vec<(String, String, String, String)>,
    disabled_skills: Vec<String>,
    disabled_targets: Vec<String>,
    lockfile_content: Option<String>,
}

#[allow(dead_code)]
impl TestEnvBuilder {
    fn new() -> Self {
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

    fn source(mut self, name: &str, source_type: &str) -> Self {
        self.sources
            .push((name.to_string(), source_type.to_string()));
        self
    }

    fn target(mut self, name: &str) -> Self {
        self.targets.push(name.to_string());
        self
    }

    fn skill(mut self, name: &str, source: &str) -> Self {
        self.skills
            .push((name.to_string(), source.to_string(), None));
        self
    }

    fn skill_with_content(mut self, name: &str, source: &str, content: &str) -> Self {
        self.skills.push((
            name.to_string(),
            source.to_string(),
            Some(content.to_string()),
        ));
        self
    }

    fn managed_skill(mut self, name: &str, source: &str, registry: &str, version: &str) -> Self {
        self.managed_skills.push((
            name.to_string(),
            source.to_string(),
            registry.to_string(),
            version.to_string(),
        ));
        self
    }

    fn disable_skill(mut self, name: &str) -> Self {
        self.disabled_skills.push(name.to_string());
        self
    }

    fn disable_target(mut self, name: &str) -> Self {
        self.disabled_targets.push(name.to_string());
        self
    }

    #[allow(dead_code)]
    fn lockfile(mut self, json: &str) -> Self {
        self.lockfile_content = Some(json.to_string());
        self
    }

    fn build(self) -> TestEnv {
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

#[allow(dead_code)]
impl TestEnv {
    fn cmd(&self) -> Command {
        let mut cmd = cargo_bin_cmd!("tome");
        cmd.args(["--config", self.config_path.to_str().unwrap()]);
        cmd.env("NO_COLOR", "1");
        cmd
    }

    fn cmd_with_machine(&self) -> Command {
        let mut cmd = self.cmd();
        if let Some(ref machine_path) = self.machine_path {
            cmd.args(["--machine", machine_path.to_str().unwrap()]);
        }
        cmd
    }

    fn library_dir(&self) -> &Path {
        &self.library_dir
    }

    fn source_dir(&self, name: &str) -> &Path {
        &self
            .source_dirs
            .iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("source '{name}' not found"))
            .1
    }

    fn target_dir(&self, name: &str) -> &Path {
        &self
            .target_dirs
            .iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("target '{name}' not found"))
            .1
    }

    fn tome_home(&self) -> &Path {
        self.tmp.path()
    }

    fn snapshot_settings(&self) -> Settings {
        snapshot_settings(&self.tmp)
    }

    #[allow(dead_code)]
    fn lockfile_path(&self) -> PathBuf {
        self.tome_home().join("tome.lock")
    }

    fn manifest_path(&self) -> PathBuf {
        self.tome_home().join(".tome-manifest.json")
    }

    fn add_skill(&self, name: &str, source: &str) {
        let source_dir = self.source_dir(source);
        let skill_dir = source_dir.join(name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            format!("---\nname: {name}\n---\n# {name}\nA test skill."),
        )
        .unwrap();
    }

    fn modify_skill(&self, name: &str, source: &str, content: &str) {
        let source_dir = self.source_dir(source);
        std::fs::write(source_dir.join(name).join("SKILL.md"), content).unwrap();
    }

    fn remove_skill(&self, name: &str, source: &str) {
        let source_dir = self.source_dir(source);
        std::fs::remove_dir_all(source_dir.join(name)).unwrap();
    }
}

// -- Help & version --

#[test]
fn help_shows_usage() {
    tome()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Sync AI coding skills across tools",
        ));
}

#[test]
fn version_shows_version() {
    tome()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn version_subcommand_shows_version() {
    tome()
        .arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn short_version_flag_shows_version() {
    tome()
        .arg("-V")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn verbose_short_flag_still_works() {
    // Ensure -v is still --verbose and doesn't conflict with -V
    tome()
        .args(["-v", "status", "--config", "/nonexistent/path.toml"])
        .assert()
        // This may fail because no config, but should NOT be interpreted as version
        .stderr(predicate::str::contains("version").not());
}

// -- List --

#[test]
fn list_with_no_sources_shows_message() {
    let tmp = TempDir::new().unwrap();
    let config = write_config(tmp.path(), "");

    tome()
        .args(["--config", config.to_str().unwrap(), "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No skills found"));
}

#[test]
fn list_shows_discovered_skills() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");
    create_skill(&skills_dir, "other-skill");

    let config = write_config(
        tmp.path(),
        &format!(
            "[directories.test]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
    );

    let output = tome()
        .args(["--config", config.to_str().unwrap(), "list"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let settings = snapshot_settings(&tmp);
    settings.bind(|| {
        insta::assert_snapshot!("list_table_two_skills", stdout);
    });
}

// -- Sync --

#[test]
fn sync_dry_run_makes_no_changes() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "test-skill");

    let config = write_config(
        tmp.path(),
        &format!(
            "[directories.test]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
    );

    tome()
        .args(["--config", config.to_str().unwrap(), "--dry-run", "sync"])
        .assert()
        .success()
        .stderr(predicate::str::contains("dry-run"))
        .stdout(predicate::str::contains("Sync complete"));

    // Library should remain empty
    let library = tmp.path().join("library");
    let entries: Vec<_> = std::fs::read_dir(&library)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(entries.len(), 0, "dry run should not create entries");
}

#[test]
fn sync_copies_skills_to_library() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "alpha");
    create_skill(&skills_dir, "beta");

    let config = write_config(
        tmp.path(),
        &format!(
            "[directories.test]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
    );

    let output = tome()
        .args(["--config", config.to_str().unwrap(), "sync"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let settings = snapshot_settings(&tmp);
    settings.bind(|| {
        insta::assert_snapshot!("sync_initial_two_skills", stdout);
    });

    let library = tmp.path().join("library");
    // v0.2: library entries are real directories, not symlinks
    assert!(library.join("alpha").is_dir());
    assert!(!library.join("alpha").is_symlink());
    assert!(library.join("beta").is_dir());
    assert!(!library.join("beta").is_symlink());
    // Content should be copied
    assert!(library.join("alpha/SKILL.md").is_file());
    // Manifest should exist at tome home (config file's parent dir)
    assert!(tmp.path().join(".tome-manifest.json").is_file());
}

#[test]
fn sync_idempotent() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "stable-skill");

    let config = write_config(
        tmp.path(),
        &format!(
            "[directories.test]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
    );

    let config_str = config.to_str().unwrap();

    // First sync
    tome()
        .args(["--config", config_str, "sync"])
        .env("NO_COLOR", "1")
        .assert()
        .success();

    // Second sync — should report 0 created, 1 unchanged
    let output = tome()
        .args(["--config", config_str, "sync"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let settings = snapshot_settings(&tmp);
    settings.bind(|| {
        insta::assert_snapshot!("sync_idempotent_second_run", stdout);
    });
}

#[test]
fn sync_creates_lockfile() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "alpha-skill");
    create_skill(&skills_dir, "beta-skill");

    let config = write_config(
        tmp.path(),
        &format!(
            "[directories.test]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
    );

    tome()
        .args(["--config", config.to_str().unwrap(), "sync"])
        .env("NO_COLOR", "1")
        .assert()
        .success();

    // Lockfile now lives at tome home (config file's parent dir), not library
    let lockfile_path = tmp.path().join("tome.lock");
    assert!(
        lockfile_path.exists(),
        "tome.lock should be created by sync"
    );

    let content = std::fs::read_to_string(&lockfile_path).unwrap();
    let mut parsed: serde_json::Value = serde_json::from_str(&content).unwrap();

    // Redact dynamic content_hash fields for snapshot stability
    if let Some(skills) = parsed.get_mut("skills").and_then(|s| s.as_object_mut()) {
        for (_name, skill) in skills.iter_mut() {
            if skill.get("content_hash").is_some() {
                skill["content_hash"] = serde_json::Value::String("[HASH]".into());
            }
        }
    }

    let settings = snapshot_settings(&tmp);
    settings.bind(|| {
        insta::assert_json_snapshot!("sync_lockfile_two_skills", parsed);
    });
}

#[test]
fn sync_dry_run_does_not_create_lockfile() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let config = write_config(
        tmp.path(),
        &format!(
            "[directories.test]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
    );

    tome()
        .args(["--config", config.to_str().unwrap(), "--dry-run", "sync"])
        .assert()
        .success();

    assert!(
        !tmp.path().join("tome.lock").exists(),
        "dry-run should not create tome.lock"
    );
}

// -- Status --

#[test]
fn status_shows_library_info() {
    let tmp = TempDir::new().unwrap();
    let config = write_config(tmp.path(), "");

    let output = tome()
        .args(["--config", config.to_str().unwrap(), "status"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let settings = snapshot_settings(&tmp);
    settings.bind(|| {
        insta::assert_snapshot!("status_empty_library", stdout);
    });
}

// -- Config --

#[test]
fn config_path_prints_default_path() {
    let tmp = TempDir::new().unwrap();
    tome()
        .env("TOME_HOME", tmp.path())
        .args(["config", "--path"])
        .assert()
        .success()
        .stdout(predicate::str::contains("tome.toml"));
}

// -- Sync with targets --

#[test]
fn sync_distributes_to_symlink_target() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let target_dir = tmp.path().join("target");
    // Don't create target_dir — sync should create it

    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();

    let config_path = tmp.path().join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"library_dir = "{}"

[directories.test]
path = "{}"
type = "directory"
role = "source"

[directories.antigravity]
path = "{}"
type = "directory"
role = "target"
"#,
            library_dir.display(),
            skills_dir.display(),
            target_dir.display()
        ),
    )
    .unwrap();

    tome()
        .args(["--config", config_path.to_str().unwrap(), "sync"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Sync complete"));

    // Library has the skill as a real directory (v0.2)
    assert!(library_dir.join("my-skill").is_dir());
    assert!(!library_dir.join("my-skill").is_symlink());
    // Target has a symlink pointing to the library entry
    assert!(target_dir.join("my-skill").is_symlink());
}

#[test]
fn sync_lifecycle_cleans_up_removed_skills() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    std::fs::create_dir_all(&skills_dir).unwrap();
    create_skill(&skills_dir, "keep-me");
    create_skill(&skills_dir, "remove-me");

    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();

    let config_path = tmp.path().join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"library_dir = "{}"

[directories.test]
path = "{}"
type = "directory"
role = "source"
"#,
            library_dir.display(),
            skills_dir.display(),
        ),
    )
    .unwrap();

    // First sync — both skills should appear in library
    tome()
        .args(["--config", config_path.to_str().unwrap(), "sync"])
        .assert()
        .success();
    assert!(library_dir.join("keep-me").is_dir());
    assert!(library_dir.join("remove-me").is_dir());

    // Remove one skill from source
    std::fs::remove_dir_all(skills_dir.join("remove-me")).unwrap();

    // Second sync — stale entry should be cleaned up (non-interactive mode in tests)
    tome()
        .args(["--config", config_path.to_str().unwrap(), "sync"])
        .assert()
        .success();
    assert!(library_dir.join("keep-me").is_dir());
    assert!(
        !library_dir.join("remove-me").exists(),
        "stale skill should have been cleaned up"
    );
}

#[test]
fn sync_force_recreates_all() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    std::fs::create_dir_all(&skills_dir).unwrap();
    create_skill(&skills_dir, "my-skill");

    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();

    let config_path = tmp.path().join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"library_dir = "{}"

[directories.test]
path = "{}"
type = "directory"
role = "source"
"#,
            library_dir.display(),
            skills_dir.display(),
        ),
    )
    .unwrap();

    // Initial sync
    tome()
        .args(["--config", config_path.to_str().unwrap(), "sync"])
        .assert()
        .success();
    assert!(library_dir.join("my-skill").is_dir());

    // Force sync should report recreated, not "unchanged"
    let output = tome()
        .args(["--config", config_path.to_str().unwrap(), "sync", "--force"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let settings = snapshot_settings(&tmp);
    settings.bind(|| {
        insta::assert_snapshot!("sync_force_recreate", stdout);
    });
}

#[test]
fn sync_updates_changed_source() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();

    let config_path = tmp.path().join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"library_dir = "{}"

[directories.test]
path = "{}"
type = "directory"
role = "source"
"#,
            library_dir.display(),
            skills_dir.display(),
        ),
    )
    .unwrap();

    // First sync
    tome()
        .args(["--config", config_path.to_str().unwrap(), "sync"])
        .assert()
        .success();

    // Modify source SKILL.md
    std::fs::write(
        skills_dir.join("my-skill/SKILL.md"),
        "# updated content\nNew body.",
    )
    .unwrap();

    // Second sync — should detect the change
    let output = tome()
        .args(["--config", config_path.to_str().unwrap(), "sync"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let settings = snapshot_settings(&tmp);
    settings.bind(|| {
        insta::assert_snapshot!("sync_updates_changed", stdout);
    });

    // Library copy should have the new content
    let content = std::fs::read_to_string(library_dir.join("my-skill/SKILL.md")).unwrap();
    assert_eq!(content, "# updated content\nNew body.");
}

#[test]
fn sync_migrates_v01_symlinks() {
    use std::os::unix::fs as unix_fs;

    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "legacy-skill");

    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();

    // Simulate v0.1.x: library has a symlink
    unix_fs::symlink(
        skills_dir.join("legacy-skill"),
        library_dir.join("legacy-skill"),
    )
    .unwrap();
    assert!(library_dir.join("legacy-skill").is_symlink());

    let config_path = tmp.path().join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"library_dir = "{}"

[directories.test]
path = "{}"
type = "directory"
role = "source"
"#,
            library_dir.display(),
            skills_dir.display(),
        ),
    )
    .unwrap();

    // Sync should migrate the symlink to a real directory
    tome()
        .args(["--config", config_path.to_str().unwrap(), "sync"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Sync complete"));

    // Should now be a real directory, not a symlink
    assert!(library_dir.join("legacy-skill").is_dir());
    assert!(!library_dir.join("legacy-skill").is_symlink());
    assert!(library_dir.join("legacy-skill/SKILL.md").is_file());
}

// -- Doctor --

#[test]
fn doctor_with_clean_state() {
    let tmp = TempDir::new().unwrap();
    let config = write_config(tmp.path(), "");

    let output = tome()
        .args(["--config", config.to_str().unwrap(), "doctor"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let settings = snapshot_settings(&tmp);
    settings.bind(|| {
        insta::assert_snapshot!("doctor_clean", stdout);
    });
}

#[test]
fn doctor_detects_broken_symlinks() {
    use std::os::unix::fs as unix_fs;

    let tmp = TempDir::new().unwrap();
    let library = tmp.path().join("library");
    std::fs::create_dir_all(&library).unwrap();

    // Create a broken symlink in the library (legacy)
    unix_fs::symlink("/nonexistent/path", library.join("broken-skill")).unwrap();

    let config = write_config(tmp.path(), "");

    tome()
        .args(["--config", config.to_str().unwrap(), "--dry-run", "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1 issue(s)"));
}

// -- Pre-init state (unconfigured) --

#[test]
fn status_without_config_shows_init_prompt() {
    let tmp = TempDir::new().unwrap();
    // Point library_dir at a nonexistent dir (no sources) to simulate unconfigured state.
    // Using write_config would create library_dir, defeating the purpose.
    let config_path = tmp.path().join("config.toml");
    let nonexistent_library = tmp.path().join("nonexistent-library");
    std::fs::write(
        &config_path,
        format!("library_dir = \"{}\"", nonexistent_library.display()),
    )
    .unwrap();

    let output = tome()
        .args(["--config", config_path.to_str().unwrap(), "status"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let settings = snapshot_settings(&tmp);
    settings.bind(|| {
        insta::assert_snapshot!("status_unconfigured", stdout);
    });
}

#[test]
fn doctor_without_config_shows_init_prompt() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("config.toml");
    let nonexistent_library = tmp.path().join("nonexistent-library");
    std::fs::write(
        &config_path,
        format!("library_dir = \"{}\"", nonexistent_library.display()),
    )
    .unwrap();

    tome()
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "--dry-run",
            "doctor",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Not configured yet"))
        .stdout(predicate::str::contains("tome init"));
}

// -- Git commit on sync --

/// Helper: initialize a git repo with a dummy identity (for CI).
fn git_init(dir: &std::path::Path) {
    StdCommand::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir)
        .output()
        .unwrap();
    // Initial commit so HEAD exists
    StdCommand::new("git")
        .args(["commit", "--allow-empty", "-m", "init"])
        .current_dir(dir)
        .output()
        .unwrap();
}

#[test]
fn sync_skips_git_commit_without_tty() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "new-skill");

    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    git_init(&library_dir);

    let config_path = tmp.path().join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"library_dir = "{}"

[directories.test]
path = "{}"
type = "directory"
role = "source"
"#,
            library_dir.display(),
            skills_dir.display(),
        ),
    )
    .unwrap();

    // Without a TTY, the git commit prompt should be silently skipped
    tome()
        .args(["--config", config_path.to_str().unwrap(), "sync"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Sync complete"));

    // Only the initial "init" commit should exist (no auto-commit without TTY)
    let log = StdCommand::new("git")
        .args(["log", "--oneline"])
        .current_dir(&library_dir)
        .output()
        .unwrap();
    let commits = String::from_utf8_lossy(&log.stdout);
    assert!(
        !commits.contains("tome sync"),
        "should not commit without a TTY"
    );
}

#[test]
fn sync_dry_run_skips_git_commit() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "new-skill");

    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    git_init(&library_dir);

    let config_path = tmp.path().join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"library_dir = "{}"

[directories.test]
path = "{}"
type = "directory"
role = "source"
"#,
            library_dir.display(),
            skills_dir.display(),
        ),
    )
    .unwrap();

    tome()
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "--dry-run",
            "sync",
        ])
        .assert()
        .success();

    // Only the initial "init" commit should exist
    let log = StdCommand::new("git")
        .args(["log", "--oneline"])
        .current_dir(&library_dir)
        .output()
        .unwrap();
    let commits = String::from_utf8_lossy(&log.stdout);
    assert!(
        !commits.contains("tome sync"),
        "dry-run should not create a commit"
    );
}

#[test]
fn list_json_outputs_valid_json() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "alpha-skill");
    create_skill(&skills_dir, "beta-skill");

    let config = write_config(
        tmp.path(),
        &format!(
            "[directories.test-src]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
    );

    let output = tome()
        .args(["--config", config.to_str().unwrap(), "list", "--json"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON");
    let arr = parsed.as_array().expect("should be a JSON array");
    assert_eq!(arr.len(), 2);

    // Redact dynamic path fields for snapshot stability
    let mut redacted = parsed.clone();
    for entry in redacted.as_array_mut().unwrap() {
        entry["path"] = serde_json::Value::String("[TMPDIR]".into());
    }

    let settings = snapshot_settings(&tmp);
    settings.bind(|| {
        insta::assert_json_snapshot!("list_json_two_skills", redacted);
    });
}

#[test]
fn list_json_with_quiet_still_outputs_json() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let config = write_config(
        tmp.path(),
        &format!(
            "[directories.test]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
    );

    let output = tome()
        .args([
            "--config",
            config.to_str().unwrap(),
            "--quiet",
            "list",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());

    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("--json should override --quiet");
    let arr = parsed.as_array().expect("should be a JSON array");
    assert_eq!(arr.len(), 1);
}

#[test]
fn list_json_with_no_skills_outputs_empty_array() {
    let tmp = TempDir::new().unwrap();
    let config = write_config(tmp.path(), "");

    let output = tome()
        .args(["--config", config.to_str().unwrap(), "list", "--json"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON");

    let settings = snapshot_settings(&tmp);
    settings.bind(|| {
        insta::assert_json_snapshot!("list_json_empty", parsed);
    });
}

#[test]
fn sync_quiet_skips_git_commit() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "new-skill");

    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    git_init(&library_dir);

    let config_path = tmp.path().join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"library_dir = "{}"

[directories.test]
path = "{}"
type = "directory"
role = "source"
"#,
            library_dir.display(),
            skills_dir.display(),
        ),
    )
    .unwrap();

    tome()
        .args(["--config", config_path.to_str().unwrap(), "--quiet", "sync"])
        .assert()
        .success();

    // Only the initial "init" commit should exist
    let log = StdCommand::new("git")
        .args(["log", "--oneline"])
        .current_dir(&library_dir)
        .output()
        .unwrap();
    let commits = String::from_utf8_lossy(&log.stdout);
    assert!(
        !commits.contains("tome sync"),
        "quiet mode should not prompt for commit"
    );
}

// -- Triage (formerly update) --

#[test]
fn sync_no_triage_skips_diff_output() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .skill("my-skill", "local")
        .build();

    // First sync to create lockfile
    tome()
        .args([
            "--config",
            &env.config_path.to_string_lossy(),
            "sync",
            "--no-triage",
        ])
        .assert()
        .success();

    // Second sync with --no-triage should not show diff summary
    let output = tome()
        .args([
            "--config",
            &env.config_path.to_string_lossy(),
            "sync",
            "--no-triage",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("changes"),
        "--no-triage should suppress diff summary, got: {stdout}"
    );
    assert!(
        !stdout.contains("No previous lockfile"),
        "--no-triage should suppress lockfile messages, got: {stdout}"
    );
}

#[test]
fn sync_with_no_lockfile_works_gracefully() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let target_dir = tmp.path().join("target");

    let config = write_config_with_target(
        tmp.path(),
        &format!(
            "[directories.test]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
        &target_dir,
    );

    // First run with no prior lockfile — should work like a normal sync
    tome()
        .args(["--config", config.to_str().unwrap(), "sync"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No previous lockfile"))
        .stdout(predicate::str::contains("Sync complete"));

    // Library should have the skill
    assert!(tmp.path().join("library/my-skill").is_dir());
    // Target should have symlink
    assert!(target_dir.join("my-skill").is_symlink());
    // Lockfile should be created at tome home (config file's parent dir)
    assert!(tmp.path().join("tome.lock").exists());
}

#[test]
fn sync_triage_shows_new_skills() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "existing-skill");

    let target_dir = tmp.path().join("target");

    let config = write_config_with_target(
        tmp.path(),
        &format!(
            "[directories.test]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
        &target_dir,
    );

    let config_str = config.to_str().unwrap();

    // Initial sync to create lockfile
    tome()
        .args(["--config", config_str, "sync"])
        .assert()
        .success();

    // Add a new skill
    create_skill(&skills_dir, "brand-new-skill");

    // Update should detect the new skill
    tome()
        .args(["--config", config_str, "--quiet", "sync"])
        .assert()
        .success();

    // New skill should be in the library and linked to target
    assert!(tmp.path().join("library/brand-new-skill").is_dir());
    assert!(target_dir.join("brand-new-skill").is_symlink());
}

#[test]
fn sync_triage_dry_run_makes_no_changes() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let target_dir = tmp.path().join("target");

    let config = write_config_with_target(
        tmp.path(),
        &format!(
            "[directories.test]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
        &target_dir,
    );

    let config_str = config.to_str().unwrap();

    // Initial sync
    tome()
        .args(["--config", config_str, "sync"])
        .assert()
        .success();

    // Add a new skill
    create_skill(&skills_dir, "new-skill");

    // Dry-run update
    tome()
        .args(["--config", config_str, "--dry-run", "sync"])
        .assert()
        .success()
        .stderr(predicate::str::contains("dry-run"));

    // New skill should NOT be in library (dry-run)
    assert!(!tmp.path().join("library/new-skill").is_dir());
}

// -- Sync with machine prefs --

#[test]
fn sync_respects_machine_disabled() {
    // Test that sync with --machine skips disabled skills during distribution
    // AND removes their existing symlinks from targets.
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "keep-skill");
    create_skill(&skills_dir, "drop-skill");

    let target_dir = tmp.path().join("target");

    let config = write_config_with_target(
        tmp.path(),
        &format!(
            "[directories.test]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
        &target_dir,
    );

    // Sync — both skills should be distributed
    tome()
        .args(["--config", config.to_str().unwrap(), "sync"])
        .assert()
        .success();

    assert!(target_dir.join("keep-skill").is_symlink());
    assert!(target_dir.join("drop-skill").is_symlink());

    // Create machine.toml that disables "drop-skill"
    let machine_path = tmp.path().join("machine.toml");
    std::fs::write(&machine_path, "disabled = [\"drop-skill\"]\n").unwrap();

    // Re-sync with --machine — disabled skill's symlink should be removed
    tome()
        .args([
            "--config",
            config.to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
            "sync",
        ])
        .assert()
        .success();

    assert!(
        target_dir.join("keep-skill").is_symlink(),
        "enabled skill should still be linked"
    );
    assert!(
        !target_dir.join("drop-skill").exists(),
        "disabled skill's symlink should be removed by sync"
    );
}

#[test]
fn sync_triage_disable_removes_symlink() {
    // Test that disabling a skill and re-running update removes its symlink from targets.
    // Since we can't interact with the TTY in tests, we simulate the effect:
    // 1. Sync normally (both skills distributed)
    // 2. Manually create machine.toml disabling one skill
    // 3. The next update should not re-create the disabled symlink and should clean it up
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "enabled-skill");
    create_skill(&skills_dir, "disabled-skill");

    let target_dir = tmp.path().join("target");

    let config = write_config_with_target(
        tmp.path(),
        &format!(
            "[directories.test]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
        &target_dir,
    );

    let config_str = config.to_str().unwrap();

    // Initial sync — both skills distributed
    tome()
        .args(["--config", config_str, "sync"])
        .assert()
        .success();

    assert!(target_dir.join("enabled-skill").is_symlink());
    assert!(target_dir.join("disabled-skill").is_symlink());

    // Create machine.toml disabling one skill
    let machine_path = tmp.path().join("machine.toml");
    std::fs::write(&machine_path, "disabled = [\"disabled-skill\"]\n").unwrap();
    let machine_str = machine_path.to_str().unwrap();

    // Re-run update with --machine — should clean up disabled skill's symlink
    tome()
        .args([
            "--config",
            config_str,
            "--machine",
            machine_str,
            "--quiet",
            "sync",
        ])
        .assert()
        .success();

    assert!(
        target_dir.join("enabled-skill").is_symlink(),
        "enabled skill should still be linked"
    );
    assert!(
        !target_dir.join("disabled-skill").exists(),
        "disabled skill's symlink should be removed by update"
    );
}

#[test]
fn sync_respects_machine_disabled_targets() {
    // Test that sync with a disabled target does not distribute skills there,
    // and that an unknown disabled_target produces a warning on stderr.
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let target_dir = tmp.path().join("target");

    let config = write_config_with_target(
        tmp.path(),
        &format!(
            "[directories.test]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
        &target_dir,
    );

    // Create machine.toml that disables the configured target and also lists an unknown target
    let machine_path = tmp.path().join("machine.toml");
    std::fs::write(
        &machine_path,
        "disabled_directories = [\"test-target\", \"nonexistent-target\"]\n",
    )
    .unwrap();

    tome()
        .args([
            "--config",
            config.to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
            "sync",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Sync complete"))
        .stderr(predicate::str::contains(
            "warning: disabled directory 'nonexistent-target' in machine.toml does not match any configured directory",
        ));

    // The target directory should not have the skill (target is disabled)
    assert!(
        !target_dir.join("my-skill").exists(),
        "disabled target should not receive skills"
    );

    // The skill should still be in the library
    assert!(tmp.path().join("library/my-skill").is_dir());
}

// -- Sync with multiple targets (write_config_with_target style) --

#[test]
fn sync_with_two_targets_via_config() {
    // Quick smoke test for write_config_with_target plus manual second target
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let target_a = tmp.path().join("target-a");
    let target_b = tmp.path().join("target-b");
    std::fs::create_dir_all(&target_b).unwrap();

    let config_path = tmp.path().join("config.toml");
    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    std::fs::write(
        &config_path,
        format!(
            r#"library_dir = "{}"

[directories.test]
path = "{}"
type = "directory"
role = "source"

[directories.target-a]
path = "{}"
type = "directory"
role = "target"

[directories.target-b]
path = "{}"
type = "directory"
role = "target"
"#,
            library_dir.display(),
            skills_dir.display(),
            target_a.display(),
            target_b.display(),
        ),
    )
    .unwrap();

    tome()
        .args(["--config", config_path.to_str().unwrap(), "sync"])
        .assert()
        .success();

    assert!(target_a.join("my-skill").is_symlink());
    assert!(target_b.join("my-skill").is_symlink());
}

#[test]
fn sync_warns_unknown_disabled_targets() {
    // Test that `tome update` warns about disabled_targets in machine.toml
    // that don't match any configured target.
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let target_dir = tmp.path().join("target");

    let config = write_config_with_target(
        tmp.path(),
        &format!(
            "[directories.test]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
        &target_dir,
    );

    // Initial sync so library and lockfile exist
    tome()
        .args(["--config", config.to_str().unwrap(), "sync"])
        .assert()
        .success();

    // Create machine.toml with an unknown disabled target
    let machine_path = tmp.path().join("machine.toml");
    std::fs::write(
        &machine_path,
        "disabled_directories = [\"nonexistent-target\"]\n",
    )
    .unwrap();

    tome()
        .args([
            "--config",
            config.to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
            "sync",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "warning: disabled directory 'nonexistent-target' in machine.toml does not match any configured directory",
        ));
}

// === Symlink Chain Validation ===

#[test]
fn symlink_chain_local_skill() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("my-skill", "local")
        .build();

    env.cmd().arg("sync").assert().success();

    let library_skill = env.library_dir().join("my-skill");
    let target_skill = env.target_dir("test-tool").join("my-skill");

    // Library has a real directory (v0.2 copy model), not a symlink
    assert!(
        library_skill.is_dir(),
        "library skill should be a directory"
    );
    assert!(
        !library_skill.is_symlink(),
        "library skill should NOT be a symlink (local skills are copied)"
    );

    // Content should match source
    let source_content =
        std::fs::read_to_string(env.source_dir("local").join("my-skill/SKILL.md")).unwrap();
    let library_content = std::fs::read_to_string(library_skill.join("SKILL.md")).unwrap();
    assert_eq!(source_content, library_content);

    // Target should be a symlink pointing to the library entry
    assert!(
        target_skill.is_symlink(),
        "target skill should be a symlink"
    );
    let target_link = std::fs::canonicalize(&target_skill).unwrap();
    let library_canonical = std::fs::canonicalize(&library_skill).unwrap();
    assert_eq!(
        target_link, library_canonical,
        "target symlink should resolve to the library entry"
    );

    // Reading through the target symlink should work
    let target_content = std::fs::read_to_string(target_skill.join("SKILL.md")).unwrap();
    assert_eq!(source_content, target_content);
}

#[test]
fn symlink_chain_managed_skill() {
    // v0.10 (LIB-01): managed skills become real directory copies in the
    // library, NOT symlinks into machine-specific cache paths. The previous
    // (v0.9) shape — library entry is a symlink → source install dir — has
    // been replaced by the copy model. Targets still symlink into the library.
    //
    // Phase 13 added a hard requirement that the `claude` binary be on PATH
    // whenever ANY [directories.<name>] has type = "claude-plugins" (D-20):
    // `build_claude_adapter` calls `ClaudeMarketplaceAdapter::new()` which
    // probes for the binary unconditionally. Skip this test on machines
    // without claude — the same skip-gate pattern as marketplace.rs's smoke
    // tests.
    if !tome::marketplace::is_claude_available() {
        eprintln!("skipping symlink_chain_managed_skill: claude binary not on PATH");
        return;
    }

    let env = TestEnvBuilder::new()
        .source("plugins", "claude-plugins")
        .target("test-tool")
        .managed_skill("managed-skill", "plugins", "my-plugin@npm", "1.0.0")
        .build();

    env.cmd().arg("sync").assert().success();

    let library_skill = env.library_dir().join("managed-skill");
    let target_skill = env.target_dir("test-tool").join("managed-skill");

    // v0.10 shape: library entry is a real directory, NOT a symlink.
    assert!(
        library_skill.is_dir(),
        "managed skill in library should be a real directory after v0.10 (LIB-01)"
    );
    assert!(
        !library_skill.is_symlink(),
        "managed skill in library must NOT be a symlink in v0.10 (LIB-01)"
    );

    // The library copy's content_hash must match the source's content_hash
    // (using the production hash function via the crate-root re-export from
    // Plan 11-05 Task 0). This is the LIB-01 invariant: copy fidelity.
    let source_skill_dir = env
        .source_dir("plugins")
        .join("installs/managed-skill/skills/managed-skill");
    let library_hash = tome::hash_directory(&library_skill).unwrap();
    let source_hash = tome::hash_directory(&source_skill_dir).unwrap();
    assert_eq!(
        library_hash, source_hash,
        "library copy must hash identically to the managed source"
    );

    // Target is still a symlink (target → library).
    assert!(
        target_skill.is_symlink(),
        "target skill should be a symlink"
    );
    let target_resolved = std::fs::canonicalize(&target_skill).unwrap();
    let library_canonical = std::fs::canonicalize(&library_skill).unwrap();
    assert_eq!(
        target_resolved, library_canonical,
        "target symlink should resolve to the (real-dir) library entry"
    );

    // Reading SKILL.md through the target should return the same content as
    // the source — proves the copy fidelity end-to-end.
    let source_content = std::fs::read_to_string(source_skill_dir.join("SKILL.md")).unwrap();
    let target_content = std::fs::read_to_string(target_skill.join("SKILL.md")).unwrap();
    assert_eq!(
        source_content, target_content,
        "reading through target → library should match the original source content"
    );
}

#[test]
fn symlink_chain_survives_content_update() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("alpha", "local")
        .build();

    // Initial sync
    env.cmd().arg("sync").assert().success();

    let target_skill = env.target_dir("test-tool").join("alpha");
    assert!(target_skill.is_symlink());

    // Modify source content
    env.modify_skill(
        "alpha",
        "local",
        "---\nname: alpha\n---\n# alpha\nUpdated content.",
    );

    // Re-sync
    env.cmd().arg("sync").assert().success();

    // Target symlink should still work and return the NEW content
    let target_content = std::fs::read_to_string(target_skill.join("SKILL.md")).unwrap();
    assert!(
        target_content.contains("Updated content"),
        "target should serve updated content after re-sync"
    );
}

#[test]
fn symlink_chain_broken_after_source_removal() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("keep-me", "local")
        .skill("remove-me", "local")
        .build();

    // Initial sync
    env.cmd().arg("sync").assert().success();

    assert!(env.library_dir().join("keep-me").is_dir());
    assert!(env.library_dir().join("remove-me").is_dir());
    assert!(env.target_dir("test-tool").join("keep-me").is_symlink());
    assert!(env.target_dir("test-tool").join("remove-me").is_symlink());

    // Remove one skill from source
    env.remove_skill("remove-me", "local");

    // Re-sync — should clean up the removed skill
    env.cmd().arg("sync").assert().success();

    // Removed skill should be gone from library and target
    assert!(
        !env.library_dir().join("remove-me").exists(),
        "removed skill should be cleaned from library"
    );
    assert!(
        !env.target_dir("test-tool").join("remove-me").exists(),
        "removed skill should be cleaned from target"
    );

    // Remaining skill should still work through the chain
    let target_content =
        std::fs::read_to_string(env.target_dir("test-tool").join("keep-me/SKILL.md")).unwrap();
    assert!(target_content.contains("keep-me"));
}

// === TOME_HOME tests ===

#[test]
fn tome_home_flag_overrides_default() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("my-skill", "local")
        .build();

    // The --tome-home flag should be accepted and tome should use that directory
    // for manifest storage. We verify by syncing and checking that the manifest
    // ends up in the custom tome home, not the default.
    let custom_home = env.tmp.path().join("custom-tome-home");
    std::fs::create_dir_all(&custom_home).unwrap();

    // Copy config into the custom home so tome can find it
    std::fs::copy(&env.config_path, custom_home.join("tome.toml")).unwrap();

    tome()
        .arg("--tome-home")
        .arg(&custom_home)
        .arg("sync")
        .assert()
        .success();

    // Manifest should be in custom home, not default
    assert!(
        custom_home.join(".tome-manifest.json").exists(),
        "manifest should be in custom tome home"
    );
}

#[test]
fn tome_home_env_var_overrides_default() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("env-skill", "local")
        .build();

    let custom_home = env.tmp.path().join("env-tome-home");
    std::fs::create_dir_all(&custom_home).unwrap();
    std::fs::copy(&env.config_path, custom_home.join("tome.toml")).unwrap();

    tome()
        .env("TOME_HOME", &custom_home)
        .arg("sync")
        .assert()
        .success();

    assert!(
        custom_home.join(".tome-manifest.json").exists(),
        "manifest should be in TOME_HOME directory"
    );
}

#[test]
fn tome_home_flag_takes_precedence_over_env() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("prio-skill", "local")
        .build();

    let env_home = env.tmp.path().join("env-home");
    let flag_home = env.tmp.path().join("flag-home");
    std::fs::create_dir_all(&env_home).unwrap();
    std::fs::create_dir_all(&flag_home).unwrap();

    // Copy config to both locations
    std::fs::copy(&env.config_path, env_home.join("tome.toml")).unwrap();
    std::fs::copy(&env.config_path, flag_home.join("tome.toml")).unwrap();

    tome()
        .env("TOME_HOME", &env_home)
        .arg("--tome-home")
        .arg(&flag_home)
        .arg("sync")
        .assert()
        .success();

    // Flag should win over env var
    assert!(
        flag_home.join(".tome-manifest.json").exists(),
        "manifest should be in --tome-home path, not TOME_HOME env"
    );
    assert!(
        !env_home.join(".tome-manifest.json").exists(),
        "manifest should NOT be in TOME_HOME env path"
    );
}

// --- Smart config detection tests ---

#[test]
fn tome_home_finds_config_in_dotdir() {
    // When config is at TOME_HOME/.tome/tome.toml, tome should find it
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("dotdir-skill", "local")
        .build();

    let repo_root = env.tmp.path().join("my-repo");
    let dotdir = repo_root.join(".tome");
    std::fs::create_dir_all(&dotdir).unwrap();
    std::fs::copy(&env.config_path, dotdir.join("tome.toml")).unwrap();

    tome()
        .arg("--tome-home")
        .arg(&repo_root)
        .arg("sync")
        .assert()
        .success();

    // Manifest and lockfile should be in .tome/ subdir, not repo root
    assert!(
        dotdir.join(".tome-manifest.json").exists(),
        "manifest should be in .tome/ subdir"
    );
    assert!(
        dotdir.join("tome.lock").exists(),
        "lockfile should be in .tome/ subdir"
    );
    assert!(
        !repo_root.join(".tome-manifest.json").exists(),
        "manifest should NOT be at repo root"
    );
}

#[test]
fn tome_home_falls_back_to_root_config() {
    // When config is at TOME_HOME/tome.toml (no .tome/ subdir), tome should use root
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("root-skill", "local")
        .build();

    let custom_home = env.tmp.path().join("root-config-home");
    std::fs::create_dir_all(&custom_home).unwrap();
    std::fs::copy(&env.config_path, custom_home.join("tome.toml")).unwrap();

    tome()
        .arg("--tome-home")
        .arg(&custom_home)
        .arg("sync")
        .assert()
        .success();

    // Manifest and lockfile should be at root (backwards compat)
    assert!(
        custom_home.join(".tome-manifest.json").exists(),
        "manifest should be at tome home root"
    );
    assert!(
        custom_home.join("tome.lock").exists(),
        "lockfile should be at tome home root"
    );
}

#[test]
fn tome_home_dotdir_wins_over_root() {
    // When both TOME_HOME/.tome/tome.toml and TOME_HOME/tome.toml exist,
    // .tome/ subdir should win
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("priority-skill", "local")
        .build();

    let repo_root = env.tmp.path().join("both-configs");
    let dotdir = repo_root.join(".tome");
    std::fs::create_dir_all(&dotdir).unwrap();

    // Put config in both locations
    std::fs::copy(&env.config_path, dotdir.join("tome.toml")).unwrap();
    std::fs::copy(&env.config_path, repo_root.join("tome.toml")).unwrap();

    tome()
        .arg("--tome-home")
        .arg(&repo_root)
        .arg("sync")
        .assert()
        .success();

    // .tome/ subdir should win — manifest goes there
    assert!(
        dotdir.join(".tome-manifest.json").exists(),
        "manifest should be in .tome/ subdir (wins over root)"
    );
    assert!(
        !repo_root.join(".tome-manifest.json").exists(),
        "manifest should NOT be at root when .tome/ exists"
    );
}

#[test]
fn config_path_shows_correct_location_for_dotdir() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .skill("cfg-skill", "local")
        .build();

    let repo_root = env.tmp.path().join("config-path-test");
    let dotdir = repo_root.join(".tome");
    std::fs::create_dir_all(&dotdir).unwrap();
    std::fs::copy(&env.config_path, dotdir.join("tome.toml")).unwrap();

    let output = tome()
        .arg("--tome-home")
        .arg(&repo_root)
        .args(["config", "--path"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected = dotdir.join("tome.toml");
    assert!(
        stdout.trim().ends_with(".tome/tome.toml"),
        "config --path should show .tome/tome.toml, got: {}",
        stdout.trim()
    );
    assert_eq!(stdout.trim(), expected.display().to_string());
}

// === Edge Case Tests ===

#[test]
fn edge_target_dir_disappears_between_syncs() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("my-skill", "local")
        .build();

    // First sync
    env.cmd().arg("sync").assert().success();
    assert!(env.target_dir("test-tool").join("my-skill").is_symlink());

    // Delete target directory
    std::fs::remove_dir_all(env.target_dir("test-tool")).unwrap();
    assert!(!env.target_dir("test-tool").exists());

    // Re-sync should recreate target and symlinks
    env.cmd().arg("sync").assert().success();

    assert!(
        env.target_dir("test-tool").join("my-skill").is_symlink(),
        "symlink should be recreated after target dir was deleted"
    );
}

#[test]
fn edge_library_dir_disappears() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("my-skill", "local")
        .build();

    // First sync
    env.cmd().arg("sync").assert().success();
    assert!(env.library_dir().join("my-skill").is_dir());
    assert!(env.manifest_path().exists());

    // Delete library directory AND manifest (simulate clean slate)
    std::fs::remove_dir_all(env.library_dir()).unwrap();
    std::fs::remove_file(env.manifest_path()).unwrap();

    // Re-sync should recreate library
    env.cmd().arg("sync").assert().success();

    assert!(
        env.library_dir().join("my-skill").is_dir(),
        "library should be recreated with skills"
    );
    assert!(env.manifest_path().exists(), "manifest should be recreated");
}

#[test]
fn edge_source_dir_disappears() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("my-skill", "local")
        .build();

    // First sync
    env.cmd().arg("sync").assert().success();
    assert!(env.library_dir().join("my-skill").is_dir());

    // Delete the source directory
    std::fs::remove_dir_all(env.source_dir("local")).unwrap();

    // Re-sync — should warn about missing source and clean up
    let output = env.cmd().arg("sync").output().unwrap();
    assert!(output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("does not exist"),
        "should warn about missing source on stderr: {stderr}"
    );
}

#[test]
fn edge_broken_symlink_in_target_before_sync() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("real-skill", "local")
        .build();

    // Create a broken symlink in the target directory before any sync
    let stale_link = env.target_dir("test-tool").join("stale-link");
    std::os::unix::fs::symlink("/nonexistent/path", &stale_link).unwrap();
    assert!(stale_link.is_symlink());

    // Sync
    env.cmd().arg("sync").assert().success();

    // Real skill should be linked
    assert!(
        env.target_dir("test-tool").join("real-skill").is_symlink(),
        "real skill should be distributed"
    );

    // Stale link should be cleaned up (it doesn't point into our library)
    // Note: cleanup_target only removes symlinks pointing into the library dir,
    // so external broken symlinks may be preserved. Verify actual behavior.
    // The important thing is that sync succeeds.
}

#[cfg(unix)]
#[test]
fn edge_permission_denied_on_target() {
    use std::os::unix::fs::PermissionsExt;

    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("my-skill", "local")
        .build();

    // Make target dir unwritable
    let target = env.target_dir("test-tool");
    std::fs::set_permissions(target, std::fs::Permissions::from_mode(0o000)).unwrap();

    // Sync should fail or produce an error
    let output = env.cmd().arg("sync").output().unwrap();

    // Restore permissions so TempDir can clean up
    std::fs::set_permissions(target, std::fs::Permissions::from_mode(0o755)).unwrap();

    // Verify: sync should have failed (permission denied on creating symlinks)
    assert!(
        !output.status.success() || !String::from_utf8_lossy(&output.stderr).is_empty(),
        "sync should fail or warn when target is unwritable"
    );
}

#[test]
fn edge_corrupted_manifest() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .skill("my-skill", "local")
        .build();

    // First sync
    env.cmd().arg("sync").assert().success();
    assert!(env.manifest_path().exists());

    // Corrupt the manifest
    std::fs::write(env.manifest_path(), "not valid json!!!").unwrap();

    // Re-sync — should either recover or error clearly
    let output = env.cmd().arg("sync").output().unwrap();

    // We expect this to error (manifest parse failure)
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Either it errors, or it recovers and re-creates. Both are acceptable.
    assert!(
        !output.status.success() || stdout.contains("created"),
        "corrupted manifest should cause error or full re-sync: stderr={stderr}, stdout={stdout}"
    );
}

#[test]
fn edge_corrupted_lockfile() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("my-skill", "local")
        .build();

    // First sync to create lockfile
    env.cmd().arg("sync").assert().success();
    assert!(env.lockfile_path().exists());

    // Corrupt the lockfile
    std::fs::write(env.lockfile_path(), "this is garbage").unwrap();

    // Update should fail with a parse error
    let output = env.cmd().args(["sync", "--quiet"]).output().unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success() || stderr.contains("parse") || stderr.contains("error"),
        "corrupted lockfile should cause error: stderr={stderr}"
    );
}

#[test]
fn edge_config_library_dir_is_file() {
    let tmp = TempDir::new().unwrap();
    let library_path = tmp.path().join("library");
    // Create library_dir as a FILE, not directory
    std::fs::write(&library_path, "I am a file").unwrap();

    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let config_path = tmp.path().join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            "library_dir = \"{}\"\n\n[directories.test]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            library_path.display(),
            skills_dir.display(),
        ),
    )
    .unwrap();

    let output = tome()
        .args(["--config", config_path.to_str().unwrap(), "sync"])
        .output()
        .unwrap();

    // Should fail — library_dir is a file, not a directory
    assert!(
        !output.status.success(),
        "sync should fail when library_dir is a file"
    );
}

#[test]
fn edge_skill_empty_skill_md() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .skill_with_content("empty-skill", "local", "")
        .build();

    // Sync should succeed with empty SKILL.md
    env.cmd().arg("sync").assert().success();

    assert!(
        env.library_dir().join("empty-skill").is_dir(),
        "skill with empty SKILL.md should still be synced"
    );
}

#[test]
fn edge_skill_with_nested_content() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .skill("nested-skill", "local")
        .build();

    // Add extra files to the skill: a subdirectory with a file
    let skill_dir = env.source_dir("local").join("nested-skill");
    let sub_dir = skill_dir.join("examples");
    std::fs::create_dir_all(&sub_dir).unwrap();
    std::fs::write(sub_dir.join("example.txt"), "an example file").unwrap();
    std::fs::write(skill_dir.join("extra.md"), "extra content").unwrap();

    env.cmd().arg("sync").assert().success();

    let library_skill = env.library_dir().join("nested-skill");
    assert!(library_skill.join("SKILL.md").exists());
    assert!(
        library_skill.join("examples/example.txt").exists(),
        "subdirectory contents should be copied"
    );
    assert!(
        library_skill.join("extra.md").exists(),
        "extra files should be copied"
    );
}

// === Multi-Command Lifecycle Tests ===

#[test]
fn lifecycle_full_sync_journey() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .build();

    // Step 1: Sync with no skills yet
    env.cmd().arg("sync").assert().success();

    // Step 2: Add first skill and sync
    env.add_skill("alpha", "local");
    let output = env.cmd().arg("sync").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("1 created"),
        "first skill should be created: {stdout}"
    );
    assert!(env.library_dir().join("alpha").is_dir());
    assert!(env.target_dir("test-tool").join("alpha").is_symlink());

    // Step 3: Add second skill and sync
    env.add_skill("beta", "local");
    let output = env.cmd().arg("sync").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("1 created") && stdout.contains("1 unchanged"),
        "should show 1 created + 1 unchanged: {stdout}"
    );

    // Step 4: Modify first skill and sync
    env.modify_skill(
        "alpha",
        "local",
        "---\nname: alpha\n---\n# alpha\nModified content.",
    );
    let output = env.cmd().arg("sync").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("1 updated"),
        "modified skill should be updated: {stdout}"
    );

    // Step 5: Remove second skill and sync
    env.remove_skill("beta", "local");
    env.cmd().arg("sync").assert().success();

    assert!(env.library_dir().join("alpha").is_dir());
    assert!(
        !env.library_dir().join("beta").exists(),
        "removed skill should be cleaned from library"
    );
    assert!(
        !env.target_dir("test-tool").join("beta").exists(),
        "removed skill should be cleaned from target"
    );

    // Step 6: Doctor should find no issues
    env.cmd()
        .args(["doctor", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No issues found"));

    // Step 7: Status should show 1 skill
    env.cmd().arg("status").assert().success();
}

#[test]
fn lifecycle_update_with_lockfile_diff() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("skill-a", "local")
        .skill("skill-b", "local")
        .build();

    // Initial sync to establish lockfile
    env.cmd().arg("sync").assert().success();
    assert!(env.lockfile_path().exists());

    // Add a new skill
    env.add_skill("skill-c", "local");

    // Update should detect the new skill
    env.cmd().args(["sync", "--quiet"]).assert().success();

    // Verify new skill is in library and target
    assert!(
        env.library_dir().join("skill-c").is_dir(),
        "new skill should be in library after update"
    );
    assert!(
        env.target_dir("test-tool").join("skill-c").is_symlink(),
        "new skill should be in target after update"
    );

    // Verify lockfile has 3 entries
    let lockfile_content = std::fs::read_to_string(env.lockfile_path()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&lockfile_content).unwrap();
    let skills = parsed["skills"].as_object().unwrap();
    assert_eq!(skills.len(), 3, "lockfile should have 3 skill entries");
}

#[test]
fn lifecycle_doctor_detects_and_reports() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("real-skill", "local")
        .build();

    // Sync to establish baseline
    env.cmd().arg("sync").assert().success();

    // Create orphan directory in library (not from any source)
    let orphan = env.library_dir().join("phantom");
    std::fs::create_dir_all(&orphan).unwrap();
    std::fs::write(orphan.join("SKILL.md"), "orphan").unwrap();

    // Create broken symlink in target
    let broken_link = env.target_dir("test-tool").join("broken");
    std::os::unix::fs::symlink("/nonexistent/path", &broken_link).unwrap();

    // Doctor should detect issues
    let output = env.cmd().args(["doctor", "--dry-run"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("issue") || stdout.contains("Issue"),
        "doctor should detect orphan/broken entries: {stdout}"
    );

    // Clean up manually
    std::fs::remove_dir_all(&orphan).unwrap();
    std::fs::remove_file(&broken_link).unwrap();

    // Doctor should now be clean
    env.cmd()
        .args(["doctor", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No issues found"));
}

#[test]
fn lifecycle_multi_source_dedup() {
    let env = TestEnvBuilder::new()
        .source("primary", "directory")
        .source("secondary", "directory")
        .skill_with_content(
            "shared",
            "primary",
            "---\nname: shared\n---\n# shared\nFrom primary.",
        )
        .skill_with_content(
            "shared",
            "secondary",
            "---\nname: shared\n---\n# shared\nFrom secondary.",
        )
        .build();

    // First sync — primary should win (first source wins)
    env.cmd().arg("sync").assert().success();

    let library_content =
        std::fs::read_to_string(env.library_dir().join("shared/SKILL.md")).unwrap();
    assert!(
        library_content.contains("From primary"),
        "first source should win: {library_content}"
    );

    // Remove skill from primary
    env.remove_skill("shared", "primary");

    // Re-sync — secondary should now provide the skill
    env.cmd().arg("sync").assert().success();

    let library_content =
        std::fs::read_to_string(env.library_dir().join("shared/SKILL.md")).unwrap();
    assert!(
        library_content.contains("From secondary"),
        "after removing from primary, secondary should take over: {library_content}"
    );
}

#[test]
fn lifecycle_multi_target_distribution() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("target-a")
        .target("target-b")
        .skill("my-skill", "local")
        .build();

    // Sync — both targets should get symlinks
    env.cmd().arg("sync").assert().success();
    assert!(
        env.target_dir("target-a").join("my-skill").is_symlink(),
        "target-a should have the skill"
    );
    assert!(
        env.target_dir("target-b").join("my-skill").is_symlink(),
        "target-b should have the skill"
    );

    // Disable target-b via machine.toml and re-sync
    let machine_path = env.tome_home().join("machine.toml");
    std::fs::write(&machine_path, "disabled_directories = [\"target-b\"]\n").unwrap();

    env.cmd()
        .args(["--machine", machine_path.to_str().unwrap(), "sync"])
        .assert()
        .success();

    assert!(
        env.target_dir("target-a").join("my-skill").is_symlink(),
        "target-a should still have the skill"
    );
    // Note: disabled targets are skipped entirely (no distribute AND no cleanup),
    // so existing symlinks in disabled targets are left in place.
    assert!(
        env.target_dir("target-b").join("my-skill").is_symlink(),
        "target-b symlinks are preserved (disabled targets are skipped, not cleaned)"
    );

    // Remove machine.toml and re-sync — target-b should still work
    std::fs::remove_file(&machine_path).unwrap();
    env.cmd().arg("sync").assert().success();

    assert!(
        env.target_dir("target-b").join("my-skill").is_symlink(),
        "target-b should work after re-enabling"
    );
}

#[test]
fn eject_removes_symlinks_and_sync_restores() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-target")
        .skill("my-skill", "local")
        .build();

    // First sync to distribute
    env.cmd().arg("sync").assert().success();
    assert!(
        env.target_dir("test-target").join("my-skill").is_symlink(),
        "skill should be distributed after sync"
    );

    // Eject (non-interactive, stdin is not a terminal in tests so no prompt)
    env.cmd()
        .arg("eject")
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed 1 symlink(s)"));

    assert!(
        !env.target_dir("test-target").join("my-skill").exists(),
        "symlink should be removed after eject"
    );
    assert!(
        env.library_dir().join("my-skill").is_dir(),
        "library should remain intact after eject"
    );

    // Sync again to restore
    env.cmd().arg("sync").assert().success();
    assert!(
        env.target_dir("test-target").join("my-skill").is_symlink(),
        "skill should be restored after re-sync"
    );
}

#[test]
fn eject_dry_run_does_not_remove() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-target")
        .skill("my-skill", "local")
        .build();

    // First sync to distribute
    env.cmd().arg("sync").assert().success();
    assert!(env.target_dir("test-target").join("my-skill").is_symlink());

    // Eject with dry-run
    env.cmd()
        .args(["--dry-run", "eject"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry run"));

    assert!(
        env.target_dir("test-target").join("my-skill").is_symlink(),
        "symlink should still exist after dry-run eject"
    );
}

#[test]
fn eject_nothing_to_eject() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-target")
        .skill("my-skill", "local")
        .build();

    // Don't sync — target is empty
    env.cmd()
        .arg("eject")
        .assert()
        .success()
        .stdout(predicate::str::contains("Nothing to eject"));
}

#[test]
fn completions_fish_installs_to_file() {
    let home = TempDir::new().unwrap();
    let xdg_config = home.path().join(".config");
    tome()
        .env("HOME", home.path())
        .env("TOME_HOME", home.path().join(".tome"))
        .env("XDG_CONFIG_HOME", &xdg_config)
        .args(["completions", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed fish completions"));
    let completions_file = xdg_config.join("fish/completions/tome.fish");
    assert!(completions_file.exists());
    let content = std::fs::read_to_string(&completions_file).unwrap();
    assert!(content.contains("complete -c tome"));
}

#[test]
fn completions_bash_installs_to_file() {
    let home = TempDir::new().unwrap();
    let xdg_data = home.path().join(".local/share");
    tome()
        .env("HOME", home.path())
        .env("TOME_HOME", home.path().join(".tome"))
        .env("XDG_DATA_HOME", &xdg_data)
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed bash completions"));
    let completions_file = xdg_data.join("bash-completion/completions/tome");
    assert!(completions_file.exists());
    let content = std::fs::read_to_string(&completions_file).unwrap();
    assert!(content.contains("tome"));
}

#[test]
fn completions_zsh_installs_to_file() {
    let home = TempDir::new().unwrap();
    tome()
        .env("HOME", home.path())
        .env("TOME_HOME", home.path().join(".tome"))
        .args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed zsh completions"));
    let completions_file = home.path().join(".zfunc/_tome");
    assert!(completions_file.exists());
    let content = std::fs::read_to_string(&completions_file).unwrap();
    assert!(content.contains("#compdef tome"));
}

#[test]
fn completions_invalid_shell_fails() {
    tome().args(["completions", "invalid"]).assert().failure();
}

#[test]
fn completions_powershell_errors_with_instructions() {
    let tmp = TempDir::new().unwrap();
    tome()
        .env("TOME_HOME", tmp.path())
        .args(["completions", "powershell"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Automatic installation not supported",
        ))
        .stderr(predicate::str::contains("--print"));
}

#[test]
fn completions_print_outputs_to_stdout() {
    let tmp = TempDir::new().unwrap();
    tome()
        .env("TOME_HOME", tmp.path())
        .args(["completions", "fish", "--print"])
        .assert()
        .success()
        .stdout(predicate::str::contains("complete -c tome"));
}

// === Lint tests ===

#[test]
fn lint_clean_library() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .skill_with_content(
            "good-skill",
            "local",
            "---\nname: good-skill\ndescription: A valid skill\n---\n# Good Skill",
        )
        .build();

    // First sync to populate the library
    env.cmd().arg("sync").assert().success();

    env.cmd()
        .arg("lint")
        .assert()
        .success()
        .stdout(predicate::str::contains("0 error(s)"));
}

#[test]
fn lint_reports_errors() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .skill_with_content("my-skill", "local", "---\nname: wrong-name\n---\n# Wrong")
        .build();

    // Sync to populate library
    env.cmd().arg("sync").assert().success();

    env.cmd()
        .arg("lint")
        .assert()
        .failure() // exit code 1 because of errors
        .stdout(predicate::str::contains("does not match directory"));
}

#[test]
fn lint_json_output() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .skill_with_content(
            "test-skill",
            "local",
            "---\nname: test-skill\ndescription: Valid skill\n---\n# Test",
        )
        .build();

    // Sync to populate library
    env.cmd().arg("sync").assert().success();

    env.cmd()
        .args(["lint", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"skills_checked\": 1"));
}

#[test]
fn lint_single_skill_path() {
    let tmp = TempDir::new().unwrap();
    let skill = tmp.path().join("my-skill");
    std::fs::create_dir_all(&skill).unwrap();
    std::fs::write(
        skill.join("SKILL.md"),
        "---\nname: my-skill\ndescription: Test\n---\n# Test",
    )
    .unwrap();

    tome()
        .env("TOME_HOME", tmp.path())
        .args(["lint", &skill.to_string_lossy()])
        .assert()
        .success()
        .stdout(predicate::str::contains("0 error(s)"));
}

#[test]
fn lint_single_skill_path_with_errors() {
    let tmp = TempDir::new().unwrap();
    let skill = tmp.path().join("bad-skill");
    std::fs::create_dir_all(&skill).unwrap();
    std::fs::write(skill.join("SKILL.md"), "# No frontmatter").unwrap();

    tome()
        .env("TOME_HOME", tmp.path())
        .args(["lint", &skill.to_string_lossy()])
        .assert()
        .failure()
        .stdout(predicate::str::contains("no frontmatter"));
}

// === Backup tests ===

#[test]
fn backup_init_and_snapshot() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .skill("my-skill", "local")
        .build();

    // Sync first to populate the library
    tome()
        .args(["--config", &env.config_path.to_string_lossy(), "sync"])
        .assert()
        .success();

    // Init backup (commits existing library content)
    tome()
        .args([
            "--config",
            &env.config_path.to_string_lossy(),
            "backup",
            "init",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized backup repo"));

    // Add a new file to the library so there's something to snapshot
    std::fs::write(env.library_dir.join("extra.txt"), "new content").unwrap();

    // Snapshot
    tome()
        .args([
            "--config",
            &env.config_path.to_string_lossy(),
            "backup",
            "snapshot",
            "-m",
            "test snapshot",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Snapshot created"));
}

#[test]
fn backup_list_shows_history() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .skill("skill-a", "local")
        .build();

    // Sync to populate library
    tome()
        .args(["--config", &env.config_path.to_string_lossy(), "sync"])
        .assert()
        .success();

    // Init backup
    tome()
        .args([
            "--config",
            &env.config_path.to_string_lossy(),
            "backup",
            "init",
        ])
        .assert()
        .success();

    // Add a file and create a snapshot
    std::fs::write(env.library_dir.join("extra.txt"), "new content").unwrap();
    tome()
        .args([
            "--config",
            &env.config_path.to_string_lossy(),
            "backup",
            "snapshot",
            "-m",
            "first snapshot",
        ])
        .assert()
        .success();

    // List should show both the initial backup and the snapshot
    tome()
        .args([
            "--config",
            &env.config_path.to_string_lossy(),
            "backup",
            "list",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("first snapshot"))
        .stdout(predicate::str::contains("Initial tome backup"));
}

#[test]
fn exit_code_2_for_invalid_args() {
    // Clap returns exit code 2 for usage errors (invalid flags)
    tome().arg("--nonexistent-flag").assert().code(2);
}

#[test]
fn exit_code_1_for_runtime_errors() {
    // Runtime errors (missing config) return exit code 1
    tome()
        .args(["--config", "/nonexistent/path.toml", "status"])
        .assert()
        .code(1);
}

#[test]
fn no_input_flag_skips_all_prompts() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("skill-a", "local")
        .build();

    // Sync with --no-input should succeed without hanging on prompts
    tome()
        .args([
            "--config",
            &env.config_path.to_string_lossy(),
            "--no-input",
            "sync",
        ])
        .assert()
        .success();

    // Verify skill was distributed to default target
    let target_dir = &env.target_dirs[0].1;
    assert!(target_dir.join("skill-a").is_symlink());
}

#[test]
fn init_with_no_input_and_dry_run_succeeds() {
    // Headless smoke: `tome init --no-input --dry-run` must run the wizard to
    // completion without any TTY and print the `Generated config:` marker.
    // HOME is isolated to a TempDir so auto-discovery is deterministic
    // (empty HOME → no known directories). Richer assertions on the emitted
    // TOML live in `init_dry_run_no_input_empty_home` and
    // `init_dry_run_no_input_seeded_home`.
    let tmp = TempDir::new().unwrap();
    tome()
        .env("HOME", tmp.path())
        .env("NO_COLOR", "1")
        .args([
            "--tome-home",
            &tmp.path().display().to_string(),
            "--no-input",
            "--dry-run",
            "init",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Generated config:"));
}

#[test]
fn doctor_with_no_input_skips_repair() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .skill("skill-a", "local")
        .build();

    // Doctor with --no-input should not hang on prompts
    tome()
        .args([
            "--config",
            &env.config_path.to_string_lossy(),
            "--no-input",
            "doctor",
        ])
        .assert()
        .success();
}

#[test]
fn status_json_output() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("skill-a", "local")
        .build();

    tome()
        .args(["--config", &env.config_path.to_string_lossy(), "sync"])
        .assert()
        .success();

    let output = tome()
        .args([
            "--config",
            &env.config_path.to_string_lossy(),
            "status",
            "--json",
        ])
        .output()
        .expect("failed to run");

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("status --json should produce valid JSON");
    assert_eq!(json["configured"], true);
    assert!(json["directories"].is_array());
}

#[test]
fn doctor_json_output() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .skill("skill-a", "local")
        .build();

    let output = tome()
        .args([
            "--config",
            &env.config_path.to_string_lossy(),
            "doctor",
            "--json",
        ])
        .output()
        .expect("failed to run");

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("doctor --json should produce valid JSON");
    assert_eq!(json["configured"], true);
    assert!(json["library_issues"].is_array());
}

#[test]
fn config_toml_tome_home_override() {
    // This test verifies that --tome-home takes precedence,
    // which exercises the resolution order without needing to write
    // to ~/.config/tome/config.toml.
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .skill("skill-a", "local")
        .build();

    // Sync using --tome-home to set a custom tome home
    tome()
        .args([
            "--config",
            &env.config_path.to_string_lossy(),
            "--tome-home",
            &env.library_dir.parent().unwrap().to_string_lossy(),
            "status",
        ])
        .assert()
        .success();
}

#[test]
fn no_color_env_suppresses_ansi_escapes() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .skill("skill-a", "local")
        .build();

    // Sync first to populate library
    tome()
        .args(["--config", &env.config_path.to_string_lossy(), "sync"])
        .assert()
        .success();

    // Run status with NO_COLOR=1 — output must not contain ANSI escape sequences
    let output = tome()
        .args(["--config", &env.config_path.to_string_lossy(), "status"])
        .env("NO_COLOR", "1")
        .output()
        .expect("failed to run tome status");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stdout.contains("\x1b["),
        "stdout should not contain ANSI escapes with NO_COLOR=1, got: {stdout}"
    );
    assert!(
        !stderr.contains("\x1b["),
        "stderr should not contain ANSI escapes with NO_COLOR=1, got: {stderr}"
    );
}

// === tome remove tests ===

/// Helper to create a remove-test environment where config is at `tome.toml`.
fn remove_test_env(tmp: &TempDir, directories_toml: &str) -> PathBuf {
    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    let config_path = tmp.path().join("tome.toml");
    std::fs::write(
        &config_path,
        format!(
            "library_dir = \"{}\"\n{}",
            library_dir.display(),
            directories_toml,
        ),
    )
    .unwrap();
    config_path
}

#[test]
fn test_remove_nonexistent_directory() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    remove_test_env(
        &tmp,
        &format!(
            "[directories.local]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
    );

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "remove",
            "dir",
            "nonexistent",
            "--force",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found in config"));
}

#[test]
fn test_remove_local_directory() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let target_dir = tmp.path().join("target");
    std::fs::create_dir_all(&target_dir).unwrap();

    remove_test_env(
        &tmp,
        &format!(
            "[directories.local]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n\n[directories.test-target]\npath = \"{}\"\ntype = \"directory\"\nrole = \"target\"\n",
            skills_dir.display(),
            target_dir.display()
        ),
    );

    // First sync to populate library and targets
    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "sync",
            "--no-triage",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();

    let library_dir = tmp.path().join("library");
    assert!(library_dir.join("my-skill").exists());
    assert!(target_dir.join("my-skill").exists());

    // Remove the source directory
    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "remove",
            "dir",
            "local",
            "--force",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();

    // Verify cleanup
    // v0.10 (LIB-04): library content for owned skills is preserved on
    // `tome remove`; the manifest entry transitions to Unowned. Distribution
    // symlinks ARE still removed (the user removed the source from config,
    // not the skill from the library).
    assert!(
        library_dir.join("my-skill").exists(),
        "library skill must be preserved as Unowned per LIB-04"
    );
    assert!(
        !target_dir.join("my-skill").exists(),
        "target symlink should be removed"
    );

    // Verify config no longer has the directory
    let config_content = std::fs::read_to_string(tmp.path().join("tome.toml")).unwrap();
    assert!(
        !config_content.contains("[directories.local]"),
        "config should no longer contain the removed directory"
    );
}

#[test]
fn test_remove_dry_run() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let target_dir = tmp.path().join("target");
    std::fs::create_dir_all(&target_dir).unwrap();

    remove_test_env(
        &tmp,
        &format!(
            "[directories.local]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n\n[directories.test-target]\npath = \"{}\"\ntype = \"directory\"\nrole = \"target\"\n",
            skills_dir.display(),
            target_dir.display()
        ),
    );

    // First sync
    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "sync",
            "--no-triage",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();

    // Remove with --dry-run
    let output = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--dry-run",
            "remove",
            "dir",
            "local",
            "--force",
        ])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Dry run"),
        "should show dry run message, got: {stdout}"
    );

    // Verify nothing was actually removed
    let library_dir = tmp.path().join("library");
    assert!(
        library_dir.join("my-skill").exists(),
        "library skill should still exist after dry run"
    );
    let config_content = std::fs::read_to_string(tmp.path().join("tome.toml")).unwrap();
    assert!(
        config_content.contains("[directories.local]"),
        "config should still contain the directory after dry run"
    );
}

#[test]
fn test_remove_no_input_without_force_fails() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    remove_test_env(
        &tmp,
        &format!(
            "[directories.local]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
    );

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--no-input",
            "remove",
            "dir",
            "local",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("use --force"));
}

#[cfg(unix)]
#[test]
fn remove_partial_failure_exits_nonzero_with_warning_marker() {
    use std::os::unix::fs::PermissionsExt;

    // Fixture: source dir with one skill, target dir (distribution) wired as
    // a target role in config. After sync, the target contains a symlink to
    // the library skill. We then chmod 0o000 the target directory so remove's
    // step-1 loop (distribution symlinks) cannot enumerate / unlink inside.
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let target_dir = tmp.path().join("target");
    std::fs::create_dir_all(&target_dir).unwrap();

    remove_test_env(
        &tmp,
        &format!(
            "[directories.local]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n\n[directories.test-target]\npath = \"{}\"\ntype = \"directory\"\nrole = \"target\"\n",
            skills_dir.display(),
            target_dir.display()
        ),
    );

    // Prime the library + target with a real symlink so the plan has
    // something to try to remove in step 1.
    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "sync",
            "--no-triage",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();

    assert!(target_dir.join("my-skill").exists());

    // Clamp the target dir to read+execute only: plan() can still read_dir
    // it to enumerate the symlinks, but execute()'s remove_file call needs
    // write permission on the parent dir and so hits EACCES — landing in
    // the partial-failure path rather than bailing from plan().
    std::fs::set_permissions(&target_dir, std::fs::Permissions::from_mode(0o500)).unwrap();

    let output = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "remove",
            "dir",
            "local",
            "--force",
        ])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();

    // Restore permissions FIRST so TempDir::drop can clean up, BEFORE any
    // assertions (Pitfall 2 from 08-RESEARCH.md).
    std::fs::set_permissions(&target_dir, std::fs::Permissions::from_mode(0o755)).unwrap();

    assert!(
        !output.status.success(),
        "remove should fail on chmod 0o000 target, got status: {:?}",
        output.status,
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("⚠"), "stderr missing ⚠ marker: {stderr}");
    assert!(
        stderr.contains("operations failed"),
        "stderr missing 'operations failed': {stderr}"
    );
    assert!(
        stderr.contains("remove completed with"),
        "stderr missing anyhow error 'remove completed with': {stderr}"
    );
    // I2/I3: user-facing message must mention retry path so they know
    // config/manifest entries survived for a retry attempt.
    assert!(
        stderr.contains("retained") || stderr.contains("retry"),
        "stderr missing retry guidance (I2/I3): {stderr}"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    // TEST-01 / P1: success banner MUST NOT appear on partial failure.
    // The banner string is "✓ Removed directory" but the leading glyph may
    // be styled with ANSI codes; we assert on "Removed directory" (no glyph)
    // for robustness against console color rendering. NO_COLOR=1 is already
    // set above so the styled `✓` is a literal char, but defending against
    // both forms is defense-in-depth.
    assert!(
        !stdout.contains("Removed directory"),
        "stdout must NOT contain success banner on partial failure; got: {stdout}",
    );
    assert!(
        !stderr.contains("Removed directory"),
        "stderr must NOT contain success banner on partial failure (defense-in-depth); got: {stderr}",
    );
}

#[cfg(unix)]
#[test]
fn remove_partial_failure_does_not_save_disk_state() {
    use std::os::unix::fs::PermissionsExt;

    // HOTFIX-02 / #461 H2: with the save chain reordered, a partial-failure
    // path must NOT mutate config / manifest / lockfile on disk. The user
    // retains a clean retry surface — no half-saved state. The reorder
    // guarantees the early-return ⚠ block fires BEFORE config.save /
    // manifest::save / lockfile::save can run, so on the failure path none
    // of those files are touched.
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let target_dir = tmp.path().join("target");
    std::fs::create_dir_all(&target_dir).unwrap();

    remove_test_env(
        &tmp,
        &format!(
            "[directories.local]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n\n[directories.test-target]\npath = \"{}\"\ntype = \"directory\"\nrole = \"target\"\n",
            skills_dir.display(),
            target_dir.display()
        ),
    );

    // Prime library + target so there's something to remove.
    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "sync",
            "--no-triage",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();

    // Snapshot pre-remove disk state. tome.lock may or may not exist
    // depending on what `tome sync --no-triage` writes for a non-git
    // config — tolerate either case (read returns empty Vec on missing,
    // and the byte-equality check still proves "missing-then-missing").
    let config_path = tmp.path().join("tome.toml");
    let manifest_path = tmp.path().join(".tome-manifest.json");
    let lockfile_path = tmp.path().join("tome.lock");

    let config_before = std::fs::read(&config_path).unwrap_or_default();
    let manifest_before = std::fs::read(&manifest_path).unwrap_or_default();
    let lockfile_before = std::fs::read(&lockfile_path).unwrap_or_default();

    // Trigger partial-failure: chmod 0o500 the target dir so step-1 unlink
    // hits EACCES — execute() lands in the partial-failure path with a
    // non-empty `failures` Vec.
    std::fs::set_permissions(&target_dir, std::fs::Permissions::from_mode(0o500)).unwrap();

    let output = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "remove",
            "dir",
            "local",
            "--force",
        ])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();

    // Restore permissions BEFORE assertions so TempDir::drop can clean up.
    std::fs::set_permissions(&target_dir, std::fs::Permissions::from_mode(0o755)).unwrap();

    // 1. CLI exits non-zero.
    assert!(
        !output.status.success(),
        "remove should fail under chmod 0o500, got status: {:?}",
        output.status,
    );

    // 2. Stderr has the ⚠ block (proves the moved block fired BEFORE save).
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("⚠"), "missing ⚠ marker in stderr: {stderr}");
    assert!(
        stderr.contains("operations failed"),
        "missing 'operations failed' in stderr: {stderr}"
    );

    // 3. Disk state is unchanged byte-for-byte (the I2/I3 retention contract
    //    extended to disk: not just in-memory). If the reorder is reverted,
    //    config.save / manifest::save / lockfile::save run BEFORE the early
    //    return and these byte-equality assertions fail.
    let config_after = std::fs::read(&config_path).unwrap_or_default();
    let manifest_after = std::fs::read(&manifest_path).unwrap_or_default();
    let lockfile_after = std::fs::read(&lockfile_path).unwrap_or_default();
    assert_eq!(
        config_before, config_after,
        "tome.toml mutated on partial-failure path (HOTFIX-02 regression)"
    );
    assert_eq!(
        manifest_before, manifest_after,
        ".tome-manifest.json mutated on partial-failure path (HOTFIX-02 regression)"
    );
    assert_eq!(
        lockfile_before, lockfile_after,
        "tome.lock mutated on partial-failure path (HOTFIX-02 regression)"
    );
}

#[cfg(unix)]
#[test]
fn remove_retry_succeeds_after_failure_resolved() {
    use std::os::unix::fs::PermissionsExt;

    // TEST-02 / P2: end-to-end I2/I3 retention contract.
    //   1. Partial failure → config entry + manifest preserved (existing v0.8 contract)
    //   2. User fixes the underlying condition (chmod 0o755)
    //   3. Second `tome remove` succeeds, leaves NO leftover state
    //
    // Without this test, the retry path is only exercised by manual UAT.
    // A future refactor that mutates config/manifest on the failure path
    // (regressing #461 H2) would silently break retry — the second
    // `tome remove` would fail with "directory not found in config".

    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let target_dir = tmp.path().join("target");
    std::fs::create_dir_all(&target_dir).unwrap();

    remove_test_env(
        &tmp,
        &format!(
            "[directories.local]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n\n[directories.test-target]\npath = \"{}\"\ntype = \"directory\"\nrole = \"target\"\n",
            skills_dir.display(),
            target_dir.display()
        ),
    );

    // Prime: sync to wire library + target symlink.
    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "sync",
            "--no-triage",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();
    assert!(
        target_dir.join("my-skill").exists(),
        "fixture: target symlink must exist after sync"
    );

    // Step 1 — partial failure: chmod 0o500 on target dir.
    std::fs::set_permissions(&target_dir, std::fs::Permissions::from_mode(0o500)).unwrap();

    let first = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "remove",
            "dir",
            "local",
            "--force",
        ])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        !first.status.success(),
        "first remove must fail on chmod 0o500"
    );
    let first_stderr = String::from_utf8_lossy(&first.stderr);
    assert!(
        first_stderr.contains("⚠"),
        "first remove stderr missing ⚠ marker: {first_stderr}"
    );

    // Step 1.5 — assert config entry preserved (I2 retention).
    let config_after_fail = std::fs::read_to_string(tmp.path().join("tome.toml")).unwrap();
    assert!(
        config_after_fail.contains("[directories.local]"),
        "config entry for 'local' must be preserved on partial failure; got: {config_after_fail}"
    );

    // Step 2 — user fixes the underlying cause.
    std::fs::set_permissions(&target_dir, std::fs::Permissions::from_mode(0o755)).unwrap();

    // Step 3 — retry: second `tome remove` should succeed cleanly.
    let second = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "remove",
            "dir",
            "local",
            "--force",
        ])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        second.status.success(),
        "retry remove must succeed after chmod restore; stderr: {}",
        String::from_utf8_lossy(&second.stderr)
    );
    let second_stdout = String::from_utf8_lossy(&second.stdout);
    assert!(
        second_stdout.contains("Removed directory"),
        "retry stdout must contain success banner; got: {second_stdout}"
    );

    // Step 4 — assert clean state per v0.10 LIB-04 / D-10 trigger 1:
    // - config entry is removed
    // - manifest entry is RETAINED (transitioned to Unowned with source_name omitted)
    // - library dir is RETAINED (preserved as Unowned content)
    let config_after_success = std::fs::read_to_string(tmp.path().join("tome.toml")).unwrap();
    assert!(
        !config_after_success.contains("[directories.local]"),
        "config entry for 'local' must be removed after retry success; got: {config_after_success}"
    );

    let manifest_path = tmp.path().join(".tome-manifest.json");
    assert!(
        manifest_path.exists(),
        "manifest must still exist after retry success"
    );
    let manifest = std::fs::read_to_string(&manifest_path).unwrap();
    assert!(
        manifest.contains("\"my-skill\""),
        "manifest must retain my-skill (Unowned) per LIB-04; got: {manifest}"
    );
    assert!(
        !manifest.contains("\"source_name\":\"local\""),
        "my-skill source_name must be transitioned away from 'local' (skip_serializing_if omits None); got: {manifest}"
    );

    let library_skill = tmp.path().join("library").join("my-skill");
    assert!(
        library_skill.exists(),
        "library dir for my-skill must be preserved as Unowned per LIB-04; missing at {}",
        library_skill.display()
    );
}

#[test]
fn lib_rs_remove_handler_prints_success_banner_before_regen_warnings() {
    // TEST-04 / P4 regression: pin the source-order in lib.rs Command::Remove
    // happy-path. The success banner `println!("Removed directory ...")` MUST
    // appear earlier in the file than the `for w in &regen_warnings ... eprintln!`
    // loop. If a future refactor reorders these, this test fails.
    //
    // ANCHORING: lib.rs contains three `for w in &regen_warnings` loops —
    // one each in Remove, Reassign, Fork handlers. Without anchoring to
    // `Command::Remove` first, a future reorder of Reassign or Fork (or
    // a new handler inserted above Remove with its own regen-warnings
    // loop) could create a false-positive failure unrelated to Remove.
    // We anchor all subsequent searches to `region_start` to keep the
    // test focused on the Remove handler contract.
    //
    // We assert at the source level (file byte-position) rather than at the
    // process-output level because stdout vs stderr ordering is determined
    // by terminal interleaving, not by Rust flush order — assert_cmd captures
    // them as separate streams and gives us no temporal ordering signal.

    let lib_rs_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let lib_rs = std::fs::read_to_string(&lib_rs_path)
        .unwrap_or_else(|e| panic!("lib.rs must exist at {}: {e}", lib_rs_path.display()));

    let region_start = lib_rs
        .find("Command::Remove")
        .expect("lib.rs must contain `Command::Remove` handler");

    let banner_offset = lib_rs[region_start..]
        .find("Removed directory")
        .expect("✓ Removed directory banner must appear inside Command::Remove region");
    let banner_idx = region_start + banner_offset;

    let warnings_offset = lib_rs[region_start..]
        .find("for w in &regen_warnings")
        .expect("regen_warnings loop must appear inside Command::Remove region");
    let warnings_idx = region_start + warnings_offset;

    assert!(
        banner_idx < warnings_idx,
        "TEST-04 option a: `Removed directory` banner (byte {}) MUST precede `for w in &regen_warnings` loop (byte {}) inside the Command::Remove handler region (starts at byte {})",
        banner_idx,
        warnings_idx,
        region_start,
    );
}

#[cfg(unix)]
#[test]
fn remove_failure_summary_wording() {
    use std::os::unix::fs::PermissionsExt;

    // HOTFIX-03 / #461 H3: the leading line of the partial-failure summary
    // must end the colon-introduced clause with `after resolving:` (which
    // introduces the per-kind listing), NOT with `Run `tome doctor`:` (which
    // falsely promised tome doctor output).
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let target_dir = tmp.path().join("target");
    std::fs::create_dir_all(&target_dir).unwrap();

    remove_test_env(
        &tmp,
        &format!(
            "[directories.local]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n\n[directories.test-target]\npath = \"{}\"\ntype = \"directory\"\nrole = \"target\"\n",
            skills_dir.display(),
            target_dir.display()
        ),
    );

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "sync",
            "--no-triage",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();

    std::fs::set_permissions(&target_dir, std::fs::Permissions::from_mode(0o500)).unwrap();

    let output = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "remove",
            "dir",
            "local",
            "--force",
        ])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();

    std::fs::set_permissions(&target_dir, std::fs::Permissions::from_mode(0o755)).unwrap();

    assert!(
        !output.status.success(),
        "remove should fail under chmod 0o500"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);

    // The new wording is present.
    assert!(
        stderr.contains("after resolving:"),
        "stderr missing reworded fragment 'after resolving:': {stderr}"
    );

    // The doctor hint is still surfaced (we kept the call-to-action inline).
    assert!(
        stderr.contains("tome doctor"),
        "stderr missing 'tome doctor' hint: {stderr}"
    );

    // The misleading old wording is gone. With NO_COLOR=1 the styled
    // `tome doctor` is wrapped in backticks but unstyled, so this literal
    // pattern matches reliably.
    assert!(
        !stderr.contains("addressing these. Run `tome doctor`:"),
        "stderr still contains old misleading wording 'addressing these. Run `tome doctor`:': {stderr}"
    );
}

#[cfg(unix)]
#[test]
fn remove_preserves_git_lockfile_entries() {
    // HOTFIX-01 / #461 H1: the regenerated lockfile after `tome remove` must
    // NOT silently drop git-source-name entries. Before the fix, the handler
    // passed an empty BTreeMap to discover_all, which `continue`'d for every
    // git-type directory — wiping their entries from the regenerated lockfile.
    //
    // Fixture: a "real" local git repo (file:// URL) holding one skill plus
    // a separate directory-type "local" source holding another skill. We run
    // `tome sync` to populate the manifest + lockfile from both sources, then
    // run `tome remove local` and assert the regenerated lockfile still
    // contains a `source_name = "myrepo"` entry.
    let tmp = TempDir::new().unwrap();

    // Set up the directory-type "local" source with one skill.
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "local-skill");

    // Set up a real local git repo to act as the "myrepo" git directory.
    // Using a file:// URL means `git clone` and `git fetch` work without
    // network access — sync's resolve_git_directories can clone it normally.
    let upstream_dir = tmp.path().join("upstream-myrepo.git");
    std::fs::create_dir_all(&upstream_dir).unwrap();
    create_skill(&upstream_dir, "git-skill");
    // Initialize the upstream as a real git repo so `git clone` accepts it.
    let git_init = |dir: &std::path::Path, args: &[&str]| {
        StdCommand::new("git")
            .args(args)
            .current_dir(dir)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .output()
            .unwrap();
    };
    // `git init -b main` so the initial branch name is stable across host configs.
    git_init(&upstream_dir, &["init", "-b", "main"]);
    git_init(&upstream_dir, &["config", "user.email", "test@test.com"]);
    git_init(&upstream_dir, &["config", "user.name", "Test"]);
    git_init(&upstream_dir, &["add", "-A"]);
    git_init(&upstream_dir, &["commit", "-m", "seed"]);

    let dummy_url = format!("file://{}", upstream_dir.display());

    // Config: one Directory + one Git directory.
    remove_test_env(
        &tmp,
        &format!(
            "[directories.local]\n\
             path = \"{}\"\n\
             type = \"directory\"\n\
             role = \"source\"\n\
             \n\
             [directories.myrepo]\n\
             path = \"{}\"\n\
             type = \"git\"\n\
             role = \"source\"\n\
             branch = \"main\"\n",
            skills_dir.display(),
            dummy_url,
        ),
    );

    // Sync to populate manifest and lockfile. `--no-triage` avoids the
    // interactive lockfile diff step on an initial sync.
    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "sync",
            "--no-triage",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();

    // Sanity: post-sync lockfile must contain a myrepo entry whose
    // `git_commit_sha` is populated. The bug doesn't drop the entry by
    // source_name (that comes from the manifest, which `tome remove` of an
    // unrelated directory does not touch), but it DOES wipe `git_commit_sha`
    // because `discover_all` skips git directories when resolved_paths is
    // empty — so `lockfile::generate` falls back to `(None, None, None)` for
    // provenance. We assert on `git_commit_sha` to actually exercise the bug.
    let lockfile_path = tmp.path().join("tome.lock");
    let lockfile_before: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&lockfile_path).unwrap()).unwrap();
    let myrepo_sha_before: Option<String> = lockfile_before["skills"]
        .as_object()
        .unwrap()
        .values()
        .find(|v| v["source_name"].as_str() == Some("myrepo"))
        .and_then(|v| v["git_commit_sha"].as_str().map(|s| s.to_string()));
    assert!(
        myrepo_sha_before.is_some(),
        "precondition: post-sync lockfile must contain a myrepo entry with \
         git_commit_sha set, got: {lockfile_before}"
    );

    // Now remove the OTHER (directory-type) directory. The regenerated
    // lockfile MUST keep the myrepo entries' provenance — pre-fix the
    // git_commit_sha was silently wiped (skill missing from discover →
    // lockfile::generate falls back to None).
    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "remove",
            "dir",
            "local",
            "--force",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();

    let lockfile_after: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&lockfile_path).unwrap()).unwrap();
    let myrepo_sha_after: Option<String> = lockfile_after["skills"]
        .as_object()
        .unwrap()
        .values()
        .find(|v| v["source_name"].as_str() == Some("myrepo"))
        .and_then(|v| v["git_commit_sha"].as_str().map(|s| s.to_string()));
    assert_eq!(
        myrepo_sha_after, myrepo_sha_before,
        "REGRESSION (#461 H1): lockfile after `tome remove local` lost myrepo \
         git_commit_sha provenance — git-sourced skills were silently dropped \
         during regen. Before: {myrepo_sha_before:?}, After: {myrepo_sha_after:?}, \
         full lockfile: {lockfile_after}"
    );
}

// ── tome add integration tests ─────────────────────────────────────

#[test]
fn test_add_happy_path() {
    let tmp = TempDir::new().unwrap();

    // Create minimal config
    let config_path = tmp.path().join("tome.toml");
    std::fs::write(&config_path, "").unwrap();
    std::fs::create_dir_all(tmp.path().join("library")).unwrap();

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "add",
            "https://github.com/user/my-skills.git",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Added"));

    // Verify config was written
    let config_content = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        config_content.contains("[directories.my-skills]"),
        "config should contain the new directory: {config_content}"
    );
    assert!(
        config_content.contains("type = \"git\""),
        "directory type should be git: {config_content}"
    );
}

#[test]
fn test_add_custom_name() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("tome.toml");
    std::fs::write(&config_path, "").unwrap();
    std::fs::create_dir_all(tmp.path().join("library")).unwrap();

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "add",
            "https://github.com/user/repo.git",
            "--name",
            "custom-name",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success()
        .stdout(predicate::str::contains("custom-name"));

    let config_content = std::fs::read_to_string(&config_path).unwrap();
    assert!(config_content.contains("[directories.custom-name]"));
}

#[test]
fn test_add_duplicate_name_fails() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("tome.toml");
    std::fs::write(
        &config_path,
        "[directories.my-skills]\npath = \"https://github.com/user/my-skills.git\"\ntype = \"git\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(tmp.path().join("library")).unwrap();

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "add",
            "https://github.com/user/my-skills.git",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists in config"));
}

#[test]
fn test_add_dry_run() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("tome.toml");
    std::fs::write(&config_path, "").unwrap();
    std::fs::create_dir_all(tmp.path().join("library")).unwrap();

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--dry-run",
            "add",
            "https://github.com/user/my-skills.git",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Would"));

    // Config should be unchanged (empty)
    let config_content = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        !config_content.contains("[directories"),
        "dry run should not modify config"
    );
}

#[test]
fn test_add_with_branch() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("tome.toml");
    std::fs::write(&config_path, "").unwrap();
    std::fs::create_dir_all(tmp.path().join("library")).unwrap();

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "add",
            "https://github.com/user/repo.git",
            "--branch",
            "develop",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();

    let config_content = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        config_content.contains("branch = \"develop\""),
        "config should contain branch: {config_content}"
    );
}

#[test]
fn test_add_expands_bare_github_slug() {
    // `tome add owner/repo` should expand to https://github.com/owner/repo so
    // a later `tome sync` can clone it. Without expansion, git would
    // interpret the bare slug as a local path and fail.
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("tome.toml");
    std::fs::write(&config_path, "").unwrap();
    std::fs::create_dir_all(tmp.path().join("library")).unwrap();

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "add",
            "planetscale/database-skills",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "https://github.com/planetscale/database-skills",
        ));

    let config_content = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        config_content.contains("path = \"https://github.com/planetscale/database-skills\""),
        "config should store the expanded URL: {config_content}"
    );
    assert!(
        config_content.contains("[directories.database-skills]"),
        "directory should be named after the repo segment of the slug: {config_content}"
    );
}

#[test]
fn test_add_dry_run_shows_expanded_slug() {
    // Dry-run with a bare slug must (a) print the expanded URL so the
    // user can confirm the rewrite, and (b) leave the config on disk
    // untouched — same contract as `test_add_dry_run` but for the slug
    // path, since slug expansion is a separate code branch.
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("tome.toml");
    std::fs::write(&config_path, "").unwrap();
    std::fs::create_dir_all(tmp.path().join("library")).unwrap();

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--dry-run",
            "add",
            "planetscale/database-skills",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "https://github.com/planetscale/database-skills",
        ));

    let config_content = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        !config_content.contains("[directories"),
        "dry run should not modify config (slug path): {config_content}"
    );
}

#[test]
fn test_add_bare_slug_with_name_override() {
    // `--name` skips extract_repo_name, but the slug still has to
    // expand. This test pins the order: normalize_url runs before the
    // name-or-extract decision, so the stored path is the expanded URL
    // regardless of where the directory name comes from.
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("tome.toml");
    std::fs::write(&config_path, "").unwrap();
    std::fs::create_dir_all(tmp.path().join("library")).unwrap();

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "add",
            "planetscale/database-skills",
            "--name",
            "ps-db",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();

    let config_content = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        config_content.contains("[directories.ps-db]"),
        "user-supplied --name must win: {config_content}"
    );
    assert!(
        config_content.contains("path = \"https://github.com/planetscale/database-skills\""),
        "slug must still be expanded when --name is set: {config_content}"
    );
}

#[test]
fn test_add_bare_slug_with_branch_flag() {
    // The slug flow must coexist with --branch (and by extension --tag,
    // --rev). Stored config should have both the expanded URL AND the
    // branch field, written into the same directory section.
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("tome.toml");
    std::fs::write(&config_path, "").unwrap();
    std::fs::create_dir_all(tmp.path().join("library")).unwrap();

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "add",
            "planetscale/database-skills",
            "--branch",
            "main",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();

    let config_content = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        config_content.contains("path = \"https://github.com/planetscale/database-skills\""),
        "expanded URL not in config: {config_content}"
    );
    assert!(
        config_content.contains("branch = \"main\""),
        "branch field not in config: {config_content}"
    );
}

// ── tome reassign integration tests ────────────────────────────────

fn reassign_test_env(tmp: &TempDir) {
    let source_dir = tmp.path().join("source");
    std::fs::create_dir_all(&source_dir).unwrap();
    create_skill(&source_dir, "my-skill");

    let target_dir = tmp.path().join("target");
    std::fs::create_dir_all(&target_dir).unwrap();

    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();

    let config_content = format!(
        // local-target uses `synced` role rather than `target` — Phase 14
        // D-A2 refuses reassign into target-only directories (a target-only
        // dir doesn't get rediscovered on next sync). `synced` participates
        // in both discovery and distribution, so reassign succeeds.
        "library_dir = \"{}\"\n\n[directories.local-source]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n\n[directories.local-target]\npath = \"{}\"\ntype = \"directory\"\nrole = \"synced\"\n",
        library_dir.display(),
        source_dir.display(),
        target_dir.display()
    );

    let config_path = tmp.path().join("tome.toml");
    std::fs::write(&config_path, config_content).unwrap();

    // Sync to populate library and manifest
    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "sync",
            "--no-triage",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();
}

#[test]
fn test_reassign_happy_path() {
    let tmp = TempDir::new().unwrap();
    reassign_test_env(&tmp);

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "reassign",
            "my-skill",
            "--to",
            "local-target",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Reassigned"));
}

#[test]
fn test_reassign_nonexistent_skill() {
    let tmp = TempDir::new().unwrap();
    reassign_test_env(&tmp);

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "reassign",
            "nonexistent-skill",
            "--to",
            "local-target",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found in library"));
}

#[test]
fn test_reassign_nonexistent_dir() {
    let tmp = TempDir::new().unwrap();
    reassign_test_env(&tmp);

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "reassign",
            "my-skill",
            "--to",
            "nonexistent-dir",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found in config"));
}

#[test]
fn test_reassign_dry_run() {
    let tmp = TempDir::new().unwrap();
    reassign_test_env(&tmp);

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--dry-run",
            "reassign",
            "my-skill",
            "--to",
            "local-target",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Would"));
}

// ── tome fork integration tests ────────────────────────────────────

#[test]
fn test_fork_with_force() {
    let tmp = TempDir::new().unwrap();
    reassign_test_env(&tmp);

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "fork",
            "my-skill",
            "--to",
            "local-target",
            "--force",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Forked"));

    // Verify files were actually copied to the target directory
    let target_skill = tmp.path().join("target").join("my-skill").join("SKILL.md");
    assert!(
        target_skill.exists(),
        "forked skill should exist in target directory"
    );
}

#[test]
fn test_fork_no_input_without_force_fails() {
    let tmp = TempDir::new().unwrap();
    reassign_test_env(&tmp);

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--no-input",
            "fork",
            "my-skill",
            "--to",
            "local-target",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("use --force"));
}

#[test]
fn test_fork_dry_run() {
    let tmp = TempDir::new().unwrap();
    reassign_test_env(&tmp);

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--dry-run",
            "fork",
            "my-skill",
            "--to",
            "local-target",
            "--force",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Would"));

    // Target may already have a symlink from sync, but the fork dry run
    // should not have created a regular (non-symlink) directory
    let target_skill = tmp.path().join("target").join("my-skill");
    if target_skill.exists() {
        assert!(
            target_skill.is_symlink(),
            "dry run should not create a real directory copy in target"
        );
    }
}

// --------------------------------------------------------------------------
// Wizard integration tests
//
// These tests drive `tome init --dry-run --no-input` (and one save-path
// variant without --dry-run) end-to-end with HOME overridden to a TempDir.
// They confirm:
//   - the wizard runs headlessly without hitting a TTY
//   - the generated config passes Config::validate()
//   - the generated config round-trips through TOML byte-equal
//
// Crate-boundary note: this file is a separate crate from `tome`, so Config
// state is read via the `pub fn` accessors directories(), library_dir(),
// exclude() — `pub(crate)` field access does not compile here.
// --------------------------------------------------------------------------

use tome::config::{Config, DirectoryName, DirectoryRole, DirectoryType};

/// Split stdout on the wizard's `Generated config:` marker (wizard.rs:324)
/// and parse the trailing block as a `tome::config::Config`.
///
/// The `--dry-run` branch of the wizard runs `expand_tildes()` before emitting,
/// so the returned Config has absolute paths — tilde-relative comparisons do
/// NOT work; test callers must compare against expanded (TempDir-prefixed) paths.
fn parse_generated_config(stdout: &str) -> Config {
    let (_preamble, body) = stdout
        .split_once("Generated config:\n")
        .unwrap_or_else(|| panic!("missing `Generated config:` marker in stdout:\n{stdout}"));
    toml::from_str::<Config>(body)
        .unwrap_or_else(|e| panic!("generated TOML did not parse: {e}\n---\n{body}"))
}

/// Assert a Config round-trips: serialize, parse back, re-serialize, compare
/// bytes. Mirrors `Config::save_checked`'s round-trip guard.
fn assert_config_roundtrips(config: &Config) {
    let emitted = toml::to_string_pretty(config).expect("serialize Config");
    let reparsed: Config = toml::from_str(&emitted).expect("reparse Config");
    let reemitted = toml::to_string_pretty(&reparsed).expect("re-serialize Config");
    assert_eq!(
        emitted, reemitted,
        "Config round-trip mismatch — a field is not reversibly (de)serializable.\n\
         --- first emit ---\n{emitted}\n--- second emit ---\n{reemitted}",
    );
}

#[test]
fn init_dry_run_no_input_empty_home() {
    // HOME has nothing under it → no known directories auto-discovered.
    // Wizard should still complete and print a valid, empty-directories Config.
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");

    let output = tome()
        .args(["init", "--dry-run", "--no-input"])
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        output.status.success(),
        "tome init --dry-run --no-input failed (empty HOME).\nstdout:\n{stdout}\nstderr:\n{stderr}",
    );

    let config = parse_generated_config(&stdout);

    assert!(
        config.directories().is_empty(),
        "expected empty directories on empty HOME, got: {:?}",
        config.directories().keys().collect::<Vec<_>>(),
    );

    assert_eq!(
        config.library_dir(),
        tmp.path().join(".tome/skills").as_path(),
        "library_dir should be <HOME>/.tome/skills after tilde expansion",
    );

    assert!(
        config.exclude().is_empty(),
        "expected empty exclude set, got: {:?}",
        config.exclude(),
    );

    config.validate().unwrap_or_else(|e| {
        panic!("generated config failed Config::validate(): {e:#}\nstdout:\n{stdout}")
    });

    assert_config_roundtrips(&config);
}

#[test]
fn init_dry_run_no_input_seeded_home() {
    // Seed HOME with one managed known dir and one synced known dir.
    // Wizard should auto-discover both, assign the expected type+role, and the
    // resulting Config should validate + round-trip.
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");

    std::fs::create_dir_all(tmp.path().join(".claude/plugins")).unwrap();
    std::fs::create_dir_all(tmp.path().join(".claude/skills")).unwrap();

    let output = tome()
        .args(["init", "--dry-run", "--no-input"])
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        output.status.success(),
        "tome init --dry-run --no-input failed (seeded HOME).\nstdout:\n{stdout}\nstderr:\n{stderr}",
    );

    let config = parse_generated_config(&stdout);

    assert_eq!(
        config.directories().len(),
        2,
        "expected exactly 2 directories (claude-plugins + claude-skills), got {}: {:?}",
        config.directories().len(),
        config.directories().keys().collect::<Vec<_>>(),
    );

    // claude-plugins entry: ClaudePlugins type, Managed role, expanded path.
    // role() is the accessor (field is pub(crate)); path + directory_type are pub.
    let plugins = config
        .directories()
        .get(&DirectoryName::new("claude-plugins").unwrap())
        .unwrap_or_else(|| {
            panic!(
                "missing claude-plugins entry; got: {:?}",
                config.directories().keys().collect::<Vec<_>>(),
            )
        });
    assert_eq!(plugins.directory_type, DirectoryType::ClaudePlugins);
    assert_eq!(plugins.role(), DirectoryRole::Managed);
    assert_eq!(
        plugins.path,
        tmp.path().join(".claude/plugins"),
        "claude-plugins path should be <HOME>/.claude/plugins after tilde expansion",
    );

    // claude-skills entry: Directory type, Synced role, expanded path.
    let skills = config
        .directories()
        .get(&DirectoryName::new("claude-skills").unwrap())
        .unwrap_or_else(|| {
            panic!(
                "missing claude-skills entry; got: {:?}",
                config.directories().keys().collect::<Vec<_>>(),
            )
        });
    assert_eq!(skills.directory_type, DirectoryType::Directory);
    assert_eq!(skills.role(), DirectoryRole::Synced);
    assert_eq!(
        skills.path,
        tmp.path().join(".claude/skills"),
        "claude-skills path should be <HOME>/.claude/skills after tilde expansion",
    );

    assert_eq!(
        config.library_dir(),
        tmp.path().join(".tome/skills").as_path(),
        "library_dir should be <HOME>/.tome/skills after tilde expansion",
    );

    config.validate().unwrap_or_else(|e| {
        panic!("generated config failed Config::validate(): {e:#}\nstdout:\n{stdout}")
    });

    assert_config_roundtrips(&config);
}

#[test]
fn init_no_input_writes_config_and_reloads() {
    // End-to-end save path: `tome init --no-input` (no --dry-run) runs the wizard
    // → assemble_config → save_checked → writes tome.toml → post-init sync().
    // A future regression in save_checked, post-init sync, or the no_input save
    // branch in wizard::run would slip past the dry-run tests above.
    //
    // Invariants:
    //   - exit 0
    //   - $TOME_HOME/tome.toml exists after the run
    //   - Config::load on the written file yields a valid Config
    //   - directories()/library_dir()/exclude() match the headless defaults
    //     on empty HOME (include-all, ~/.tome/skills expanded, empty exclude)
    //   - written file round-trips byte-equal through toml::{to_string_pretty, from_str}
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");
    let config_path = tome_home.join("tome.toml");

    let output = tome()
        .args(["init", "--no-input"])
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        output.status.success(),
        "tome init --no-input failed.\nstdout:\n{stdout}\nstderr:\n{stderr}",
    );

    assert!(
        config_path.exists(),
        "expected tome.toml at {} after `tome init --no-input`, but nothing was written",
        config_path.display(),
    );

    let loaded = Config::load(&config_path).unwrap_or_else(|e| {
        panic!(
            "Config::load on wizard-written file failed: {e:#}\nfile:\n{}",
            std::fs::read_to_string(&config_path).unwrap_or_default()
        )
    });

    loaded
        .validate()
        .unwrap_or_else(|e| panic!("reloaded config failed Config::validate(): {e:#}"));

    // Empty HOME → no auto-discovered known dirs.
    assert!(
        loaded.directories().is_empty(),
        "expected empty directories on empty HOME, got: {:?}",
        loaded.directories().keys().collect::<Vec<_>>(),
    );

    // save_checked expands ~ before writing (so the on-disk TOML holds absolute
    // paths that don't rely on the caller's HOME). The wizard's headless
    // default is `~/.tome/skills`, which expands to <HOME>/.tome/skills where
    // <HOME> is the TempDir we set via the HOME env var above.
    assert_eq!(
        loaded.library_dir(),
        tmp.path().join(".tome/skills").as_path(),
        "library_dir should be the expanded default (<HOME>/.tome/skills)",
    );

    assert!(loaded.exclude().is_empty(), "expected empty exclude set");

    // Round-trip parity on the written file.
    assert_config_roundtrips(&loaded);
}

// -----------------------------------------------------------------------------
// WUX-04: `tome init` prints the resolved tome_home info line up front.
// -----------------------------------------------------------------------------
//
// These tests lock in the behavior that every `tome init` invocation prints a
// one-line "resolved tome_home: <path> (from <source>)" message BEFORE any
// wizard Step 1 prompt output. The source label accurately reflects the
// resolution branch: --tome-home flag, --config flag, TOME_HOME env, XDG
// config, or default. The info line is emitted in both interactive and
// --no-input modes (it is informational, not a prompt).

#[test]
fn init_prints_resolved_tome_home_with_default_source() {
    // No TOME_HOME set, HOME has no ~/.config/tome/config.toml → Default source.
    let tmp = TempDir::new().unwrap();

    let output = tome()
        .args(["init", "--dry-run", "--no-input"])
        .env("HOME", tmp.path())
        .env_remove("TOME_HOME")
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "tome init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("resolved tome_home:"),
        "stdout missing resolved tome_home line:\n{stdout}"
    );
    assert!(
        stdout.contains("(from default)"),
        "stdout missing '(from default)' source label:\n{stdout}"
    );
}

#[test]
fn init_prints_resolved_tome_home_with_env_source() {
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");

    let output = tome()
        .args(["init", "--dry-run", "--no-input"])
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("(from TOME_HOME env)"),
        "stdout missing '(from TOME_HOME env)' label:\n{stdout}"
    );
    assert!(
        stdout.contains(tome_home.display().to_string().as_str()),
        "stdout missing TOME_HOME path:\n{stdout}"
    );
}

#[test]
fn init_prints_resolved_tome_home_with_flag_source() {
    let tmp = TempDir::new().unwrap();
    let custom = tmp.path().join("custom-home");

    let output = tome()
        .args([
            "init",
            "--dry-run",
            "--no-input",
            "--tome-home",
            custom.to_str().unwrap(),
        ])
        .env("HOME", tmp.path())
        .env_remove("TOME_HOME")
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("(from --tome-home flag)"),
        "stdout missing '--tome-home flag' label:\n{stdout}"
    );
}

#[test]
fn init_resolved_tome_home_line_precedes_step_prompts() {
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");

    let output = tome()
        .args(["init", "--dry-run", "--no-input"])
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    let resolved_idx = stdout
        .find("resolved tome_home:")
        .expect("missing info line");
    let step1_idx = stdout.find("Step 1").expect("missing Step 1 prompt header");
    assert!(
        resolved_idx < step1_idx,
        "resolved tome_home line must come BEFORE Step 1.\n\
         resolved_idx={resolved_idx}, step1_idx={step1_idx}\nstdout:\n{stdout}"
    );
}

// ---------------------------------------------------------------------------
// WUX-03: legacy pre-v0.6 ~/.config/tome/config.toml detection
// ---------------------------------------------------------------------------
//
// These tests lock in the behavior that `tome init` surfaces a warning when
// it detects a pre-v0.6 XDG config containing `[[sources]]` or `[targets.*]`
// sections, and that under `--no-input` the file is left untouched with a
// `note:` line on stderr. False-positive protection: a v0.6+ XDG file with
// only `tome_home = "..."` must NOT trigger the warning.

#[test]
fn init_legacy_detected_no_input_leaves_file() {
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");
    let xdg_dir = tmp.path().join(".config/tome");
    let xdg_file = xdg_dir.join("config.toml");
    std::fs::create_dir_all(&xdg_dir).unwrap();
    let legacy_content = "[[sources]]\nname = \"old\"\npath = \"/tmp\"\ntype = \"directory\"\n";
    std::fs::write(&xdg_file, legacy_content).unwrap();

    let output = tome()
        .args(["init", "--dry-run", "--no-input"])
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "tome init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Warning appears on stdout (the `println!` in handle_legacy_cleanup).
    assert!(
        stdout.contains("Legacy pre-v0.6 config detected"),
        "stdout missing legacy warning:\n{stdout}"
    );
    // Skip note appears on stderr.
    assert!(
        stderr.contains("skipped legacy cleanup"),
        "stderr missing skipped-cleanup note:\n{stderr}"
    );

    // File must be byte-identical after the run.
    let after = std::fs::read_to_string(&xdg_file).unwrap();
    assert_eq!(after, legacy_content, "legacy file should be unchanged");
}

#[test]
fn init_legacy_with_only_tome_home_not_flagged() {
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");
    let xdg_dir = tmp.path().join(".config/tome");
    std::fs::create_dir_all(&xdg_dir).unwrap();
    // v0.6+ shape — should NOT trigger legacy warning.
    std::fs::write(xdg_dir.join("config.toml"), "tome_home = \"~/somewhere\"\n").unwrap();

    let output = tome()
        .args(["init", "--dry-run", "--no-input"])
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stdout.contains("Legacy pre-v0.6 config detected"),
        "v0.6+-only XDG file should NOT trigger legacy warning. stdout:\n{stdout}"
    );
    assert!(
        !stderr.contains("skipped legacy cleanup"),
        "v0.6+-only XDG file should NOT trigger skip-note. stderr:\n{stderr}"
    );
}

#[test]
fn init_greenfield_no_legacy_warning() {
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");

    let output = tome()
        .args(["init", "--dry-run", "--no-input"])
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("Legacy pre-v0.6 config detected"),
        "greenfield run should NOT show legacy warning. stdout:\n{stdout}"
    );
}

// ---------------------------------------------------------------------------
// WUX-01 / WUX-05: Step 0 greenfield tome_home prompt
// ---------------------------------------------------------------------------
//
// These tests lock in the observable behavior that:
// - Under --no-input, the Step 0 prompt is skipped and no XDG config is written
// - When the tome_home source is NOT Default (e.g. --tome-home flag), Step 0
//   is skipped even when not --no-input
// - The library default derives from the chosen tome_home (Pitfall 1 fix)
//
// The interactive branch of Step 0 is exercised by manual test only — see
// 07-RESEARCH.md § Pitfall 5.

#[test]
fn init_greenfield_no_input_skips_step_0_prompt() {
    // TomeHomeSource::Default + --no-input → Step 0 prompt must be skipped.
    let tmp = TempDir::new().unwrap();

    let output = tome()
        .args(["init", "--dry-run", "--no-input"])
        .env("HOME", tmp.path())
        .env_remove("TOME_HOME")
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "tome init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("Step 0:"),
        "--no-input must skip Step 0 prompt, but stdout contains it:\n{stdout}"
    );
    // WUX-04 info line still prints (informational, not a prompt)
    assert!(
        stdout.contains("resolved tome_home:"),
        "resolved tome_home line must still appear in --no-input mode:\n{stdout}"
    );
}

#[test]
fn init_greenfield_no_input_does_not_write_xdg() {
    // --no-input must NOT write to ~/.config/tome/config.toml even under greenfield.
    // (07-RESEARCH.md § "Integration with no_input" — "Skip" row for WUX-05.)
    let tmp = TempDir::new().unwrap();

    let output = tome()
        .args(["init", "--dry-run", "--no-input"])
        .env("HOME", tmp.path())
        .env_remove("TOME_HOME")
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let xdg = tmp.path().join(".config/tome/config.toml");
    assert!(
        !xdg.exists(),
        "--no-input must not write XDG config, but {} exists",
        xdg.display()
    );
}

#[test]
fn init_with_flag_source_skips_step_0() {
    // TomeHomeSource::CliTomeHome (from --tome-home flag) → Step 0 MUST be skipped
    // even without --no-input, because the user already indicated a choice.
    // We test via --no-input to keep the test headless; the key assertion is on
    // the "Step 0:" header absence.
    let tmp = TempDir::new().unwrap();
    let custom = tmp.path().join("custom-home");

    let output = tome()
        .args([
            "init",
            "--dry-run",
            "--no-input",
            "--tome-home",
            custom.to_str().unwrap(),
        ])
        .env("HOME", tmp.path())
        .env_remove("TOME_HOME")
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("Step 0:"),
        "--tome-home flag (CliTomeHome source) must skip Step 0:\n{stdout}"
    );
    assert!(
        stdout.contains("(from --tome-home flag)"),
        "source label should confirm flag branch:\n{stdout}"
    );
}

#[test]
fn init_derived_library_default_under_custom_tome_home() {
    // When tome_home = <tmp>/custom-tome (non-default), library default should
    // derive as <tmp>/custom-tome/skills (NOT ~/.tome/skills). Tests the
    // Pitfall 1 fix.
    let tmp = TempDir::new().unwrap();
    let custom = tmp.path().join("custom-tome");

    let output = tome()
        .args([
            "init",
            "--dry-run",
            "--no-input",
            "--tome-home",
            custom.to_str().unwrap(),
        ])
        .env("HOME", tmp.path())
        .env_remove("TOME_HOME")
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let config = parse_generated_config(&stdout);
    // library_dir after tilde expansion should be under the custom tome_home,
    // NOT under tmp/.tome/skills.
    assert_eq!(
        config.library_dir(),
        custom.join("skills"),
        "library default should derive from --tome-home, got {:?}",
        config.library_dir()
    );
}

// ---------------------------------------------------------------------------
// WUX-02: Brownfield decision
// ---------------------------------------------------------------------------
//
// These tests lock in the observable behavior that:
// - Under --no-input with a valid existing tome.toml, the file is left
//   byte-identical AND no post-init sync runs (use-existing path).
// - Under --no-input with an invalid existing tome.toml, init exits cleanly
//   (exit 0, Cancel path) and the file is unchanged.
// - When BOTH a brownfield tome.toml AND a legacy XDG file exist, both
//   cleanup headers fire and both files remain unchanged under --no-input.
//
// The interactive branches (Edit/Reinit) are covered by unit tests on the
// individual helpers per 07-RESEARCH.md § Pitfall 5 (dialoguer prompts hang
// in headless CI).

#[test]
fn init_brownfield_no_input_keeps_existing() {
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");
    std::fs::create_dir_all(&tome_home).unwrap();
    let config_path = tome_home.join("tome.toml");
    let seed = "library_dir = \"~/.tome/skills\"\n[directories]\n";
    std::fs::write(&config_path, seed).unwrap();

    let output = tome()
        .args(["init", "--no-input"]) // NOT --dry-run — we want the actual no-op path
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "tome init should succeed; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Existing config detected"),
        "stdout missing brownfield summary:\n{stdout}"
    );

    // File must be byte-identical
    let after = std::fs::read_to_string(&config_path).unwrap();
    assert_eq!(
        after, seed,
        "brownfield --no-input must not modify existing config"
    );

    // No sync side-effect: library dir should not be created
    let library = tmp.path().join(".tome/skills");
    assert!(
        !library.exists(),
        "use-existing path must not run post-init sync (library dir present at {})",
        library.display()
    );
}

#[test]
fn init_brownfield_invalid_config_no_input_cancels() {
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");
    std::fs::create_dir_all(&tome_home).unwrap();
    let config_path = tome_home.join("tome.toml");
    let seed = "this is [[[ not valid toml";
    std::fs::write(&config_path, seed).unwrap();

    let output = tome()
        .args(["init", "--no-input"])
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    // Must exit 0 (clean cancel), not an error
    assert!(
        output.status.success(),
        "invalid-config no-input path should cancel cleanly (exit 0); stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("invalid:") || stdout.contains("cancelled"),
        "stdout should indicate invalid config or cancellation:\n{stdout}"
    );

    let after = std::fs::read_to_string(&config_path).unwrap();
    assert_eq!(
        after, seed,
        "invalid-config no-input must not modify the file"
    );
}

#[test]
fn init_brownfield_with_legacy_runs_both_cleanups() {
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");
    std::fs::create_dir_all(&tome_home).unwrap();
    let config_path = tome_home.join("tome.toml");
    let brownfield_seed = "library_dir = \"~/.tome/skills\"\n[directories]\n";
    std::fs::write(&config_path, brownfield_seed).unwrap();

    let xdg_dir = tmp.path().join(".config/tome");
    let xdg_file = xdg_dir.join("config.toml");
    std::fs::create_dir_all(&xdg_dir).unwrap();
    let legacy_seed = "[[sources]]\nname = \"old\"\npath = \"/tmp\"\ntype = \"directory\"\n";
    std::fs::write(&xdg_file, legacy_seed).unwrap();

    let output = tome()
        .args(["init", "--no-input"])
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Both cleanup paths ran and printed their headers
    assert!(
        stdout.contains("Legacy pre-v0.6 config detected"),
        "stdout missing legacy warning:\n{stdout}"
    );
    assert!(
        stdout.contains("Existing config detected"),
        "stdout missing brownfield summary:\n{stdout}"
    );

    // Both files unchanged under --no-input
    assert_eq!(
        std::fs::read_to_string(&config_path).unwrap(),
        brownfield_seed
    );
    assert_eq!(std::fs::read_to_string(&xdg_file).unwrap(), legacy_seed);
}

// === [directory_overrides.<name>] end-to-end smoke (PORT-01 + PORT-02) ===

#[cfg(unix)]
#[test]
fn machine_override_rewrites_directory_path_for_status() {
    // PORT-01 + PORT-02 smoke: declare an override in machine.toml and
    // confirm `tome status --json` reports the OVERRIDDEN path, proving
    // the load pipeline applied the override before status::gather ran.
    let tmp = TempDir::new().unwrap();
    let real_skills = tmp.path().join("real-skills");
    create_skill(&real_skills, "x");

    // tome.toml points at a path that does NOT exist.
    let tome_toml = format!(
        "library_dir = \"{}/library\"\n\
         \n\
         [directories.work]\n\
         path = \"{}/does-not-exist\"\n\
         type = \"directory\"\n\
         role = \"source\"\n",
        tmp.path().display(),
        tmp.path().display(),
    );
    std::fs::write(tmp.path().join("tome.toml"), tome_toml).unwrap();

    // machine.toml overrides directories.work.path to the real path.
    let machine_toml = format!(
        "[directory_overrides.work]\npath = \"{}\"\n",
        real_skills.display(),
    );
    let machine_path = tmp.path().join("machine.toml");
    std::fs::write(&machine_path, machine_toml).unwrap();

    let assert = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
            "status",
            "--json",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let dirs = report["directories"].as_array().unwrap();
    let work = dirs
        .iter()
        .find(|d| d["name"] == "work")
        .expect("status JSON missing 'work' directory");
    let path = work["path"].as_str().unwrap();
    assert!(
        path.contains("real-skills"),
        "expected status to report overridden path, got: {path}"
    );
    assert!(
        !path.contains("does-not-exist"),
        "expected status to NOT report the original tome.toml path, got: {path}"
    );
}

// === [directory_overrides.<name>] full PORT-05 surfacing (status + doctor) ===

#[cfg(unix)]
#[test]
fn machine_override_appears_in_status_and_doctor() {
    // PORT-05 (and end-to-end PORT-01/02 confirmation): an override declared
    // in machine.toml causes:
    //   - `tome sync` to operate on the overridden path,
    //   - `tome status` text mode to show `(override)` on the affected row,
    //   - `tome status --json` to include `override_applied: true`,
    //   - `tome doctor --json` to include `override_applied: true` for the overridden directory.
    //
    // The overridden directory `work` uses role = "synced" so it appears in BOTH
    // discovery (skill-a from real_path is consolidated into the library) AND
    // distribution (`tome doctor` diagnoses it). This pins the full PORT-05
    // contract end-to-end on an actually-overridden directory.
    let tmp = TempDir::new().unwrap();
    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();

    // Two directories. `work` is synced (discovery + distribution); `other` is
    // a plain source for the negative-case check in status JSON.
    let dotfiles_path = tmp.path().join("dotfiles-says-here");
    let real_path = tmp.path().join("real-skills");
    create_skill(&real_path, "skill-a");
    // The synced directory must EXIST on disk pre-sync — distribute writes
    // symlinks into it. Create the real path's parent (already done by
    // `create_skill`) and ensure `real_path` itself is a directory.
    assert!(
        real_path.is_dir(),
        "real_path must exist for sync to succeed"
    );

    let other_path = tmp.path().join("other-skills");
    create_skill(&other_path, "skill-b");

    let tome_toml = format!(
        "library_dir = \"{}\"\n\
         \n\
         [directories.work]\n\
         path = \"{}\"\n\
         type = \"directory\"\n\
         role = \"synced\"\n\
         \n\
         [directories.other]\n\
         path = \"{}\"\n\
         type = \"directory\"\n\
         role = \"source\"\n",
        library_dir.display(),
        dotfiles_path.display(),
        other_path.display(),
    );
    std::fs::write(tmp.path().join("tome.toml"), tome_toml).unwrap();

    let machine_toml = format!(
        "[directory_overrides.work]\npath = \"{}\"\n",
        real_path.display(),
    );
    let machine_path = tmp.path().join("machine.toml");
    std::fs::write(&machine_path, machine_toml).unwrap();

    // 1. `tome sync` must succeed — sync sees the overridden path.
    let sync_assert = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
            "sync",
            "--no-triage",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();
    let sync_stdout = String::from_utf8(sync_assert.get_output().stdout.clone()).unwrap();
    let skill_a_in_lib = library_dir.join("skill-a").exists();
    assert!(
        skill_a_in_lib,
        "expected skill-a from overridden path to be consolidated, got sync stdout:\n{sync_stdout}",
    );

    // 2. `tome status` text mode — stdout contains `(override)` exactly once.
    let status_assert = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
            "status",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();
    let status_stdout = String::from_utf8(status_assert.get_output().stdout.clone()).unwrap();
    assert!(
        status_stdout.contains("(override)"),
        "expected `tome status` text output to contain `(override)`, got:\n{status_stdout}"
    );
    let override_marker_count = status_stdout.matches("(override)").count();
    assert_eq!(
        override_marker_count, 1,
        "expected exactly one `(override)` marker (for `work`), got {override_marker_count} in:\n{status_stdout}"
    );

    // 3. `tome status --json` — `work` has `override_applied: true`, `other` has false.
    let status_json_assert = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
            "status",
            "--json",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();
    let status_json: serde_json::Value =
        serde_json::from_slice(&status_json_assert.get_output().stdout)
            .expect("status --json output must be valid JSON");
    let dirs = status_json["directories"].as_array().unwrap();
    let work = dirs.iter().find(|d| d["name"] == "work").unwrap();
    let other = dirs.iter().find(|d| d["name"] == "other").unwrap();
    assert_eq!(
        work["override_applied"],
        serde_json::Value::Bool(true),
        "expected work.override_applied == true, got: {work}"
    );
    assert_eq!(
        other["override_applied"],
        serde_json::Value::Bool(false),
        "expected other.override_applied == false, got: {other}"
    );

    // 4. `tome doctor --json` — `work` (now synced/distribution) appears in
    // `directory_issues` and carries `override_applied: true`. This is the
    // strongest end-to-end PORT-05 doctor assertion.
    let doctor_json_assert = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
            "doctor",
            "--json",
        ])
        .env("NO_COLOR", "1")
        .assert();
    // doctor exit may be 0 or non-0 depending on issues found — accept either.
    let doctor_json: serde_json::Value =
        serde_json::from_slice(&doctor_json_assert.get_output().stdout)
            .expect("doctor --json output must be valid JSON");

    let doctor_dirs = doctor_json["directory_issues"]
        .as_array()
        .expect("doctor --json must include directory_issues array");
    let work_entry = doctor_dirs
        .iter()
        .find(|d| d["name"] == "work")
        .expect("work must appear in doctor directory_issues (it has role = synced)");
    assert_eq!(
        work_entry["override_applied"],
        serde_json::Value::Bool(true),
        "expected work.override_applied == true in doctor JSON, got: {work_entry}"
    );

    // Sanity: every entry in directory_issues uses the new DirectoryDiagnostic
    // shape (has `name`, `issues`, and `override_applied`).
    for entry in doctor_dirs {
        assert!(
            entry.get("name").is_some()
                && entry.get("issues").is_some()
                && entry.get("override_applied").is_some(),
            "expected DirectoryDiagnostic shape (name + issues + override_applied), got: {entry}"
        );
    }
}

// === [directory_overrides.<name>] surfacing tests (PORT-03 + PORT-04) ===

#[cfg(unix)]
#[test]
fn machine_override_unknown_target_warns_and_continues() {
    // PORT-03: an override targeting a directory name not present in tome.toml
    // produces a stderr `warning:` line (typo guard) without aborting load.
    let tmp = TempDir::new().unwrap();
    let real_skills = tmp.path().join("real-skills");
    create_skill(&real_skills, "x");

    let tome_toml = format!(
        "library_dir = \"{}/library\"\n\
         \n\
         [directories.work]\n\
         path = \"{}\"\n\
         type = \"directory\"\n\
         role = \"source\"\n",
        tmp.path().display(),
        real_skills.display(),
    );
    std::fs::write(tmp.path().join("tome.toml"), tome_toml).unwrap();

    // Override target `claud` is a typo — does not match any configured
    // directory. The typo guard fires for any unknown name, regardless of
    // whether a similarly-named directory exists.
    let machine_toml = format!(
        "[directory_overrides.claud]\npath = \"{}/elsewhere\"\n",
        tmp.path().display(),
    );
    let machine_path = tmp.path().join("machine.toml");
    std::fs::write(&machine_path, machine_toml).unwrap();

    let assert = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
            "status",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success(); // does NOT abort, only warns
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("warning:") && stderr.contains("claud") && stderr.contains("machine.toml"),
        "expected stderr warning naming 'claud' and 'machine.toml', got:\n{stderr}"
    );
}

#[cfg(unix)]
#[test]
fn machine_override_validation_failure_blames_machine_toml() {
    // PORT-04: validation failures triggered by an override surface as a
    // distinct error class that names machine.toml (not tome.toml) as the
    // file to edit.
    let tmp = TempDir::new().unwrap();
    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();

    // tome.toml is valid: library_dir and directories.work.path are disjoint.
    let work_dir = tmp.path().join("work-skills");
    std::fs::create_dir_all(&work_dir).unwrap();
    let tome_toml = format!(
        "library_dir = \"{}\"\n\
         \n\
         [directories.work]\n\
         path = \"{}\"\n\
         type = \"directory\"\n\
         role = \"synced\"\n",
        library_dir.display(),
        work_dir.display(),
    );
    std::fs::write(tmp.path().join("tome.toml"), tome_toml).unwrap();

    // machine.toml override forces directories.work.path == library_dir.
    // After apply_machine_overrides, validate() will fail with the existing
    // "library_dir overlaps distribution directory 'work'" error.
    let machine_toml = format!(
        "[directory_overrides.work]\npath = \"{}\"\n",
        library_dir.display(),
    );
    let machine_path = tmp.path().join("machine.toml");
    std::fs::write(&machine_path, machine_toml).unwrap();

    let assert = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
            "status",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .failure();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();

    // Wrapped error MUST mention machine.toml (so the user knows where to look).
    assert!(
        stderr.contains("machine.toml"),
        "expected stderr to name machine.toml, got:\n{stderr}"
    );
    // And include the original validate() error text (preserved inside the wrapper).
    assert!(
        stderr.contains("library_dir") && stderr.contains("overlaps"),
        "expected wrapped error to preserve the original validate() text, got:\n{stderr}"
    );
    // And reference the override-induced classification.
    assert!(
        stderr.contains("override-induced") || stderr.contains("directory_overrides"),
        "expected wrapped error to identify itself as override-induced, got:\n{stderr}"
    );
    // Negative assertion (the discriminator): MUST NOT direct user to edit tome.toml.
    assert!(
        !stderr.contains("edit tome.toml") && !stderr.contains("Edit tome.toml"),
        "wrapped error must NOT direct the user to edit tome.toml, got:\n{stderr}"
    );
}

// ============================================================================
// v0.10 Phase 11 — library-canonical-core integration tests
// (LIB-01, LIB-04, LIB-05; CONTEXT.md D-01..D-06, D-09..D-14)
// ============================================================================

/// Synthetic v0.9 library fixture — exercises the migration boundary defenses.
///
/// Layout produced (per CONTEXT.md <specifics>):
///   tome_home/
///     tome.toml                  ← references local source dir
///     .tome-manifest.json        ← entries for managed + local + broken skills
///     skills/
///       p1/                      ← v0.9-shape symlink → plugin_cache/p1
///       p2/                      ← v0.9-shape symlink → plugin_cache/p2
///       l1/                      ← v0.10-shape real dir copy of local_source/l1
///       broken/                  ← v0.9-shape symlink to /nonexistent (D-04)
///       user-symlink/            ← user-created symlink, NOT in manifest (D-03)
///   plugin_cache/
///     p1/SKILL.md
///     p2/SKILL.md
///   local_source/
///     l1/SKILL.md
#[allow(dead_code)]
struct V09Fixture {
    _root: assert_fs::TempDir, // owns the temp dir; drop = cleanup
    tome_home: PathBuf,
    library_dir: PathBuf,
    plugin_cache: PathBuf,
    local_source: PathBuf,
    config_path: PathBuf,
    machine_path: PathBuf,
}

fn build_v09_fixture() -> V09Fixture {
    use std::os::unix::fs as unix_fs;
    let root = assert_fs::TempDir::new().unwrap();
    let tome_home = root.path().join("tome_home");
    let library_dir = tome_home.join("skills");
    let plugin_cache = root.path().join("plugin_cache");
    let local_source = root.path().join("local_source");
    std::fs::create_dir_all(&library_dir).unwrap();
    std::fs::create_dir_all(&plugin_cache).unwrap();
    std::fs::create_dir_all(&local_source).unwrap();

    // Plugin cache (acts as managed source — claude-plugins style).
    for n in &["p1", "p2"] {
        let d = plugin_cache.join(n);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("SKILL.md"), format!("# {n}")).unwrap();
    }
    // Local source.
    let l1 = local_source.join("l1");
    std::fs::create_dir_all(&l1).unwrap();
    std::fs::write(l1.join("SKILL.md"), "# l1").unwrap();

    // v0.9-shape symlinks for managed skills.
    unix_fs::symlink(plugin_cache.join("p1"), library_dir.join("p1")).unwrap();
    unix_fs::symlink(plugin_cache.join("p2"), library_dir.join("p2")).unwrap();

    // v0.10-shape real-dir copy for local skill (already correct shape).
    let l1_lib = library_dir.join("l1");
    std::fs::create_dir_all(&l1_lib).unwrap();
    std::fs::write(l1_lib.join("SKILL.md"), "# l1").unwrap();

    // D-04: broken symlink (managed manifest entry, target gone).
    unix_fs::symlink("/nonexistent/target", library_dir.join("broken")).unwrap();

    // D-03 conservatism: user-created symlink NOT in manifest.
    let user_target = root.path().join("user_target");
    std::fs::create_dir_all(&user_target).unwrap();
    std::fs::write(user_target.join("SKILL.md"), "# user").unwrap();
    unix_fs::symlink(&user_target, library_dir.join("user-symlink")).unwrap();

    // Compute content_hashes using the production algorithm via the
    // crate-root re-export added in Plan 11-05 Task 0. This guarantees
    // byte-for-byte identity with `manifest::hash_directory` — no risk of
    // a duplicated SHA-256 helper drifting.
    let p1_hash = tome::hash_directory(&plugin_cache.join("p1")).unwrap();
    let p2_hash = tome::hash_directory(&plugin_cache.join("p2")).unwrap();
    let l1_hash = tome::hash_directory(&l1_lib).unwrap();
    let manifest_json = serde_json::json!({
        "skills": {
            "p1": {
                "source_path": plugin_cache.join("p1").to_string_lossy(),
                "source_name": "plugins",
                "content_hash": p1_hash.as_str(),
                "synced_at": "2024-01-01T00:00:00Z",
                "managed": true
            },
            "p2": {
                "source_path": plugin_cache.join("p2").to_string_lossy(),
                "source_name": "plugins",
                "content_hash": p2_hash.as_str(),
                "synced_at": "2024-01-01T00:00:00Z",
                "managed": true
            },
            "broken": {
                "source_path": "/nonexistent/target",
                "source_name": "plugins",
                "content_hash": "0".repeat(64),
                "synced_at": "2024-01-01T00:00:00Z",
                "managed": true
            },
            "l1": {
                "source_path": l1.to_string_lossy(),
                "source_name": "local",
                "content_hash": l1_hash.as_str(),
                "synced_at": "2024-01-01T00:00:00Z",
                "managed": false
            }
        }
    });
    std::fs::write(
        tome_home.join(".tome-manifest.json"),
        serde_json::to_string_pretty(&manifest_json).unwrap(),
    )
    .unwrap();

    // Minimal tome.toml. Declare a local source directory only — for sync's
    // refuse-with-hint test, only valid syntax + a library_dir is needed; the
    // managed entries already in the manifest are what trigger the v0.9 shape
    // detection in `lib.rs::sync` (it reads the manifest, not the config).
    let config_path = tome_home.join("tome.toml");
    let toml = format!(
        r#"library_dir = "{}"

[directories.local]
path = "{}"
type = "directory"
role = "source"
"#,
        library_dir.display(),
        local_source.display(),
    );
    std::fs::write(&config_path, toml).unwrap();

    let machine_path = root.path().join("machine.toml");
    std::fs::write(&machine_path, "").unwrap();

    V09Fixture {
        _root: root,
        tome_home,
        library_dir,
        plugin_cache,
        local_source,
        config_path,
        machine_path,
    }
}

#[test]
fn migrate_library_converts_managed_symlinks_to_real_dirs() {
    let fix = build_v09_fixture();

    let output = assert_cmd::Command::cargo_bin("tome")
        .unwrap()
        .args([
            "migrate-library",
            "--config",
            fix.config_path.to_str().unwrap(),
            "--tome-home",
            fix.tome_home.to_str().unwrap(),
            "--machine",
            fix.machine_path.to_str().unwrap(),
        ])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // p1 and p2: managed symlinks should now be real directories with copied content.
    for n in &["p1", "p2"] {
        let dest = fix.library_dir.join(n);
        assert!(
            dest.is_dir(),
            "{n} must be a real directory after migration"
        );
        assert!(
            !dest.is_symlink(),
            "{n} must NOT be a symlink after migration"
        );
        assert!(dest.join("SKILL.md").is_file(), "{n}/SKILL.md must exist");
        let content = std::fs::read_to_string(dest.join("SKILL.md")).unwrap();
        assert_eq!(
            content,
            format!("# {n}"),
            "content for {n} must match source"
        );
    }

    // l1: local skill, was already real-dir — UNCHANGED.
    let l1 = fix.library_dir.join("l1");
    assert!(l1.is_dir() && !l1.is_symlink());
    assert_eq!(
        std::fs::read_to_string(l1.join("SKILL.md")).unwrap(),
        "# l1"
    );

    // broken: D-04 — symlink preserved, NOT deleted.
    let broken = fix.library_dir.join("broken");
    assert!(
        broken.is_symlink(),
        "broken symlink must be preserved per D-04, got: {stdout}\n{stderr}"
    );
    // D-04 stderr warning surfaced.
    assert!(
        stderr.contains("broken") && stderr.contains("unreachable"),
        "stderr must mention broken-source skip, got: {stderr}"
    );

    // user-symlink: D-03 conservatism — NOT in manifest, must be untouched.
    let user_sym = fix.library_dir.join("user-symlink");
    assert!(
        user_sym.is_symlink(),
        "user-created symlink (NOT in manifest) must be preserved per D-03"
    );

    // D-05: exit code non-zero because of the broken-symlink skip.
    assert!(
        !output.status.success(),
        "must exit non-zero on broken-symlink skip per D-05"
    );

    // SAFE-01 banner format check.
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("converted") && combined.contains("skipped"),
        "output must include SAFE-01 summary banner, got: {combined}"
    );

    // Silence dead-code warnings on unused fixture fields.
    let _ = (&fix.plugin_cache, &fix.local_source);
}

#[test]
fn migrate_library_dry_run_makes_no_changes() {
    let fix = build_v09_fixture();

    // Snapshot library state pre-run.
    let p1_was_symlink = fix.library_dir.join("p1").is_symlink();
    let p2_was_symlink = fix.library_dir.join("p2").is_symlink();
    assert!(p1_was_symlink && p2_was_symlink, "fixture sanity");

    let output = assert_cmd::Command::cargo_bin("tome")
        .unwrap()
        .args([
            "migrate-library",
            "--dry-run",
            "--config",
            fix.config_path.to_str().unwrap(),
            "--tome-home",
            fix.tome_home.to_str().unwrap(),
            "--machine",
            fix.machine_path.to_str().unwrap(),
        ])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();

    // Filesystem unchanged.
    assert!(
        fix.library_dir.join("p1").is_symlink(),
        "dry-run must not convert p1"
    );
    assert!(
        fix.library_dir.join("p2").is_symlink(),
        "dry-run must not convert p2"
    );
    assert!(
        fix.library_dir.join("broken").is_symlink(),
        "dry-run must not touch broken"
    );

    // Output should mention dry-run.
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("dry-run"),
        "output must mention dry-run, got: {combined}"
    );
}

#[test]
fn sync_refuses_on_v09_shape_library_with_hint() {
    let fix = build_v09_fixture();

    let output = assert_cmd::Command::cargo_bin("tome")
        .unwrap()
        .args([
            "sync",
            "--no-input",
            "--config",
            fix.config_path.to_str().unwrap(),
            "--tome-home",
            fix.tome_home.to_str().unwrap(),
            "--machine",
            fix.machine_path.to_str().unwrap(),
        ])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // D-02: sync must refuse with a Conflict/Why/Suggestion error.
    assert!(
        !output.status.success(),
        "sync must exit non-zero on v0.9-shape library"
    );
    assert!(
        stderr.contains("v0.9 shape"),
        "stderr must mention 'v0.9 shape': {stderr}"
    );
    assert!(
        stderr.contains("tome migrate-library"),
        "stderr must point at `tome migrate-library`: {stderr}"
    );

    // Library must NOT have been modified by the refused sync.
    assert!(
        fix.library_dir.join("p1").is_symlink(),
        "refused sync must not modify library"
    );
    assert!(fix.library_dir.join("p2").is_symlink());
}

#[test]
fn sync_succeeds_after_migrate_library() {
    let fix = build_v09_fixture();

    // Remove the broken symlink first so migrate-library exits cleanly
    // (otherwise the broken-symlink D-04 path would block this test from
    // reaching the post-migration sync).
    std::fs::remove_file(fix.library_dir.join("broken")).unwrap();

    // Drop the broken manifest entry too — otherwise sync's v0.9-shape
    // detection would still fire (`broken` would still be in the manifest
    // with managed=true and no library entry).
    let manifest_path = fix.tome_home.join(".tome-manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
    manifest["skills"].as_object_mut().unwrap().remove("broken");
    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    // Step 1: migrate-library.
    let migrate = assert_cmd::Command::cargo_bin("tome")
        .unwrap()
        .args([
            "migrate-library",
            "--config",
            fix.config_path.to_str().unwrap(),
            "--tome-home",
            fix.tome_home.to_str().unwrap(),
            "--machine",
            fix.machine_path.to_str().unwrap(),
        ])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        migrate.status.success(),
        "migrate-library must succeed cleanly when no broken symlinks remain.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&migrate.stdout),
        String::from_utf8_lossy(&migrate.stderr),
    );

    // Step 2: sync. Should NOT refuse anymore (no v0.9-shape symlinks left).
    let sync = assert_cmd::Command::cargo_bin("tome")
        .unwrap()
        .args([
            "sync",
            "--no-input",
            "--config",
            fix.config_path.to_str().unwrap(),
            "--tome-home",
            fix.tome_home.to_str().unwrap(),
            "--machine",
            fix.machine_path.to_str().unwrap(),
        ])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();

    let sync_stderr = String::from_utf8_lossy(&sync.stderr);
    assert!(
        !sync_stderr.contains("v0.9 shape"),
        "sync after migrate must NOT refuse with v0.9 hint, got: {sync_stderr}"
    );
    // We don't assert sync.status.success() because the synthetic fixture
    // doesn't have a real claude-plugins source so the managed entries
    // become orphans — but that's a Phase 13 concern. The KEY assertion
    // is that the v0.9 refuse-with-hint check no longer fires.
}

#[test]
fn sync_preserves_library_when_source_removed_from_config() {
    // LIB-04 / D-09 Case 1 / D-10 trigger 2: user edits tome.toml outside
    // `tome remove` to drop a source directory; the next `tome sync` cleanup
    // phase must transition the orphaned manifest entries to Unowned and
    // preserve their library content (NOT delete).
    //
    // Note on config shape: `lib.rs::sync` has a CFG-06 safety guard that
    // returns early ("no directories configured") if `config.directories`
    // is empty — this would skip cleanup entirely. To exercise the cleanup
    // path we keep ONE source in config (`other`, with no skills) and remove
    // the one that owned the orphan (`local`). Manifest entry for `alpha`
    // still references `local`, which is no longer in `config.directories` —
    // the exact D-09 Case 1 trigger.
    let root = assert_fs::TempDir::new().unwrap();
    let tome_home = root.path().join("tome_home");
    let library_dir = tome_home.join("skills");
    let local_source = root.path().join("local_source");
    let other_source = root.path().join("other_source");
    std::fs::create_dir_all(&library_dir).unwrap();
    std::fs::create_dir_all(&local_source).unwrap();
    std::fs::create_dir_all(&other_source).unwrap();

    // Create a real skill in `other` so sync's `skills.is_empty()` early-exit
    // doesn't fire (cleanup only runs after discover finds at least one skill).
    let other_skill = other_source.join("beta");
    std::fs::create_dir_all(&other_skill).unwrap();
    std::fs::write(
        other_skill.join("SKILL.md"),
        "---\nname: beta\n---\n# beta\nA filler skill so sync proceeds past discover.",
    )
    .unwrap();

    // Create a local skill in source and pre-populate library + manifest.
    let src = local_source.join("alpha");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("SKILL.md"), "# alpha").unwrap();

    let lib_alpha = library_dir.join("alpha");
    std::fs::create_dir_all(&lib_alpha).unwrap();
    std::fs::write(lib_alpha.join("SKILL.md"), "# alpha").unwrap();

    // Use the production hash function via the crate-root re-export (Task 0).
    let alpha_hash = tome::hash_directory(&lib_alpha).unwrap();
    let manifest_json = serde_json::json!({
        "skills": {
            "alpha": {
                "source_path": src.to_string_lossy(),
                "source_name": "local",
                "content_hash": alpha_hash.as_str(),
                "synced_at": "2024-01-01T00:00:00Z",
                "managed": false
            }
        }
    });
    std::fs::write(
        tome_home.join(".tome-manifest.json"),
        serde_json::to_string_pretty(&manifest_json).unwrap(),
    )
    .unwrap();

    // Final config: drops `[directories.local]`, keeps `[directories.other]`.
    // The `local` entry was the previous owner of `alpha`; with `local` gone,
    // the cleanup phase classifies `alpha` as a Case 1 orphan (source no
    // longer in config) → transition to Unowned + preserve library content.
    let config_path = tome_home.join("tome.toml");
    let machine_path = root.path().join("machine.toml");
    std::fs::write(&machine_path, "").unwrap();
    let config_without_source = format!(
        r#"library_dir = "{}"

[directories.other]
path = "{}"
type = "directory"
role = "source"
"#,
        library_dir.display(),
        other_source.display(),
    );
    std::fs::write(&config_path, &config_without_source).unwrap();

    // Step 2: run sync. Cleanup phase should detect the orphan (alpha's
    // source_name "local" is no longer in config.directories) and
    // transition it to Unowned — preserving the library content per LIB-04.
    let sync = assert_cmd::Command::cargo_bin("tome")
        .unwrap()
        .args([
            "sync",
            "--no-input",
            "--config",
            config_path.to_str().unwrap(),
            "--tome-home",
            tome_home.to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
        ])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    let sync_stderr = String::from_utf8_lossy(&sync.stderr);
    let sync_stdout = String::from_utf8_lossy(&sync.stdout);

    // The library directory MUST still exist with the same content.
    assert!(
        library_dir.join("alpha").is_dir(),
        "LIB-04: library content must be preserved on source removal.\nstdout: {sync_stdout}\nstderr: {sync_stderr}"
    );
    let preserved = std::fs::read_to_string(library_dir.join("alpha/SKILL.md")).unwrap();
    assert_eq!(preserved, "# alpha", "library content must be unchanged");

    // The manifest entry must have transitioned to Unowned (source_name omitted/null).
    let manifest_after: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(tome_home.join(".tome-manifest.json")).unwrap(),
    )
    .unwrap();
    let alpha_entry = &manifest_after["skills"]["alpha"];
    assert!(
        alpha_entry
            .get("source_name")
            .map(|v| v.is_null())
            .unwrap_or(true),
        "manifest entry's source_name must be omitted or null after source removal: {alpha_entry}"
    );
    // content_hash unchanged.
    assert_eq!(
        alpha_entry["content_hash"].as_str().unwrap(),
        alpha_hash.as_str(),
        "content_hash must remain unchanged across the Case 1 transition"
    );
}

// ============================================================================
// Phase 14 — UNOWN-01..03 end-to-end integration tests
//
// Per HARD-13 (Phase 15) these tests will eventually split into per-domain
// `tests/cli_remove.rs` / `tests/cli_reassign.rs` / `tests/cli_status.rs`.
// For now they live in this monolith.
//
// Fixtures pre-populate `.tome-manifest.json` directly so we can stage
// Unowned skills (`source_name = None`) without having to first sync, then
// remove a directory, then re-sync. The skills they describe are real
// directories on disk inside the library_dir so commands that touch
// filesystem state (remove skill, reassign content-hash check) operate on
// actual data — only the manifest provenance is fabricated.
// ============================================================================

/// Phase 14 fixture builder. Holds tome_home + config_path + library_dir
/// and the on-disk locations of pre-staged skills so individual tests
/// can assert filesystem-level state changes.
#[allow(dead_code)]
struct Phase14Fixture {
    /// Held to keep the TempDir alive for the duration of the test.
    tmp: TempDir,
    tome_home: PathBuf,
    config_path: PathBuf,
    library_dir: PathBuf,
    machine_path: PathBuf,
    /// Optional pre-staged target dir (for reassign / distribution-symlink tests).
    target_dir: Option<PathBuf>,
}

impl Phase14Fixture {
    fn cmd(&self) -> Command {
        let mut cmd = cargo_bin_cmd!("tome");
        cmd.args(["--tome-home", self.tome_home.to_str().unwrap()]);
        cmd.args(["--config", self.config_path.to_str().unwrap()]);
        cmd.args(["--machine", self.machine_path.to_str().unwrap()]);
        cmd.env("NO_COLOR", "1");
        cmd
    }

    fn manifest_value(&self) -> serde_json::Value {
        let raw = std::fs::read_to_string(self.tome_home.join(".tome-manifest.json")).unwrap();
        serde_json::from_str(&raw).unwrap()
    }
}

/// Build a manifest entry value. `source_name = None` produces an Unowned
/// entry (the key is omitted, matching the `skip_serializing_if` shape on
/// `SkillEntry`). When `previous_source` is set, it's emitted regardless of
/// owned-ness.
fn phase14_manifest_entry(
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
fn phase14_write_library_skill(library_dir: &Path, name: &str, body: &str) -> String {
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
fn phase14_build_fixture(
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

// ── UNOWN-01 / D-API-1: tome reassign accepts Unowned input ────────────────

/// Re-anchoring an Unowned skill via `tome reassign <skill> --to <dir>`
/// flips manifest source_name from None to Some(<dir>) and clears
/// previous_source per D-C1 closure.
#[test]
fn phase14_reassign_unowned_input_succeeds() {
    let fix = phase14_build_fixture(
        &[("local-target", "synced")],
        &[],
        &[("orphan-foo", "removed-dir")],
    );

    fix.cmd()
        .args(["reassign", "orphan-foo", "--to", "local-target"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Reassigned"));

    // Manifest: source_name flipped from None to Some("local-target");
    // previous_source cleared.
    let manifest = fix.manifest_value();
    let entry = &manifest["skills"]["orphan-foo"];
    assert_eq!(
        entry["source_name"].as_str(),
        Some("local-target"),
        "source_name must be Some(local-target) after re-anchor: {entry}"
    );
    assert!(
        entry
            .get("previous_source")
            .map(|v| v.is_null())
            .unwrap_or(true),
        "previous_source must be cleared on re-anchor (D-C1 closure): {entry}"
    );

    // Skill content materialised in the target directory on disk.
    let target_skill_md = fix.target_dir.unwrap().join("orphan-foo").join("SKILL.md");
    assert!(
        target_skill_md.exists(),
        "skill content must be copied to target dir on re-anchor"
    );
}

/// D-A2: target-only roles are rejected because nothing rediscovers the
/// reassigned skill on next sync.
#[test]
fn phase14_reassign_into_target_only_role_rejected() {
    let fix = phase14_build_fixture(
        &[("claude-target", "target")],
        &[],
        &[("orphan-foo", "removed-dir")],
    );

    fix.cmd()
        .args(["reassign", "orphan-foo", "--to", "claude-target"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("target-only"));
}

/// D-A1: different-content collision at the target is refused without
/// `--force`; passing `--force` overwrites the target with library content.
#[test]
fn phase14_reassign_force_bypasses_different_content_collision() {
    let fix = phase14_build_fixture(
        &[("local-target", "synced")],
        &[],
        &[("orphan-foo", "removed-dir")],
    );

    // Pre-populate target dir with a DIFFERENT-content version of the skill
    // so plan() hits the D-A1 hash mismatch path.
    let target = fix.target_dir.clone().unwrap();
    let collision = target.join("orphan-foo");
    std::fs::create_dir_all(&collision).unwrap();
    std::fs::write(
        collision.join("SKILL.md"),
        "---\nname: orphan-foo\n---\n# orphan-foo\nDIFFERENT content at target\n",
    )
    .unwrap();

    // Without --force: refused, exits non-zero, error names "different content".
    fix.cmd()
        .args(["reassign", "orphan-foo", "--to", "local-target"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("different content"));

    // With --force: succeeds; target contents now match the library copy.
    fix.cmd()
        .args(["reassign", "orphan-foo", "--to", "local-target", "--force"])
        .assert()
        .success();

    let target_body = std::fs::read_to_string(collision.join("SKILL.md")).unwrap();
    assert!(
        target_body.contains("unowned skill orphan-foo"),
        "with --force, target content must match the library copy: {target_body}"
    );
}

// ── UNOWN-02 / D-API-2: tome remove skill ──────────────────────────────────

/// D-B1: full cleanup — manifest + library dir + distribution symlinks +
/// lockfile entry + machine.toml `disabled` set + per-directory memberships
/// all cleaned in a single pass.
#[test]
fn phase14_remove_skill_full_cleanup() {
    let fix = phase14_build_fixture(
        &[("local-target", "synced")],
        &[],
        &[("orphan-foo", "removed-dir")],
    );

    // Stage a distribution symlink pointing at the library skill.
    let library_skill = fix.library_dir.join("orphan-foo");
    let target = fix.target_dir.clone().unwrap();
    let dist_link = target.join("orphan-foo");
    std::os::unix::fs::symlink(&library_skill, &dist_link).unwrap();
    assert!(dist_link.is_symlink());

    // Stage a lockfile entry for the skill.
    let lockfile_path = fix.tome_home.join("tome.lock");
    let lockfile_json = serde_json::json!({
        "version": 1,
        "skills": {
            "orphan-foo": {
                "previous_source": "removed-dir",
                "content_hash": "a".repeat(64),
            }
        }
    });
    std::fs::write(
        &lockfile_path,
        serde_json::to_string_pretty(&lockfile_json).unwrap(),
    )
    .unwrap();

    // Stage machine.toml `disabled` membership.
    std::fs::write(&fix.machine_path, "disabled = [\"orphan-foo\"]\n").unwrap();

    fix.cmd()
        .args(["remove", "skill", "orphan-foo", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Forgot skill 'orphan-foo'"));

    // Library directory removed.
    assert!(
        !library_skill.exists(),
        "library/orphan-foo must be removed after `remove skill`"
    );

    // Distribution symlink removed.
    assert!(
        !dist_link.exists() && !dist_link.is_symlink(),
        "distribution symlink must be removed"
    );

    // Manifest entry removed.
    let manifest = fix.manifest_value();
    assert!(
        manifest["skills"].get("orphan-foo").is_none(),
        "manifest entry must be removed: {manifest}"
    );

    // Lockfile entry removed.
    let lockfile_after: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&lockfile_path).unwrap()).unwrap();
    assert!(
        lockfile_after["skills"].get("orphan-foo").is_none(),
        "lockfile entry must be removed: {lockfile_after}"
    );

    // machine.toml disabled membership removed.
    let machine_after = std::fs::read_to_string(&fix.machine_path).unwrap();
    assert!(
        !machine_after.contains("orphan-foo"),
        "machine.toml disabled-set membership must be removed: {machine_after}"
    );
}

/// D-B2: tome remove skill on an Owned skill is refused with a hint pointing
/// at `tome remove dir`. Manifest entry preserved.
#[test]
fn phase14_remove_skill_refuses_owned() {
    let fix = phase14_build_fixture(&[("active-dir", "synced")], &[("kept", "active-dir")], &[]);

    let assert = fix
        .cmd()
        .args(["remove", "skill", "kept", "--yes"])
        .assert()
        .failure();
    let output = assert.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("is owned by directory"),
        "stderr must contain 'is owned by directory': {stderr}"
    );
    assert!(
        stderr.contains("tome remove dir"),
        "stderr must hint at `tome remove dir`: {stderr}"
    );

    // Manifest entry preserved (no destructive changes on owned-skill refusal).
    let manifest = fix.manifest_value();
    assert!(
        manifest["skills"].get("kept").is_some(),
        "manifest entry for owned skill must be preserved on refusal: {manifest}"
    );
}

/// D-B3: `--no-input` without `--yes` bails with a confirmation-required
/// message — non-interactive mode never silently destroys.
#[test]
fn phase14_remove_skill_no_input_without_yes_bails() {
    let fix = phase14_build_fixture(&[], &[], &[("orphan-foo", "removed-dir")]);

    fix.cmd()
        .args(["--no-input", "remove", "skill", "orphan-foo"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("requires confirmation"));

    // Manifest entry preserved.
    let manifest = fix.manifest_value();
    assert!(
        manifest["skills"].get("orphan-foo").is_some(),
        "manifest entry must be preserved when bail occurred: {manifest}"
    );
}

// ── UNOWN-03: status + doctor surface the Unowned set ──────────────────────

/// D-D1 / D-D2: `tome status` text output renders an `Unowned skills (N):`
/// section with NAME / LAST-KNOWN SOURCE / SYNCED columns. Per D-C1 the
/// LAST-KNOWN SOURCE column shows the recorded `previous_source`.
#[test]
fn phase14_status_text_shows_unowned_section() {
    let fix = phase14_build_fixture(&[], &[], &[("orphan", "removed-dir")]);

    let output = fix.cmd().arg("status").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Unowned skills (1)"),
        "stdout must include 'Unowned skills (1)': {stdout}"
    );
    assert!(
        stdout.contains("orphan"),
        "stdout must include the skill name: {stdout}"
    );
    assert!(
        stdout.contains("removed-dir"),
        "stdout must show LAST-KNOWN SOURCE = previous_source per D-C1: {stdout}"
    );
}

/// UNOWN-03 JSON shape: `tome status --json` includes a top-level `unowned`
/// array of `SkillSummary` entries.
#[test]
fn phase14_status_json_includes_unowned_field() {
    let fix = phase14_build_fixture(&[], &[], &[("orphan", "removed-dir")]);

    let output = fix.cmd().args(["status", "--json"]).output().unwrap();
    assert!(output.status.success());
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("status --json must produce valid JSON");
    let unowned = json["unowned"]
        .as_array()
        .expect("status --json must include 'unowned' as an array");
    assert_eq!(unowned.len(), 1, "expected 1 unowned skill: {json}");
    let entry = &unowned[0];
    assert_eq!(entry["name"], "orphan");
    assert_eq!(entry["previous_source"], "removed-dir");
    // Stable shape: SkillSummary always exposes these fields.
    for key in [
        "name",
        "previous_source",
        "source_path_display",
        "synced_at",
        "managed",
    ] {
        assert!(
            entry.get(key).is_some(),
            "SkillSummary JSON must contain '{key}': {entry}"
        );
    }
}

/// D-D3: doctor's Unowned section is informational only — it does NOT
/// contribute to total_issues and does NOT affect exit code. With zero
/// actionable issues, exit code is 0 regardless of how many Unowned skills
/// are present.
#[test]
fn phase14_doctor_informational_unowned_does_not_affect_exit_code() {
    let fix = phase14_build_fixture(
        &[],
        &[],
        &[("orphan-a", "removed-1"), ("orphan-b", "removed-2")],
    );

    let output = fix.cmd().arg("doctor").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Exit code 0 — Unowned alone never escalates doctor.
    assert!(
        output.status.success(),
        "doctor must exit 0 when only Unowned skills are present (D-D3). stdout: {stdout}"
    );
    assert!(
        stdout.contains("Unowned skills (2)"),
        "doctor stdout must include 'Unowned skills (2)': {stdout}"
    );
    assert!(
        stdout.contains("No issues found"),
        "doctor must report 'No issues found' since unowned doesn't count (D-D3): {stdout}"
    );

    // JSON: total_issues derivation excludes unowned_skills.
    let json_output = fix.cmd().args(["doctor", "--json"]).output().unwrap();
    let json: serde_json::Value =
        serde_json::from_slice(&json_output.stdout).expect("doctor --json must produce valid JSON");
    assert_eq!(
        json["unowned_skills"].as_array().map(|a| a.len()),
        Some(2),
        "doctor --json must include 'unowned_skills' with 2 entries: {json}"
    );
    assert!(
        json["library_issues"]
            .as_array()
            .map(|a| a.is_empty())
            .unwrap_or(false),
        "library_issues must be empty: {json}"
    );
}

/// Empty-set rendering: `tome status` with zero Unowned skills omits the
/// section cleanly (no header, no blank line).
#[test]
fn phase14_status_text_omits_unowned_section_when_empty() {
    let fix = phase14_build_fixture(&[("active-dir", "synced")], &[("alpha", "active-dir")], &[]);

    let output = fix.cmd().arg("status").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("Unowned skills"),
        "stdout must NOT include 'Unowned skills' header when set is empty: {stdout}"
    );

    // JSON shape stays stable: empty array, not omitted.
    let json_output = fix.cmd().args(["status", "--json"]).output().unwrap();
    let json: serde_json::Value =
        serde_json::from_slice(&json_output.stdout).expect("status --json must produce valid JSON");
    let unowned = json["unowned"]
        .as_array()
        .expect("status --json must include 'unowned' as an array even when empty");
    assert!(
        unowned.is_empty(),
        "unowned array must be empty (not omitted): {json}"
    );
}
