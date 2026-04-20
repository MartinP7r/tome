//! Interactive `tome init` setup wizard using dialoguer.
//!
//! Auto-discovers known directory locations and assigns roles from a merged
//! `KNOWN_DIRECTORIES` registry — replacing the former separate KNOWN_SOURCES
//! and KNOWN_TARGETS arrays.

use anyhow::{Context, Result};
use console::{Term, style};
use dialoguer::{Confirm, Input, MultiSelect, Select};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::config::{
    Config, DirectoryConfig, DirectoryName, DirectoryRole, DirectoryType, default_config_path,
    expand_tilde,
};

// ---------------------------------------------------------------------------
// Known directory registry
// ---------------------------------------------------------------------------

/// A well-known directory that tome can auto-discover on the filesystem.
#[derive(Debug)]
struct KnownDirectory {
    /// Identifier used as the BTreeMap key (e.g. "claude-plugins")
    name: &'static str,
    /// Human-readable label shown in prompts (e.g. "Claude Code Plugins")
    display: &'static str,
    /// Path relative to `$HOME`
    default_path: &'static str,
    /// Discovery / consolidation strategy
    directory_type: DirectoryType,
    /// Default role assigned during auto-discovery
    default_role: DirectoryRole,
}

/// Merged registry of all known directories — replaces the former separate
/// `KNOWN_SOURCES` and `KNOWN_TARGETS` arrays.
///
/// Entries are ordered roughly by popularity / likelihood of being present.
const KNOWN_DIRECTORIES: &[KnownDirectory] = &[
    KnownDirectory {
        name: "claude-plugins",
        display: "Claude Code Plugins",
        default_path: ".claude/plugins",
        directory_type: DirectoryType::ClaudePlugins,
        default_role: DirectoryRole::Managed,
    },
    KnownDirectory {
        name: "claude-skills",
        display: "Claude Code Skills",
        default_path: ".claude/skills",
        directory_type: DirectoryType::Directory,
        default_role: DirectoryRole::Synced,
    },
    KnownDirectory {
        name: "antigravity",
        display: "Antigravity",
        default_path: ".gemini/antigravity/skills",
        directory_type: DirectoryType::Directory,
        default_role: DirectoryRole::Synced,
    },
    KnownDirectory {
        name: "codex",
        display: "Codex",
        default_path: ".codex/skills",
        directory_type: DirectoryType::Directory,
        default_role: DirectoryRole::Synced,
    },
    KnownDirectory {
        name: "codex-agents",
        display: "Codex Agents",
        default_path: ".agents/skills",
        directory_type: DirectoryType::Directory,
        default_role: DirectoryRole::Synced,
    },
    KnownDirectory {
        name: "openclaw",
        display: "OpenClaw",
        default_path: ".openclaw/skills",
        directory_type: DirectoryType::Directory,
        default_role: DirectoryRole::Synced,
    },
    KnownDirectory {
        name: "goose",
        display: "Goose",
        default_path: ".config/goose/skills",
        directory_type: DirectoryType::Directory,
        default_role: DirectoryRole::Synced,
    },
    KnownDirectory {
        name: "gemini-cli",
        display: "Gemini CLI",
        default_path: ".gemini/skills",
        directory_type: DirectoryType::Directory,
        default_role: DirectoryRole::Synced,
    },
    KnownDirectory {
        name: "amp",
        display: "Amp",
        default_path: ".config/amp/skills",
        directory_type: DirectoryType::Directory,
        default_role: DirectoryRole::Synced,
    },
    KnownDirectory {
        name: "opencode",
        display: "OpenCode",
        default_path: ".config/opencode/skills",
        directory_type: DirectoryType::Directory,
        default_role: DirectoryRole::Synced,
    },
    KnownDirectory {
        name: "copilot",
        display: "VS Code Copilot",
        default_path: ".copilot/skills",
        directory_type: DirectoryType::Directory,
        default_role: DirectoryRole::Synced,
    },
];

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run the interactive setup wizard.
///
/// When `no_input` is true, every dialoguer prompt is replaced with its
/// documented default (per Phase 5 D-01): select all auto-discovered known
/// directories, library = `~/.tome/skills`, empty exclusions, no role edits,
/// no custom directories, no git init. Dry-run and save paths behave the same
/// as interactive mode — `no_input` only affects how prompts are resolved.
pub fn run(dry_run: bool, no_input: bool) -> Result<Config> {
    println!();
    println!("{}", style("Welcome to tome setup!").bold().cyan());
    println!("This wizard will help you configure your skill directories.");
    println!();

    println!("{}", style("How it works:").bold());
    println!("  Each directory you configure has a role:");
    println!("    Managed  - read-only, owned by a package manager (e.g. Claude plugins)");
    println!("    Synced   - skills are discovered AND distributed here");
    println!("    Source   - skills are discovered here but not distributed");
    println!("    Target   - skills are distributed here but not discovered");
    println!();
    println!("  Tome copies local skills into a central library for safekeeping.");
    println!("  Managed skills are symlinked instead. Each tool receives symlinks");
    println!("  into the library -- your originals are never touched.");
    println!();

    // Step 1: Auto-discover and select directories
    let mut directories = configure_directories(no_input)?;

    // Discover skills now so step 3 can offer an exclusion picker
    let discovered = {
        let tmp = Config {
            directories: directories.clone(),
            ..Config::default()
        };
        match crate::discover::discover_all(
            &tmp,
            &std::collections::BTreeMap::new(),
            &mut Vec::new(),
        ) {
            Ok(skills) => skills,
            Err(e) => {
                eprintln!("warning: could not discover skills from selected directories: {e}");
                eprintln!("  (exclusions can be added manually to config later)");
                Vec::new()
            }
        }
    };

    // Step 2: Choose library location
    let library_dir = configure_library(no_input)?;

    // Step 3: Exclusions
    let exclude = configure_exclusions(&discovered, no_input)?;

    // Step 4: Summary table
    step_divider("Summary");
    show_directory_summary(&directories);

    // Offer to edit roles (skipped entirely under --no-input per D-01)
    #[allow(clippy::while_immutable_condition)]
    // no_input is a const gate; loop exits on user input via break
    while !no_input {
        let edit = Confirm::new()
            .with_prompt("Would you like to edit any directory's role?")
            .default(false)
            .interact()?;

        if !edit {
            break;
        }

        let editable: Vec<(DirectoryName, DirectoryConfig)> = directories
            .iter()
            .filter(|(_, cfg)| cfg.directory_type != DirectoryType::ClaudePlugins)
            .map(|(n, c)| (n.clone(), c.clone()))
            .collect();

        if editable.is_empty() {
            println!("  No editable directories (ClaudePlugins are always Managed).");
            break;
        }

        let labels: Vec<String> = editable
            .iter()
            .map(|(n, c)| format!("{} ({})", n, c.role().description()))
            .collect();

        let idx = Select::new()
            .with_prompt("Which directory to edit?")
            .items(&labels)
            .interact()?;

        let (name, cfg) = &editable[idx];
        let valid = cfg.directory_type.valid_roles();
        let role_labels: Vec<&str> = valid.iter().map(|r| r.description()).collect();

        let role_idx = Select::new()
            .with_prompt(format!("New role for {name}"))
            .items(&role_labels)
            .default(0)
            .interact()?;

        if let Some(entry) = directories.get_mut(name) {
            entry.role = Some(valid[role_idx].clone());
        }

        show_directory_summary(&directories);
    }

    // Offer to add custom directories (skipped entirely under --no-input per D-01)
    #[allow(clippy::while_immutable_condition)]
    // no_input is a const gate; loop exits on user input via break
    while !no_input {
        let add = Confirm::new()
            .with_prompt("Add a custom directory?")
            .default(false)
            .interact()?;

        if !add {
            break;
        }

        let name: String = Input::new()
            .with_prompt("Directory name (identifier)")
            .interact_text()?;

        let dir_name = DirectoryName::new(name)?;

        let path_str: String = Input::new().with_prompt("Path").interact_text()?;

        let path = crate::paths::collapse_home_path(&expand_tilde(&PathBuf::from(&path_str))?);

        // Type picker (Git not available in wizard since it needs URLs)
        let type_labels = ["directory", "claude-plugins"];
        let type_idx = Select::new()
            .with_prompt("Directory type")
            .items(type_labels)
            .default(0)
            .interact()?;

        let directory_type = match type_idx {
            0 => DirectoryType::Directory,
            _ => DirectoryType::ClaudePlugins,
        };

        // Role picker (filtered by type)
        let valid = directory_type.valid_roles();
        let role = if valid.len() == 1 {
            valid[0].clone()
        } else {
            let role_labels: Vec<&str> = valid.iter().map(|r| r.description()).collect();
            let role_idx = Select::new()
                .with_prompt("Role")
                .items(&role_labels)
                .default(0)
                .interact()?;
            valid[role_idx].clone()
        };

        directories.insert(
            dir_name,
            DirectoryConfig {
                path,
                directory_type,
                role: Some(role),
                branch: None,
                tag: None,
                rev: None,
                subdir: None,
            },
        );

        show_directory_summary(&directories);
    }

    println!();

    let config = assemble_config(directories, library_dir, exclude);

    // Save config
    let config_path = default_config_path()?;
    println!(
        "Config will be saved to: {}",
        style(config_path.display()).cyan()
    );

    if dry_run {
        println!("  (dry run -- not saving)");
        // D-07/D-08/D-09: validate the same way a real save would, but without
        // writing to disk. Use a clone so we can expand tildes without mutating
        // the original Config (which might be returned to the caller).
        let mut expanded = config.clone();
        expanded
            .expand_tildes()
            .context("wizard dry-run: tilde expansion failed")?;
        expanded
            .validate()
            .context("wizard dry-run: configuration is invalid")?;
        let toml_str = toml::to_string_pretty(&expanded)
            .context("wizard dry-run: failed to serialize config")?;
        // Defense-in-depth (D-03): reparse to confirm round-trip integrity.
        let _: Config =
            toml::from_str(&toml_str).context("wizard dry-run: generated TOML did not reparse")?;
        println!();
        println!("{}", style("Generated config:").bold());
        println!("{}", toml_str);
    } else if no_input
        || Confirm::new()
            .with_prompt("Save configuration?")
            .default(true)
            .interact()?
    {
        // D-01/D-03/D-07/D-08: expand → validate → round-trip → save.
        // On any failure, return Err — no retry loop (D-08/D-09).
        config
            .save_checked(&config_path)
            .context("wizard save aborted: configuration is invalid")?;
        println!("{} Config saved!", style("done").green());

        // Offer to git-init the tome home directory for backup tracking
        let tome_home = config_path
            .parent()
            .expect("config path should have a parent");
        if !tome_home.join(".git").exists() {
            if no_input {
                // Surface the skipped step so CI/script users aren't surprised
                // when `tome backup list` later reports "not a git repo".
                eprintln!(
                    "{} skipped git-init for backup tracking (--no-input). Run {} to enable.",
                    style("note:").cyan(),
                    style("tome backup init").bold(),
                );
            } else {
                let do_init = Confirm::new()
                    .with_prompt("Initialize a git repo for backup tracking?")
                    .default(false)
                    .interact()?;
                if do_init {
                    crate::backup::init(tome_home, false)
                        .unwrap_or_else(|e| eprintln!("warning: backup init failed: {e}"));
                }
            }
        }
    }

    Ok(config)
}

// ---------------------------------------------------------------------------
// Pure config assembly (WHARD-04 — unit-testable without dialoguer)
// ---------------------------------------------------------------------------

/// Assemble the final `Config` from wizard-produced inputs.
///
/// Pure function: no dialoguer, no filesystem, no env access. Called once at
/// the end of `run()` and driven directly by unit tests (see `wizard.rs::tests`).
///
/// Inputs:
/// - `directories`: map of selected directories (auto-discovered + custom)
/// - `library_dir`: library location (tilde-shaped or absolute; not expanded here)
/// - `exclude`: skill names to exclude
pub(crate) fn assemble_config(
    directories: BTreeMap<DirectoryName, DirectoryConfig>,
    library_dir: PathBuf,
    exclude: std::collections::BTreeSet<crate::discover::SkillName>,
) -> Config {
    Config {
        library_dir,
        exclude,
        directories,
        ..Config::default()
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn step_divider(label: &str) {
    println!(
        "{}",
        style(format!("-- {label} ----------------------------------")).dim()
    );
}

fn show_directory_summary(directories: &BTreeMap<DirectoryName, DirectoryConfig>) {
    if directories.is_empty() {
        println!("  (no directories configured)");
        return;
    }
    // Header
    println!(
        "  {:<20} {:<35} {:<16} {}",
        style("Name").bold(),
        style("Path").bold(),
        style("Type").bold(),
        style("Role").bold(),
    );
    for (name, cfg) in directories {
        println!(
            "  {:<20} {:<35} {:<16} {}",
            name,
            cfg.path.display(),
            cfg.directory_type,
            cfg.role().description(),
        );
    }
    println!();
}

fn configure_directories(no_input: bool) -> Result<BTreeMap<DirectoryName, DirectoryConfig>> {
    step_divider("Step 1: Directories");

    let found = find_known_directories()?;
    let mut directories = BTreeMap::new();

    if !found.is_empty() {
        let labels: Vec<String> = found
            .iter()
            .map(|(kd, _path)| {
                format!(
                    "{} (~/{}) [{}]",
                    kd.display,
                    kd.default_path,
                    kd.default_role.description()
                )
            })
            .collect();

        let selections: Vec<usize> = if no_input {
            // D-01: include all auto-discovered directories.
            (0..found.len()).collect()
        } else {
            MultiSelect::new()
                .with_prompt(
                    "Found these directories -- select which to include\n  (space to toggle, enter to confirm)",
                )
                .items(&labels)
                .defaults(&vec![true; found.len()])
                .report(false)
                .interact()?
        };

        for &idx in &selections {
            let (kd, _path) = &found[idx];
            let dir_name = DirectoryName::new(kd.name)?;
            directories.insert(
                dir_name,
                DirectoryConfig {
                    path: PathBuf::from("~").join(kd.default_path),
                    directory_type: kd.directory_type.clone(),
                    role: Some(kd.default_role.clone()),
                    branch: None,
                    tag: None,
                    rev: None,
                    subdir: None,
                },
            );
        }

        println!(
            "  {} {} directory(ies) selected",
            style("v").green(),
            selections.len()
        );
    }

    println!();
    Ok(directories)
}

fn configure_library(no_input: bool) -> Result<PathBuf> {
    step_divider("Step 2: Library location");

    let default = PathBuf::from("~/.tome/skills");

    let options = vec![
        format!("{} (default)", default.display()),
        "Custom path...".to_string(),
    ];

    let path = if no_input {
        // D-01: default library = ~/.tome/skills
        default
    } else {
        let selection = Select::new()
            .with_prompt("Where should the skill library live?")
            .items(&options)
            .default(0)
            .interact()?;

        if selection == 0 {
            default
        } else {
            let custom: String = Input::new().with_prompt("Library path").interact_text()?;
            crate::paths::collapse_home_path(&expand_tilde(&PathBuf::from(custom))?)
        }
    };

    println!();
    Ok(path)
}

fn configure_exclusions(
    skills: &[crate::discover::DiscoveredSkill],
    no_input: bool,
) -> Result<std::collections::BTreeSet<crate::discover::SkillName>> {
    step_divider("Step 3: Exclusions");

    if skills.is_empty() {
        println!("  (no skills discovered yet -- exclusions can be added manually to config)");
        println!();
        return Ok(std::collections::BTreeSet::new());
    }

    let labels: Vec<String> = skills.iter().map(|s| s.name.to_string()).collect();
    let selections: Vec<usize> = if no_input {
        // D-01: empty exclusions.
        Vec::new()
    } else {
        // Cap visible rows to terminal height minus some overhead for prompt/chrome
        let max_rows = Term::stderr().size().0.saturating_sub(6).max(5) as usize;
        MultiSelect::new()
            .with_prompt("Select skills to exclude (space to toggle, enter to confirm)")
            .items(&labels)
            .defaults(&vec![false; labels.len()])
            .max_length(max_rows)
            .interact()?
    };

    let exclude = selections
        .iter()
        .filter_map(
            |&i| match crate::discover::SkillName::new(labels[i].clone()) {
                Ok(name) => Some(name),
                Err(e) => {
                    eprintln!("warning: could not parse skill name '{}': {e}", labels[i]);
                    None
                }
            },
        )
        .collect();
    println!();
    Ok(exclude)
}

/// Scan well-known locations for existing directories.
fn find_known_directories() -> Result<Vec<(&'static KnownDirectory, PathBuf)>> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    find_known_directories_in(&home)
}

/// Scan well-known locations relative to `home` for existing directories.
///
/// Uses `std::fs::metadata()` instead of `path.is_dir()` so that permission
/// errors surface as warnings rather than being silently swallowed.
fn find_known_directories_in(home: &Path) -> Result<Vec<(&'static KnownDirectory, PathBuf)>> {
    let mut found = Vec::new();

    for kd in KNOWN_DIRECTORIES {
        let abs_path = home.join(kd.default_path);
        match std::fs::metadata(&abs_path) {
            Ok(meta) if meta.is_dir() => {
                found.push((kd, abs_path));
            }
            Ok(_) => {} // exists but not a directory -- skip
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {} // expected -- skip
            Err(e) => {
                eprintln!("warning: could not check {}: {}", abs_path.display(), e);
            }
        }
    }

    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn known_directories_has_no_duplicate_names() {
        let mut names: Vec<&str> = KNOWN_DIRECTORIES.iter().map(|kd| kd.name).collect();
        let original_len = names.len();
        names.sort();
        names.dedup();
        assert_eq!(
            names.len(),
            original_len,
            "KNOWN_DIRECTORIES contains duplicate names"
        );
    }

    #[test]
    fn known_directories_all_have_valid_names() {
        for kd in KNOWN_DIRECTORIES {
            DirectoryName::new(kd.name)
                .unwrap_or_else(|e| panic!("invalid directory name '{}': {e}", kd.name));
        }
    }

    #[test]
    fn find_known_directories_in_empty_home_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let found = find_known_directories_in(tmp.path()).unwrap();
        assert!(found.is_empty());
    }

    #[test]
    fn find_known_directories_in_discovers_existing_dirs() {
        let tmp = TempDir::new().unwrap();

        // Create one of the known directory paths
        let skills_dir = tmp.path().join(".claude/skills");
        std::fs::create_dir_all(&skills_dir).unwrap();

        let found = find_known_directories_in(tmp.path()).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0.name, "claude-skills");
        assert_eq!(found[0].1, skills_dir);
    }

    #[test]
    fn find_known_directories_in_skips_files_with_same_name() {
        let tmp = TempDir::new().unwrap();

        // Create a file (not a directory) at a known path
        let claude_dir = tmp.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(claude_dir.join("skills"), "not a directory").unwrap();

        let found = find_known_directories_in(tmp.path()).unwrap();
        assert!(
            found.is_empty(),
            "expected no directories when path is a file, got: {found:?}"
        );
    }

    #[test]
    fn claude_plugins_always_managed() {
        let entry = KNOWN_DIRECTORIES
            .iter()
            .find(|kd| kd.name == "claude-plugins")
            .expect("claude-plugins entry must exist");
        assert_eq!(entry.directory_type, DirectoryType::ClaudePlugins);
        assert_eq!(entry.default_role, DirectoryRole::Managed);
        // ClaudePlugins type only allows Managed role
        assert_eq!(
            entry.directory_type.valid_roles(),
            vec![DirectoryRole::Managed]
        );
    }

    #[test]
    fn known_directories_default_role_matches_type() {
        // For every entry, the registry's `default_role` must equal what
        // DirectoryType::default_role() returns for that entry's type.
        // Prevents silent drift when new entries are added or enum semantics change.
        for kd in KNOWN_DIRECTORIES {
            assert_eq!(
                kd.default_role,
                kd.directory_type.default_role(),
                "entry '{}': default_role {:?} disagrees with DirectoryType::default_role() for type {:?}",
                kd.name,
                kd.default_role,
                kd.directory_type,
            );
        }
    }

    #[test]
    fn known_directories_default_role_is_in_valid_roles() {
        // Every KNOWN_DIRECTORIES default_role must be accepted by its type's valid_roles().
        // If this fails, `tome init` would produce a Config that Config::validate() rejects.
        for kd in KNOWN_DIRECTORIES {
            let valid = kd.directory_type.valid_roles();
            assert!(
                valid.contains(&kd.default_role),
                "entry '{}': default_role {:?} not in valid_roles for type {:?} ({:?})",
                kd.name,
                kd.default_role,
                kd.directory_type,
                valid,
            );
        }
    }

    #[test]
    fn find_known_directories_in_discovers_every_registry_entry() {
        // Seed HOME with one instance of every registry entry's default_path,
        // then assert find_known_directories_in returns exactly KNOWN_DIRECTORIES.len()
        // results and each expected name appears exactly once.
        let tmp = TempDir::new().unwrap();
        for kd in KNOWN_DIRECTORIES {
            std::fs::create_dir_all(tmp.path().join(kd.default_path)).unwrap();
        }

        let found = find_known_directories_in(tmp.path()).unwrap();
        assert_eq!(
            found.len(),
            KNOWN_DIRECTORIES.len(),
            "expected {} entries, got {}: {:?}",
            KNOWN_DIRECTORIES.len(),
            found.len(),
            found.iter().map(|(kd, _)| kd.name).collect::<Vec<_>>(),
        );

        let mut found_names: Vec<&str> = found.iter().map(|(kd, _)| kd.name).collect();
        found_names.sort();
        let mut expected_names: Vec<&str> = KNOWN_DIRECTORIES.iter().map(|kd| kd.name).collect();
        expected_names.sort();
        assert_eq!(found_names, expected_names);

        // Returned PathBufs are absolute and point under tmp.path().
        for (kd, path) in &found {
            assert!(
                path.is_absolute(),
                "entry '{}' path not absolute: {:?}",
                kd.name,
                path
            );
            assert!(
                path.starts_with(tmp.path()),
                "entry '{}' path {:?} not inside TempDir {:?}",
                kd.name,
                path,
                tmp.path(),
            );
        }
    }

    #[test]
    fn find_known_directories_in_discovers_multiple_entries() {
        // Seed two known paths and assert exactly those two come back.
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".claude/skills")).unwrap();
        std::fs::create_dir_all(tmp.path().join(".codex/skills")).unwrap();

        let found = find_known_directories_in(tmp.path()).unwrap();
        let names: std::collections::BTreeSet<&str> = found.iter().map(|(kd, _)| kd.name).collect();

        assert_eq!(
            found.len(),
            2,
            "expected 2 entries, got {}: {:?}",
            found.len(),
            names
        );
        assert!(
            names.contains("claude-skills"),
            "missing claude-skills: {:?}",
            names
        );
        assert!(names.contains("codex"), "missing codex: {:?}", names);
    }

    #[test]
    fn find_known_directories_in_mixed_dir_and_file() {
        // .claude/skills is a valid directory; .codex/skills exists as a file, not a dir.
        // Expect exactly one result: the real directory, with the file-path silently skipped.
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".claude/skills")).unwrap();
        std::fs::create_dir_all(tmp.path().join(".codex")).unwrap();
        std::fs::write(tmp.path().join(".codex/skills"), "i am a file").unwrap();

        let found = find_known_directories_in(tmp.path()).unwrap();
        assert_eq!(
            found.len(),
            1,
            "expected 1 entry, got {}: {:?}",
            found.len(),
            found.iter().map(|(kd, _)| kd.name).collect::<Vec<_>>(),
        );
        assert_eq!(found[0].0.name, "claude-skills");
    }

    // --- assemble_config tests (WHARD-04) ---

    fn test_dir(path: &str, kind: DirectoryType, role: DirectoryRole) -> DirectoryConfig {
        DirectoryConfig {
            path: PathBuf::from(path),
            directory_type: kind,
            role: Some(role),
            branch: None,
            tag: None,
            rev: None,
            subdir: None,
        }
    }

    #[test]
    fn assemble_config_empty_inputs_produces_empty_config() {
        let config = assemble_config(
            BTreeMap::new(),
            PathBuf::from("~/.tome/skills"),
            std::collections::BTreeSet::new(),
        );
        assert!(config.directories.is_empty());
        assert!(config.exclude.is_empty());
        assert_eq!(config.library_dir, PathBuf::from("~/.tome/skills"));
    }

    #[test]
    fn assemble_config_single_entry_is_preserved() {
        let mut dirs = BTreeMap::new();
        dirs.insert(
            DirectoryName::new("claude-skills").unwrap(),
            test_dir(
                "~/.claude/skills",
                DirectoryType::Directory,
                DirectoryRole::Synced,
            ),
        );

        let config = assemble_config(
            dirs,
            PathBuf::from("~/.tome/skills"),
            std::collections::BTreeSet::new(),
        );

        assert_eq!(config.directories.len(), 1);
        let entry = config
            .directories
            .get("claude-skills")
            .expect("claude-skills must be present");
        assert_eq!(entry.path, PathBuf::from("~/.claude/skills"));
        assert_eq!(entry.directory_type, DirectoryType::Directory);
        assert_eq!(entry.role(), DirectoryRole::Synced);
    }

    #[test]
    fn assemble_config_multi_entry_preserves_all() {
        let mut dirs = BTreeMap::new();
        dirs.insert(
            DirectoryName::new("claude-plugins").unwrap(),
            test_dir(
                "~/.claude/plugins",
                DirectoryType::ClaudePlugins,
                DirectoryRole::Managed,
            ),
        );
        dirs.insert(
            DirectoryName::new("claude-skills").unwrap(),
            test_dir(
                "~/.claude/skills",
                DirectoryType::Directory,
                DirectoryRole::Synced,
            ),
        );
        dirs.insert(
            DirectoryName::new("codex").unwrap(),
            test_dir(
                "~/.codex/skills",
                DirectoryType::Directory,
                DirectoryRole::Synced,
            ),
        );

        let config = assemble_config(
            dirs,
            PathBuf::from("~/.tome/skills"),
            std::collections::BTreeSet::new(),
        );

        assert_eq!(config.directories.len(), 3);
        assert!(config.directories.contains_key("claude-plugins"));
        assert!(config.directories.contains_key("claude-skills"));
        assert!(config.directories.contains_key("codex"));
        assert_eq!(
            config
                .directories
                .get("claude-plugins")
                .unwrap()
                .directory_type,
            DirectoryType::ClaudePlugins,
        );
    }

    #[test]
    fn assemble_config_custom_entry_alongside_known() {
        // Custom dir has a non-registry name but a valid identifier.
        let mut dirs = BTreeMap::new();
        dirs.insert(
            DirectoryName::new("claude-skills").unwrap(),
            test_dir(
                "~/.claude/skills",
                DirectoryType::Directory,
                DirectoryRole::Synced,
            ),
        );
        dirs.insert(
            DirectoryName::new("my-custom").unwrap(),
            test_dir(
                "~/work/team-skills",
                DirectoryType::Directory,
                DirectoryRole::Source,
            ),
        );

        let config = assemble_config(
            dirs,
            PathBuf::from("~/.tome/skills"),
            std::collections::BTreeSet::new(),
        );

        assert_eq!(config.directories.len(), 2);
        let custom = config.directories.get("my-custom").unwrap();
        assert_eq!(custom.path, PathBuf::from("~/work/team-skills"));
        assert_eq!(custom.directory_type, DirectoryType::Directory);
        assert_eq!(custom.role(), DirectoryRole::Source);
    }

    #[test]
    fn assemble_config_exclusions_preserved() {
        let mut exclude = std::collections::BTreeSet::new();
        exclude.insert(crate::discover::SkillName::new("skill-a").unwrap());
        exclude.insert(crate::discover::SkillName::new("skill-b").unwrap());

        let config = assemble_config(
            BTreeMap::new(),
            PathBuf::from("~/.tome/skills"),
            exclude.clone(),
        );

        assert_eq!(config.exclude.len(), 2);
        for name in &exclude {
            assert!(
                config.exclude.contains(name),
                "exclude set missing {:?}",
                name,
            );
        }
    }

    #[test]
    fn assemble_config_library_dir_passed_through_verbatim() {
        // assemble_config does NOT expand tildes or collapse home paths — it's a pure
        // plumbing helper. Verify it leaves library_dir byte-identical to input.
        let lib_tilde = PathBuf::from("~/custom/location");
        let config = assemble_config(
            BTreeMap::new(),
            lib_tilde.clone(),
            std::collections::BTreeSet::new(),
        );
        assert_eq!(config.library_dir, lib_tilde);

        let lib_abs = PathBuf::from("/opt/skills");
        let config = assemble_config(
            BTreeMap::new(),
            lib_abs.clone(),
            std::collections::BTreeSet::new(),
        );
        assert_eq!(config.library_dir, lib_abs);
    }
}
