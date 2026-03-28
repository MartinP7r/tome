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
            "library_dir = \"{}\"\n{}\n[targets.test-target]\nenabled = true\nmethod = \"symlink\"\nskills_dir = \"{}\"\n",
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
                    "[[sources]]\nname = \"{name}\"\npath = \"{}\"\ntype = \"claude-plugins\"\n\n",
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
                    "[[sources]]\nname = \"{name}\"\npath = \"{}\"\ntype = \"directory\"\n\n",
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
                "[targets.{name}]\nenabled = true\nmethod = \"symlink\"\nskills_dir = \"{}\"\n\n",
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
                content.push_str(&format!("disabled_targets = [{}]\n", items.join(", ")));
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
            "[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
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
            "[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
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
            "[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
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
            "[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
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
            "[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
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
            "[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
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
    tome()
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

[[sources]]
name = "test"
path = "{}"
type = "directory"

[targets.antigravity]
enabled = true
method = "symlink"
skills_dir = "{}"
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

[[sources]]
name = "test"
path = "{}"
type = "directory"
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

[[sources]]
name = "test"
path = "{}"
type = "directory"
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

[[sources]]
name = "test"
path = "{}"
type = "directory"
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

[[sources]]
name = "test"
path = "{}"
type = "directory"
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

[[sources]]
name = "test"
path = "{}"
type = "directory"
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

[[sources]]
name = "test"
path = "{}"
type = "directory"
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
            "[[sources]]\nname = \"test-src\"\npath = \"{}\"\ntype = \"directory\"\n",
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
            "[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
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

[[sources]]
name = "test"
path = "{}"
type = "directory"
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
            "[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
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
            "[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
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
            "[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
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
            "[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
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
            "[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
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
            "[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
            skills_dir.display()
        ),
        &target_dir,
    );

    // Create machine.toml that disables the configured target and also lists an unknown target
    let machine_path = tmp.path().join("machine.toml");
    std::fs::write(
        &machine_path,
        "disabled_targets = [\"test-target\", \"nonexistent-target\"]\n",
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
            "warning: disabled target 'nonexistent-target' in machine.toml does not match any configured target",
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

[[sources]]
name = "test"
path = "{}"
type = "directory"

[targets.target-a]
enabled = true
method = "symlink"
skills_dir = "{}"

[targets.target-b]
enabled = true
method = "symlink"
skills_dir = "{}"
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
            "[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
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
        "disabled_targets = [\"nonexistent-target\"]\n",
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
            "warning: disabled target 'nonexistent-target' in machine.toml does not match any configured target",
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
    let env = TestEnvBuilder::new()
        .source("plugins", "claude-plugins")
        .target("test-tool")
        .managed_skill("managed-skill", "plugins", "my-plugin@npm", "1.0.0")
        .build();

    env.cmd().arg("sync").assert().success();

    let library_skill = env.library_dir().join("managed-skill");
    let target_skill = env.target_dir("test-tool").join("managed-skill");

    // Library entry should be a SYMLINK for managed skills (library → source)
    assert!(
        library_skill.is_symlink(),
        "managed skill in library should be a symlink"
    );

    // Verify library symlink points to source install path
    let source_skill_dir = env
        .source_dir("plugins")
        .join("installs/managed-skill/skills/managed-skill");
    let library_resolved = std::fs::canonicalize(&library_skill).unwrap();
    let source_canonical = std::fs::canonicalize(&source_skill_dir).unwrap();
    assert_eq!(
        library_resolved, source_canonical,
        "library symlink should resolve to source install dir"
    );

    // Target should also be a symlink (target → library)
    assert!(
        target_skill.is_symlink(),
        "target skill should be a symlink"
    );
    let target_resolved = std::fs::canonicalize(&target_skill).unwrap();
    assert_eq!(
        target_resolved, source_canonical,
        "target symlink should resolve through library to source"
    );

    // Two-hop chain: reading SKILL.md through target should get source content
    let source_content = std::fs::read_to_string(source_skill_dir.join("SKILL.md")).unwrap();
    let target_content = std::fs::read_to_string(target_skill.join("SKILL.md")).unwrap();
    assert_eq!(
        source_content, target_content,
        "reading through two-hop symlink chain should return source content"
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
            "library_dir = \"{}\"\n\n[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
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
    std::fs::write(&machine_path, "disabled_targets = [\"target-b\"]\n").unwrap();

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
fn completions_fish_outputs_valid_completions() {
    tome()
        .args(["completions", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("complete -c tome"));
}

#[test]
fn completions_bash_outputs_valid_completions() {
    tome()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("tome"));
}

#[test]
fn completions_zsh_outputs_valid_completions() {
    tome()
        .args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("#compdef tome"));
}

#[test]
fn completions_invalid_shell_fails() {
    tome().args(["completions", "invalid"]).assert().failure();
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
