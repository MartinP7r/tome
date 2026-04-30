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
use tabled::Table;
use tabled::settings::{Format, Modify, Style, Width, object::Rows, peaker::PriorityMax};
use terminal_size::{Width as TermWidth, terminal_size};

use crate::config::{
    Config, DirectoryConfig, DirectoryName, DirectoryRole, DirectoryType, TomeHomeSource,
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
/// documented default: select all auto-discovered known directories,
/// library = `~/.tome/skills`, empty exclusions, no role edits, no custom
/// directories, save accepted, skip git-init-for-backup (a one-line `note:`
/// is printed to stderr pointing at `tome backup init`). Dry-run and save
/// paths behave the same as interactive mode — `no_input` only affects how
/// prompts are resolved.
pub(crate) fn run(
    dry_run: bool,
    no_input: bool,
    tome_home: &Path,
    tome_home_source: TomeHomeSource,
    prefill: Option<&Config>,
) -> Result<Config> {
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

    // Step 0: Greenfield tome_home prompt (WUX-01)
    // Only runs when:
    // - the user has NOT already indicated a tome_home (flag, env, or XDG),
    // - AND we're not in --no-input mode.
    // If the user picks a custom path, also offer to persist via XDG (WUX-05).
    //
    // Use a distinct local name `chosen_tome_home` (not `tome_home`) so the
    // owned buffer never shadows the `&Path` parameter (avoids clippy::shadow_same).
    let mut chosen_tome_home = tome_home.to_path_buf();
    if matches!(tome_home_source, TomeHomeSource::Default) && !no_input {
        step_divider("Step 0: Tome home location");
        let default_home = dirs::home_dir()
            .context("could not determine home directory")?
            .join(".tome");
        let options = vec![
            format!("{} (default)", crate::paths::collapse_home(&default_home)),
            "Custom path...".to_string(),
        ];
        let selection = Select::new()
            .with_prompt("Where should tome_home live?")
            .items(&options)
            .default(0)
            .interact()?;
        if selection == 1 {
            let custom: String = Input::<String>::new()
                .with_prompt("tome_home path")
                .validate_with(|s: &String| -> std::result::Result<(), String> {
                    let path = PathBuf::from(s);
                    let expanded =
                        expand_tilde(&path).map_err(|_| "could not expand ~".to_string())?;
                    if !expanded.is_absolute() {
                        return Err("must be absolute".to_string());
                    }
                    if expanded.exists() && !expanded.is_dir() {
                        return Err("path exists but is not a directory".to_string());
                    }
                    Ok(())
                })
                .interact_text()?;
            chosen_tome_home = expand_tilde(&PathBuf::from(custom))?;

            // WUX-05: offer to persist custom choice to XDG
            let persist = Confirm::new()
                .with_prompt(
                    "Persist this choice to ~/.config/tome/config.toml?\n  \
                     (otherwise subsequent `tome sync`/`tome status` need TOME_HOME=... or --tome-home=...)",
                )
                .default(true)
                .interact()?;
            if persist {
                crate::config::write_xdg_tome_home(&chosen_tome_home)?;
                println!(
                    "  {} Wrote tome_home to ~/.config/tome/config.toml",
                    style("done").green()
                );
            }
        }
        println!();
    }
    // Downstream helpers take `&Path`; rebind a borrow under the name they expect.
    // `chosen_tome_home` equals the incoming parameter unless Step 0 chose a custom path.
    let tome_home: &Path = &chosen_tome_home;

    // Step 1: Auto-discover and select directories
    let mut directories = configure_directories(no_input, prefill.map(|c| c.directories()))?;

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
    let library_dir = configure_library(no_input, tome_home, prefill.map(|c| c.library_dir()))?;

    // Step 3: Exclusions
    let exclude = configure_exclusions(&discovered, no_input, prefill.map(|c| c.exclude()))?;

    // Step 4: Summary table
    step_divider("Summary");
    show_directory_summary(&directories);

    // Offer to edit roles (skipped entirely under --no-input)
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

    // Offer to add custom directories (skipped entirely under --no-input)
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
                git_ref: None,
                subdir: None,
                override_applied: false,
            },
        );

        show_directory_summary(&directories);
    }

    println!();

    let config = assemble_config(directories, library_dir, exclude);

    // Save config
    // Save path is derived from the resolved tome_home threaded in from lib.rs,
    // not from default_config_path() (which would re-probe TOME_HOME+XDG and may
    // disagree with what sync() uses below). This fixes the latent bug where the
    // wizard could display a save path that differed from the one sync() used.
    let config_path = crate::config::resolve_config_dir(tome_home).join("tome.toml");
    println!(
        "Config will be saved to: {}",
        style(config_path.display()).cyan()
    );

    if dry_run {
        println!("  (dry run -- not saving)");
        // Dry-run validates the same way a real save would, but without writing
        // to disk. Use a clone so we can expand tildes without mutating the
        // original Config (which is returned to the caller).
        let mut expanded = config.clone();
        expanded
            .expand_tildes()
            .context("wizard dry-run: tilde expansion failed")?;
        expanded
            .validate()
            .context("wizard dry-run: configuration is invalid")?;
        let toml_str = toml::to_string_pretty(&expanded)
            .context("wizard dry-run: failed to serialize config")?;
        // Defense in depth: reparse to confirm round-trip integrity before
        // telling the user "this config would save cleanly".
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
        // save_checked runs expand → validate → TOML round-trip → write.
        // On any failure, return Err — no retry loop.
        config
            .save_checked(&config_path)
            .context("wizard save aborted: configuration is invalid")?;
        println!("{} Config saved!", style("done").green());

        // Offer to git-init the tome home directory for backup tracking
        let tome_home = config_path.parent().with_context(|| {
            format!(
                "config path {} has no parent directory; cannot locate tome home for backup git-init",
                config_path.display()
            )
        })?;
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
// Pure config assembly — unit-testable without dialoguer
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

    // Build rows: header + one row per directory entry.
    // Column order per D-02: NAME / TYPE / ROLE / PATH.
    let mut rows: Vec<[String; 4]> = Vec::with_capacity(directories.len() + 1);
    rows.push([
        "NAME".to_string(),
        "TYPE".to_string(),
        "ROLE".to_string(),
        "PATH".to_string(),
    ]);
    for (name, cfg) in directories {
        rows.push([
            name.to_string(),
            cfg.directory_type.to_string(),
            cfg.role().description().to_string(),
            crate::paths::collapse_home(&cfg.path),
        ]);
    }

    // Detect terminal width; fall back to 80 columns on non-TTY / piped output (D-05).
    let term_cols: usize = terminal_size()
        .map(|(TermWidth(w), _)| w as usize)
        .unwrap_or(80);

    // Style::rounded() is a deliberate aesthetic divergence from status.rs's
    // Style::blank(): tome init is a one-shot ceremonial summary (D-01).
    // Width::truncate + PriorityMax::right() shrinks the widest column first —
    // in practice the PATH column, which can hold git-repo clone paths (D-04).
    let table = Table::from_iter(rows)
        .with(Style::rounded())
        .with(Modify::new(Rows::first()).with(Format::content(|s| style(s).bold().to_string())))
        .with(Width::truncate(term_cols).priority(PriorityMax::right()))
        .to_string();
    println!("{table}");
    println!();
}

fn configure_directories(
    no_input: bool,
    prefill: Option<&BTreeMap<DirectoryName, DirectoryConfig>>,
) -> Result<BTreeMap<DirectoryName, DirectoryConfig>> {
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

        // Pre-select entries that are in the prefill map (WUX-02 edit mode).
        // For a fresh wizard run (prefill = None), pre-select everything.
        let defaults: Vec<bool> = found
            .iter()
            .map(|(kd, _)| match prefill {
                Some(map) => DirectoryName::new(kd.name)
                    .ok()
                    .is_some_and(|n| map.contains_key(&n)),
                None => true,
            })
            .collect();

        let selections: Vec<usize> = if no_input {
            // --no-input: include entries that were pre-selected above
            // (everything for fresh, only prefilled entries for edit).
            defaults
                .iter()
                .enumerate()
                .filter_map(|(i, &on)| if on { Some(i) } else { None })
                .collect()
        } else {
            MultiSelect::new()
                .with_prompt(
                    "Found these directories -- select which to include\n  (space to toggle, enter to confirm)",
                )
                .items(&labels)
                .defaults(&defaults)
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
                    git_ref: None,
                    subdir: None,
                    override_applied: false,
                },
            );
        }

        println!(
            "  {} {} directory(ies) selected",
            style("v").green(),
            selections.len()
        );
    }

    // Union with any prefill entries not already present from auto-discovery.
    // This preserves custom directories (not in KNOWN_DIRECTORIES) through
    // an "edit existing" flow — Pitfall 2 from 07-RESEARCH.md.
    if let Some(prefill_map) = prefill {
        for (name, cfg) in prefill_map {
            directories
                .entry(name.clone())
                .or_insert_with(|| cfg.clone());
        }
    }

    println!();
    Ok(directories)
}

fn configure_library(no_input: bool, tome_home: &Path, prefill: Option<&Path>) -> Result<PathBuf> {
    step_divider("Step 2: Library location");

    // Default library = <tome_home>/skills, collapsed to ~/ form when possible
    // for TOML portability (matches existing tilde preservation convention).
    let default = crate::paths::collapse_home_path(&tome_home.join("skills"));

    // Under --no-input: use prefill if given (WUX-02 edit mode), else derived default.
    if no_input {
        return Ok(prefill.map(|p| p.to_path_buf()).unwrap_or(default));
    }

    // Interactive: build options. When prefill is present and differs from the
    // derived default, offer it as a "current" leading choice.
    let mut options: Vec<String> = Vec::new();
    let mut paths: Vec<PathBuf> = Vec::new();
    if let Some(prefilled) = prefill
        && prefilled != default
    {
        options.push(format!(
            "{} (current)",
            crate::paths::collapse_home(prefilled)
        ));
        paths.push(prefilled.to_path_buf());
    }
    options.push(format!("{} (default)", default.display()));
    paths.push(default.clone());
    let custom_idx = options.len();
    options.push("Custom path...".to_string());

    let selection = Select::new()
        .with_prompt("Where should the skill library live?")
        .items(&options)
        .default(0)
        .interact()?;

    let path = if selection == custom_idx {
        let custom: String = Input::new().with_prompt("Library path").interact_text()?;
        crate::paths::collapse_home_path(&expand_tilde(&PathBuf::from(custom))?)
    } else {
        paths[selection].clone()
    };

    println!();
    Ok(path)
}

fn configure_exclusions(
    skills: &[crate::discover::DiscoveredSkill],
    no_input: bool,
    prefill: Option<&std::collections::BTreeSet<crate::discover::SkillName>>,
) -> Result<std::collections::BTreeSet<crate::discover::SkillName>> {
    step_divider("Step 3: Exclusions");

    if skills.is_empty() {
        println!("  (no skills discovered yet -- exclusions can be added manually to config)");
        println!();
        // Under empty skill list the prefill is the only source of exclusions.
        return Ok(prefill.cloned().unwrap_or_default());
    }

    if no_input {
        // --no-input: use prefill if given (WUX-02 edit mode), else empty.
        return Ok(prefill.cloned().unwrap_or_default());
    }

    let labels: Vec<String> = skills.iter().map(|s| s.name.to_string()).collect();
    // Pre-select skills that are already in the prefill set.
    let defaults: Vec<bool> = skills
        .iter()
        .map(|s| prefill.is_some_and(|p| p.contains(&s.name)))
        .collect();

    // Cap visible rows to terminal height minus some overhead for prompt/chrome
    let max_rows = Term::stderr().size().0.saturating_sub(6).max(5) as usize;
    let selections: Vec<usize> = MultiSelect::new()
        .with_prompt("Select skills to exclude (space to toggle, enter to confirm)")
        .items(&labels)
        .defaults(&defaults)
        .max_length(max_rows)
        .interact()?;

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

// ---------------------------------------------------------------------------
// Machine state detection (WUX-03 legacy config + future brownfield dispatch)
// ---------------------------------------------------------------------------

/// The machine state the wizard is running against.
///
/// Probes the filesystem for:
/// - `tome.toml` at the resolved tome_home config dir
///   (via [`crate::config::resolve_config_dir`])
/// - `~/.config/tome/config.toml` for pre-v0.6 `[[sources]]` / `[targets.*]`
///   sections that v0.6+ silently ignores
///
/// Returned by [`detect_machine_state`]. Consumed by the `tome init` dispatcher
/// in `lib.rs`. Plan 04 (WUX-02) will extend the match to act on the
/// Brownfield variants; plan 02 (this plan, WUX-03) only handles the
/// `Legacy` and `BrownfieldWithLegacy` variants.
///
/// Note: `MachineState` cannot derive `Clone` or `PartialEq` because the
/// `Result<Config>` field wraps `anyhow::Error`, which is intentionally
/// non-cloneable and non-equatable. Callers should only pattern-match on
/// variants, not compare states.
#[derive(Debug)]
pub(crate) enum MachineState {
    /// No tome.toml at tome_home; no legacy XDG config with [[sources]]/[targets.*].
    Greenfield,
    /// tome.toml exists at tome_home; no legacy XDG file.
    Brownfield {
        existing_config_path: PathBuf,
        existing_config: Result<Config>,
    },
    /// Legacy pre-v0.6 XDG config detected; no brownfield tome.toml at tome_home.
    Legacy { legacy_path: PathBuf },
    /// Both brownfield AND legacy present. Handled in order: legacy first,
    /// then brownfield (plan 04 will wire up the brownfield branch).
    BrownfieldWithLegacy {
        existing_config_path: PathBuf,
        existing_config: Result<Config>,
        legacy_path: PathBuf,
    },
}

/// Classify the machine state by probing two filesystem locations.
///
/// `home` is passed explicitly (not sourced from `dirs::home_dir()`) so unit
/// tests can isolate via `TempDir`. Production callers pass
/// `dirs::home_dir()?`.
///
/// Resolution:
/// - brownfield: `resolve_config_dir(tome_home).join("tome.toml")` exists
/// - legacy: `home/.config/tome/config.toml` contains `[[sources]]` or
///   `[targets.*]` (parsed, not substring-matched — see
///   [`has_legacy_sections`])
pub(crate) fn detect_machine_state(home: &Path, tome_home: &Path) -> Result<MachineState> {
    let config_path = crate::config::resolve_config_dir(tome_home).join("tome.toml");
    let legacy_path = home.join(".config/tome/config.toml");

    let brownfield = config_path.is_file();
    let legacy = has_legacy_sections(&legacy_path)?;

    Ok(match (brownfield, legacy) {
        (false, None) => MachineState::Greenfield,
        (true, None) => MachineState::Brownfield {
            existing_config: Config::load(&config_path),
            existing_config_path: config_path,
        },
        (false, Some(legacy_path)) => MachineState::Legacy { legacy_path },
        (true, Some(legacy_path)) => MachineState::BrownfieldWithLegacy {
            existing_config: Config::load(&config_path),
            existing_config_path: config_path,
            legacy_path,
        },
    })
}

/// Returns `Some(path)` if the XDG file at `path` exists, parses as TOML, and
/// contains either a top-level `sources` array-of-tables or a `targets` table
/// — the pre-v0.6 schema that v0.6+ silently ignores.
///
/// Returns `None` for:
/// - missing file
/// - malformed TOML (graceful degradation — the user can clean up manually)
/// - v0.6+ shape (e.g. only `tome_home = "..."` and/or `[directories.*]`)
///
/// This function MUST parse (not substring-match) so that comments like
/// `# TODO: re-add [[sources]]` do not false-positive.
fn has_legacy_sections(path: &Path) -> Result<Option<PathBuf>> {
    if !path.is_file() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    // Malformed TOML: treat as "can't tell, not legacy". Do not crash the wizard.
    let Ok(table) = content.parse::<toml::Table>() else {
        return Ok(None);
    };
    let has_sources = table.get("sources").is_some_and(|v| v.is_array());
    let has_targets = table.get("targets").is_some_and(|v| v.is_table());
    Ok((has_sources || has_targets).then(|| path.to_path_buf()))
}

/// Interactive handler for the [`MachineState::Legacy`] and
/// [`MachineState::BrownfieldWithLegacy`] variants.
///
/// Prints a warning that the pre-v0.6 config file is ignored by current tome,
/// then:
/// - If `no_input` is `true`: emits a `note:` line to stderr and returns
///   `Ok(())` — the file is left on disk unchanged.
/// - Otherwise: prompts the user to pick one of 3 actions:
///   1. Leave as-is (warn again next time)
///   2. Move aside (rename to `config.toml.legacy-backup-<unix-ts>`)
///   3. Delete permanently
///
/// The interactive default is action 2 (move-aside) — a user pressing Enter
/// without reading gets the non-destructive backup rather than a no-op.
/// Under `--no-input` the effective default is "leave" per plan spec
/// (WUX-03 must-haves: "Under --no-input, the legacy file is left alone").
pub(crate) fn handle_legacy_cleanup(legacy_path: &Path, no_input: bool) -> Result<()> {
    println!();
    println!(
        "{} Legacy pre-v0.6 config detected: {}",
        style("warning:").yellow(),
        style(legacy_path.display()).cyan()
    );
    println!("  This file contains [[sources]] or [targets.*] sections, which tome v0.6+");
    println!("  does not read. It is silently ignored -- likely not what you want.");

    if no_input {
        eprintln!(
            "{} skipped legacy cleanup (--no-input). Run `tome init` interactively to handle.",
            style("note:").cyan()
        );
        return Ok(());
    }

    let items = [
        "Leave as-is (warn again next time)",
        "Move aside (rename to config.toml.legacy-backup-<timestamp>)",
        "Delete permanently",
    ];
    let selection = Select::new()
        .with_prompt("What do you want to do with it?")
        .items(items)
        .default(1) // move-aside — non-destructive, sorts friendly
        .interact()?;

    match selection {
        0 => {
            println!("  {} Left unchanged.", style("note:").cyan());
        }
        1 => {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .context("system clock is before UNIX epoch")?
                .as_secs();
            let backup_name = format!("config.toml.legacy-backup-{ts}");
            let backup_path = legacy_path
                .parent()
                .context("legacy path has no parent directory")?
                .join(&backup_name);
            std::fs::rename(legacy_path, &backup_path).with_context(|| {
                format!(
                    "failed to rename {} -> {}",
                    legacy_path.display(),
                    backup_path.display()
                )
            })?;
            println!(
                "  {} Moved to: {}",
                style("done").green(),
                style(backup_path.display()).cyan()
            );
        }
        2 => {
            std::fs::remove_file(legacy_path)
                .with_context(|| format!("failed to delete {}", legacy_path.display()))?;
            println!(
                "  {} Deleted: {}",
                style("done").green(),
                style(legacy_path.display()).cyan()
            );
        }
        _ => unreachable!("Select returned out-of-range index"),
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Brownfield decision (WUX-02)
// ---------------------------------------------------------------------------

/// User's choice for how to handle an existing tome.toml at the resolved tome_home.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrownfieldAction {
    /// Exit the wizard cleanly, leaving the existing file untouched. Default for --no-input.
    UseExisting,
    /// Run the wizard with existing values pre-filled. Preserves custom directories.
    Edit,
    /// Back up the existing file to `tome.toml.backup-<unix-ts>` and run wizard as greenfield.
    Reinit,
    /// Exit the wizard without sync; print a short confirmation line to stdout.
    Cancel,
}

/// Display the brownfield summary and prompt the user for an action.
///
/// - `no_input=true` returns `UseExisting` (D-3 locked decision — safest for the
///   dotfiles-sync workflow) when the existing config parses successfully, or
///   `Cancel` when it does not (no silent advance with invalid config in headless
///   mode — the user must investigate).
/// - Otherwise, `Select` with default=0 (UseExisting) for parseable configs, or a
///   reduced `[Reinitialize, Cancel]` menu for unparsable configs.
pub(crate) fn brownfield_decision(
    existing_config_path: &Path,
    existing_config: &Result<Config>,
    no_input: bool,
) -> Result<BrownfieldAction> {
    step_divider("Existing config detected");
    println!(
        "  {} {}",
        style("path:").bold(),
        style(existing_config_path.display()).cyan()
    );
    match existing_config {
        Ok(c) => {
            println!("  directories: {}", c.directories().len());
            println!(
                "  library_dir: {}",
                crate::paths::collapse_home(c.library_dir())
            );
            // Last-modified summary; relative-friendly.
            if let Ok(meta) = std::fs::metadata(existing_config_path)
                && let Ok(mtime) = meta.modified()
                && let Ok(dur) = std::time::SystemTime::now().duration_since(mtime)
            {
                println!("  last modified: {} ago", format_duration(dur));
            }
        }
        Err(e) => {
            println!("  {} {:#}", style("invalid:").red(), e);
            println!("  ('use existing' and 'edit' unavailable while config is invalid)");
        }
    }
    println!();

    // --no-input: D-3 says default = UseExisting. But refuse to default when
    // the config doesn't parse — in headless mode, advancing with an invalid
    // config would be surprising. Return Cancel so the caller exits cleanly
    // and the user investigates.
    if no_input {
        return Ok(if existing_config.is_ok() {
            BrownfieldAction::UseExisting
        } else {
            BrownfieldAction::Cancel
        });
    }

    // Interactive: offer different menus based on whether the config parses.
    if existing_config.is_ok() {
        let items = [
            "Use existing (exit wizard, run `tome sync`)",
            "Edit existing (pre-fill wizard with current values)",
            "Reinitialize (backup + overwrite)",
            "Cancel",
        ];
        let selection = Select::new()
            .with_prompt("What do you want to do?")
            .items(items)
            .default(0)
            .interact()?;
        Ok(match selection {
            0 => BrownfieldAction::UseExisting,
            1 => BrownfieldAction::Edit,
            2 => BrownfieldAction::Reinit,
            3 => BrownfieldAction::Cancel,
            _ => unreachable!("Select returned out-of-range index"),
        })
    } else {
        // No "use existing" or "edit" when parse failed.
        let items = ["Reinitialize (backup + overwrite)", "Cancel"];
        let idx = Select::new()
            .with_prompt("What do you want to do?")
            .items(items)
            .default(0)
            .interact()?;
        Ok(if idx == 0 {
            BrownfieldAction::Reinit
        } else {
            BrownfieldAction::Cancel
        })
    }
}

/// Best-effort human-readable duration for the brownfield last-modified display.
fn format_duration(dur: std::time::Duration) -> String {
    let secs = dur.as_secs();
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86400 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d", secs / 86400)
    }
}

/// Copy `existing_config_path` to `<parent>/tome.toml.backup-<unix-ts>`.
///
/// Uses copy (not rename) so that a Cancel later in the flow leaves the original
/// intact. Returns the backup path so callers can surface it to the user.
pub(crate) fn backup_brownfield_config(existing_config_path: &Path) -> Result<PathBuf> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .context("system clock is before UNIX epoch")?
        .as_secs();
    let backup_name = format!("tome.toml.backup-{ts}");
    let parent = existing_config_path
        .parent()
        .context("existing config path has no parent directory")?;
    let backup_path = parent.join(&backup_name);
    std::fs::copy(existing_config_path, &backup_path).with_context(|| {
        format!(
            "failed to copy {} -> {}",
            existing_config_path.display(),
            backup_path.display()
        )
    })?;
    Ok(backup_path)
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

    // --- assemble_config tests ---

    fn test_dir(path: &str, kind: DirectoryType, role: DirectoryRole) -> DirectoryConfig {
        DirectoryConfig {
            path: PathBuf::from(path),
            directory_type: kind,
            role: Some(role),
            git_ref: None,
            subdir: None,
            override_applied: false,
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

    // --- has_legacy_sections -------------------------------------------------

    #[test]
    fn has_legacy_sections_returns_none_for_missing_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("nope.toml");
        assert_eq!(has_legacy_sections(&path).unwrap(), None);
    }

    #[test]
    fn has_legacy_sections_detects_sources_array() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        std::fs::write(
            &path,
            "[[sources]]\nname = \"old\"\npath = \"/tmp\"\ntype = \"directory\"\n",
        )
        .unwrap();
        assert_eq!(has_legacy_sections(&path).unwrap(), Some(path));
    }

    #[test]
    fn has_legacy_sections_detects_targets_table() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        std::fs::write(&path, "[targets.claude]\npath = \"~/.claude/skills\"\n").unwrap();
        assert_eq!(has_legacy_sections(&path).unwrap(), Some(path));
    }

    #[test]
    fn has_legacy_sections_detects_both_sources_and_targets() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        std::fs::write(
            &path,
            "[[sources]]\nname = \"old\"\npath = \"/tmp\"\ntype = \"directory\"\n\n\
             [targets.claude]\npath = \"~/.claude/skills\"\n",
        )
        .unwrap();
        assert_eq!(has_legacy_sections(&path).unwrap(), Some(path));
    }

    #[test]
    fn has_legacy_sections_ignores_v0_6_only_tome_home() {
        // Critical false-positive protection: v0.6+ users who hand-wrote the XDG
        // file with only the tome_home key must NOT be flagged as legacy.
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        std::fs::write(&path, "tome_home = \"~/dotfiles/tome\"\n").unwrap();
        assert_eq!(has_legacy_sections(&path).unwrap(), None);
    }

    #[test]
    fn has_legacy_sections_ignores_malformed_toml() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        std::fs::write(&path, "this is [[[ not valid toml").unwrap();
        // Graceful no-op — return Ok(None), do not crash.
        assert_eq!(has_legacy_sections(&path).unwrap(), None);
    }

    #[test]
    fn has_legacy_sections_ignores_comment_with_sources_substring() {
        // Comment mentioning [[sources]] must not trigger a false positive —
        // we parse TOML, not grep. Comments are stripped post-parse.
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        std::fs::write(
            &path,
            "# TODO: migrate [[sources]] to [directories.*]\n\
             tome_home = \"~/.tome\"\n",
        )
        .unwrap();
        assert_eq!(has_legacy_sections(&path).unwrap(), None);
    }

    // --- detect_machine_state ------------------------------------------------

    #[test]
    fn detect_machine_state_greenfield() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path();
        let tome_home = home.join(".tome");
        let state = detect_machine_state(home, &tome_home).unwrap();
        assert!(matches!(state, MachineState::Greenfield));
    }

    #[test]
    fn detect_machine_state_brownfield_at_tome_home_root() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path();
        let tome_home = home.join(".tome");
        std::fs::create_dir_all(&tome_home).unwrap();
        // Minimal valid v0.6 config
        std::fs::write(
            tome_home.join("tome.toml"),
            "library_dir = \"~/.tome/skills\"\n[directories]\n",
        )
        .unwrap();
        let state = detect_machine_state(home, &tome_home).unwrap();
        assert!(matches!(state, MachineState::Brownfield { .. }));
    }

    #[test]
    fn detect_machine_state_brownfield_at_dotted_subdir() {
        // `resolve_config_dir` picks `<tome_home>/.tome/` when that subdir has
        // a tome.toml (custom-repo layout).
        let tmp = TempDir::new().unwrap();
        let home = tmp.path();
        let tome_home = home.join("dotfiles/tome");
        let subdir = tome_home.join(".tome");
        std::fs::create_dir_all(&subdir).unwrap();
        std::fs::write(
            subdir.join("tome.toml"),
            "library_dir = \"~/.tome/skills\"\n[directories]\n",
        )
        .unwrap();
        let state = detect_machine_state(home, &tome_home).unwrap();
        assert!(matches!(state, MachineState::Brownfield { .. }));
    }

    #[test]
    fn detect_machine_state_legacy_only() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path();
        let tome_home = home.join(".tome");
        let xdg = home.join(".config/tome/config.toml");
        std::fs::create_dir_all(xdg.parent().unwrap()).unwrap();
        std::fs::write(
            &xdg,
            "[[sources]]\nname = \"x\"\npath = \"/tmp\"\ntype = \"directory\"\n",
        )
        .unwrap();
        let state = detect_machine_state(home, &tome_home).unwrap();
        assert!(matches!(state, MachineState::Legacy { .. }));
    }

    #[test]
    fn detect_machine_state_brownfield_with_legacy() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path();
        let tome_home = home.join(".tome");
        std::fs::create_dir_all(&tome_home).unwrap();
        std::fs::write(
            tome_home.join("tome.toml"),
            "library_dir = \"~/.tome/skills\"\n[directories]\n",
        )
        .unwrap();
        let xdg = home.join(".config/tome/config.toml");
        std::fs::create_dir_all(xdg.parent().unwrap()).unwrap();
        std::fs::write(
            &xdg,
            "[[sources]]\nname = \"x\"\npath = \"/tmp\"\ntype = \"directory\"\n",
        )
        .unwrap();

        let state = detect_machine_state(home, &tome_home).unwrap();
        assert!(matches!(state, MachineState::BrownfieldWithLegacy { .. }));
    }

    #[test]
    fn detect_machine_state_v0_6_only_xdg_is_greenfield() {
        // XDG file exists with only `tome_home = "..."` — NOT legacy.
        let tmp = TempDir::new().unwrap();
        let home = tmp.path();
        let tome_home = home.join(".tome");
        let xdg = home.join(".config/tome/config.toml");
        std::fs::create_dir_all(xdg.parent().unwrap()).unwrap();
        std::fs::write(&xdg, "tome_home = \"~/.tome\"\n").unwrap();
        let state = detect_machine_state(home, &tome_home).unwrap();
        assert!(matches!(state, MachineState::Greenfield));
    }

    // --- handle_legacy_cleanup -----------------------------------------------

    #[test]
    fn handle_legacy_cleanup_no_input_leaves_file() {
        // Under --no-input the handler must leave the file byte-identical and
        // return Ok(()). Interactive branches (move-aside, delete) are NOT
        // tested automatically — dialoguer prompts would hang in headless CI.
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        let original = "[[sources]]\nname = \"x\"\npath = \"/tmp\"\ntype = \"directory\"\n";
        std::fs::write(&path, original).unwrap();

        handle_legacy_cleanup(&path, /* no_input = */ true).unwrap();

        assert!(
            path.is_file(),
            "file should still exist after no_input handler"
        );
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            content, original,
            "content should be byte-identical after no_input handler, got: {content}"
        );
    }

    // -------------------------------------------------------------------
    // WUX-01: configure_library derives default from tome_home
    // -------------------------------------------------------------------

    #[test]
    fn configure_library_no_input_derives_from_tome_home() {
        // Under --no-input with no prefill, configure_library returns
        // <tome_home>/skills (collapsed). With tome_home = /tmp/... (not under
        // HOME), collapse_home_path is a no-op and we get the literal absolute
        // path. This intentionally side-steps HOME expansion; the "collapse to
        // ~/" case is covered by existing integration tests.
        let custom = Path::new("/tmp/zzz-test-custom-tome-home");
        let result = configure_library(true, custom, None).unwrap();
        assert_eq!(
            result,
            PathBuf::from("/tmp/zzz-test-custom-tome-home/skills")
        );
    }

    // -------------------------------------------------------------------
    // WUX-02: brownfield_decision + backup_brownfield_config
    // -------------------------------------------------------------------

    #[test]
    fn brownfield_decision_no_input_returns_use_existing_for_valid_config() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("tome.toml");
        std::fs::write(&path, "library_dir = \"~/.tome/skills\"\n[directories]\n").unwrap();
        let cfg: Result<Config> = Config::load(&path);
        assert!(cfg.is_ok(), "seed config should parse: {:?}", cfg);

        let action = brownfield_decision(&path, &cfg, /* no_input = */ true).unwrap();
        assert_eq!(action, BrownfieldAction::UseExisting);
    }

    #[test]
    fn brownfield_decision_no_input_returns_cancel_for_invalid_config() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("tome.toml");
        std::fs::write(&path, "this is [[[ not valid toml").unwrap();
        let cfg: Result<Config> = Config::load(&path);
        assert!(cfg.is_err(), "seed should fail to parse");

        let action = brownfield_decision(&path, &cfg, /* no_input = */ true).unwrap();
        assert_eq!(action, BrownfieldAction::Cancel);
    }

    // -------------------------------------------------------------------
    // WUX-02: prefill plumbing (Task 2)
    // -------------------------------------------------------------------

    #[test]
    fn configure_directories_preserves_custom_prefill() {
        // A custom directory (not in KNOWN_DIRECTORIES) must survive through
        // edit. Under --no-input with an empty HOME (no auto-discovery hits),
        // the result map should include the prefill's custom entry.
        use crate::config::{DirectoryConfig, DirectoryName, DirectoryRole, DirectoryType};

        let mut prefill_map = std::collections::BTreeMap::new();
        prefill_map.insert(
            DirectoryName::new("my-team").unwrap(),
            DirectoryConfig {
                path: PathBuf::from("/tmp/my-team"),
                directory_type: DirectoryType::Directory,
                role: Some(DirectoryRole::Synced),
                git_ref: None,
                subdir: None,
                override_applied: false,
            },
        );

        // Point find_known_directories at an isolated empty HOME so
        // auto-discovery doesn't match anything real on the dev machine.
        // We can't set HOME directly (edition 2024 unsafe env), but we can
        // call the pure `find_known_directories_in` through configure_directories
        // by using an empty filesystem via a helper. The current configure_directories
        // calls `find_known_directories()` which reads HOME; we accept this
        // and assert on the inclusion of the custom entry (not strict equality).
        let tmp = TempDir::new().unwrap();
        let prev = std::env::var_os("HOME");
        // SAFETY: single-threaded test context; we restore immediately after.
        unsafe {
            std::env::set_var("HOME", tmp.path());
        }

        let result = configure_directories(true, Some(&prefill_map));

        // Restore HOME before any assertion can panic.
        unsafe {
            match prev {
                Some(v) => std::env::set_var("HOME", v),
                None => std::env::remove_var("HOME"),
            }
        }

        let result = result.unwrap();
        assert!(
            result.contains_key(&DirectoryName::new("my-team").unwrap()),
            "custom directory 'my-team' should survive edit. Got: {:?}",
            result.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn configure_library_no_input_uses_prefill() {
        let prefilled = PathBuf::from("/custom/library");
        let tome_home = Path::new("/tmp/any");
        let result = configure_library(true, tome_home, Some(&prefilled)).unwrap();
        assert_eq!(result, prefilled);
    }

    #[test]
    fn configure_library_no_input_uses_derived_default_when_no_prefill() {
        let tome_home = Path::new("/tmp/zzz-not-under-home");
        let result = configure_library(true, tome_home, None).unwrap();
        assert_eq!(result, PathBuf::from("/tmp/zzz-not-under-home/skills"));
    }

    #[test]
    fn configure_exclusions_no_input_uses_prefill() {
        use crate::discover::SkillName;
        let mut prefill = std::collections::BTreeSet::new();
        prefill.insert(SkillName::new("skill-a").unwrap());
        prefill.insert(SkillName::new("skill-b").unwrap());

        let result = configure_exclusions(&[], true, Some(&prefill)).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.contains(&SkillName::new("skill-a").unwrap()));
        assert!(result.contains(&SkillName::new("skill-b").unwrap()));
    }

    #[test]
    fn backup_brownfield_config_copies_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("tome.toml");
        let original_content = "library_dir = \"~/.tome/skills\"\n";
        std::fs::write(&path, original_content).unwrap();

        let backup_path = backup_brownfield_config(&path).unwrap();
        assert!(backup_path.exists(), "backup file should exist");
        assert!(
            path.exists(),
            "original should still exist (copy, not rename)"
        );
        assert_eq!(
            std::fs::read_to_string(&backup_path).unwrap(),
            original_content,
            "backup should have identical content"
        );
        assert!(
            backup_path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("tome.toml.backup-"),
            "backup filename must start with tome.toml.backup-: {:?}",
            backup_path.file_name()
        );
    }
}
