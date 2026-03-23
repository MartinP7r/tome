//! Interactive `tome init` setup wizard using dialoguer. Auto-discovers known source locations.

use anyhow::{Context, Result};
use console::{Term, style};
use dialoguer::{Confirm, Input, MultiSelect, Select};
use std::path::{Path, PathBuf};

use std::collections::BTreeMap;

use crate::config::{
    Config, Source, SourceType, TargetConfig, TargetMethod, TargetName, default_config_path,
    expand_tilde,
};

/// Run the interactive setup wizard.
pub fn run(dry_run: bool) -> Result<Config> {
    println!();
    println!("{}", style("Welcome to tome setup!").bold().cyan());
    println!("This wizard will help you configure skill sources and targets.");
    println!();

    println!("{}", style("How it works:").bold());
    println!("  Tome copies your local skills into a central library for safekeeping.");
    println!("  Managed skills (e.g. installed plugins) are symlinked there instead.");
    println!("  Each target tool receives symlinks into the library — your originals");
    println!("  are never touched. Removing tome leaves all source files untouched.");
    println!();

    // Step 1: Discover and select sources
    let sources = configure_sources()?;

    // Discover skills now so step 4 can offer a MultiSelect
    let discovered = {
        let tmp = Config {
            sources: sources.clone(),
            ..Config::default()
        };
        match crate::discover::discover_all(&tmp, &mut Vec::new()) {
            Ok(skills) => skills,
            Err(e) => {
                eprintln!("warning: could not discover skills from selected sources: {e}");
                eprintln!("  (exclusions can be added manually to config later)");
                Vec::new()
            }
        }
    };

    // Step 2: Choose library location
    let library_dir = configure_library()?;

    // Step 3: Configure targets
    let targets = configure_targets()?;

    // Warn if any source path overlaps with a symlink target path
    let overlaps = find_source_target_overlaps(&sources, &targets);
    for (name, path) in &overlaps {
        println!(
            "  {} \"{}\" ({}) is both a source and a target — this may cause circular symlinks",
            style("warning:").yellow().bold(),
            name,
            path.display()
        );
    }
    if !overlaps.is_empty() {
        println!();
    }

    // Step 4: Exclusions
    let exclude = configure_exclusions(&discovered)?;

    let config = Config {
        library_dir,
        exclude,
        sources,
        targets,
    };

    // Summary
    step_divider("Summary");
    println!(
        "  Sources:    {}",
        if config.sources.is_empty() {
            style("none".to_string()).yellow()
        } else {
            style(format!("{}", config.sources.len())).cyan()
        }
    );
    println!(
        "  Library:    {}",
        style(config.library_dir.display()).cyan()
    );
    let target_count = config.targets.len();
    println!(
        "  Targets:    {}",
        if target_count == 0 {
            style("none".to_string()).yellow()
        } else {
            style(format!("{target_count}")).cyan()
        }
    );
    if !config.exclude.is_empty() {
        let names: Vec<_> = config.exclude.iter().map(|n| n.as_str()).collect();
        println!("  Exclusions: {}", style(names.join(", ")).dim());
    }
    println!();

    // Save config
    let config_path = default_config_path()?;
    println!(
        "Config will be saved to: {}",
        style(config_path.display()).cyan()
    );

    if dry_run {
        println!("  (dry run — not saving)");
        let toml_str = toml::to_string_pretty(&config)?;
        println!();
        println!("{}", style("Generated config:").bold());
        println!("{}", toml_str);
    } else if Confirm::new()
        .with_prompt("Save configuration?")
        .default(true)
        .interact()?
    {
        config.save(&config_path)?;
        println!("{} Config saved!", style("done").green());

        // Offer to git-init the library directory for change tracking
        if !config.library_dir.join(".git").exists()
            && Confirm::new()
                .with_prompt("Initialize a git repo in the library directory for change tracking?")
                .default(false)
                .interact()?
        {
            std::fs::create_dir_all(&config.library_dir)?;
            let output = std::process::Command::new("git")
                .args(["init"])
                .current_dir(&config.library_dir)
                .output()
                .context("failed to run git init")?;
            if output.status.success() {
                println!(
                    "  {} Initialized git repo in {}",
                    style("✓").green(),
                    config.library_dir.display()
                );
            } else {
                eprintln!(
                    "warning: git init failed (exit code {})",
                    output.status.code().unwrap_or(-1)
                );
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stderr.trim().is_empty() {
                    eprintln!("  git said: {}", stderr.trim());
                }
            }
        }
    }

    Ok(config)
}

fn step_divider(label: &str) {
    println!(
        "{}",
        style(format!("── {label} ──────────────────────────────")).dim()
    );
}

fn configure_sources() -> Result<Vec<Source>> {
    step_divider("Step 1: Skill sources");

    let known_sources = find_known_sources()?;
    let mut sources = Vec::new();

    if !known_sources.is_empty() {
        let labels: Vec<String> = known_sources
            .iter()
            .map(|s| match s.source_type {
                SourceType::ClaudePlugins => {
                    format!("{} — installed marketplace plugins", s.path.display())
                }
                SourceType::Directory => {
                    format!("{} ({})", s.path.display(), s.source_type)
                }
            })
            .collect();

        let selections = MultiSelect::new()
            .with_prompt(
                "Found skills in these locations — select sources to include\n  (space to toggle, enter to confirm)",
            )
            .items(&labels)
            .defaults(&vec![true; known_sources.len()])
            .report(false)
            .interact()?;

        for idx in &selections {
            sources.push(known_sources[*idx].clone());
        }

        println!(
            "  {} {} source(s) selected:",
            style("✓").green(),
            selections.len()
        );
        for idx in &selections {
            let s = &known_sources[*idx];
            println!("    • {} ({})", s.name, s.path.display());
        }
    }

    // Offer to add custom paths
    loop {
        let custom: String = Input::new()
            .with_prompt("Add another directory? (path or Enter to skip)")
            .default(String::new())
            .allow_empty(true)
            .interact_text()?;

        if custom.is_empty() {
            break;
        }

        let name: String = Input::new()
            .with_prompt("Name for this source")
            .interact_text()?;

        sources.push(Source {
            name,
            path: expand_tilde(&PathBuf::from(&custom))?,
            source_type: SourceType::Directory,
        });
    }

    println!();
    Ok(sources)
}

fn configure_library() -> Result<PathBuf> {
    step_divider("Step 2: Library location");

    let default = dirs::home_dir()
        .context("could not determine home directory")?
        .join(".tome/skills");

    let options = vec![
        format!("{} (default)", default.display()),
        "Custom path...".to_string(),
    ];

    let selection = Select::new()
        .with_prompt("Where should the skill library live?")
        .items(&options)
        .default(0)
        .interact()?;

    let path = if selection == 0 {
        default
    } else {
        let custom: String = Input::new().with_prompt("Library path").interact_text()?;
        expand_tilde(&PathBuf::from(custom))?
    };

    println!();
    Ok(path)
}

/// Well-known distribution targets with sensible defaults.
struct KnownTarget {
    name: &'static str,
    display: &'static str,
    /// Path relative to $HOME
    default_path: &'static str,
    path_prompt: &'static str,
}

const KNOWN_TARGETS: &[KnownTarget] = &[
    KnownTarget {
        name: "claude",
        display: "Claude Code",
        default_path: ".claude/skills",
        path_prompt: "Claude Code skills directory",
    },
    KnownTarget {
        name: "antigravity",
        display: "Antigravity",
        default_path: ".gemini/antigravity/skills",
        path_prompt: "Antigravity skills directory",
    },
    KnownTarget {
        name: "codex",
        display: "Codex",
        default_path: ".agents/skills",
        path_prompt: "Codex skills directory",
    },
    KnownTarget {
        name: "openclaw",
        display: "OpenClaw",
        default_path: ".openclaw/skills",
        path_prompt: "OpenClaw skills directory",
    },
    KnownTarget {
        name: "goose",
        display: "Goose",
        default_path: ".config/goose/skills",
        path_prompt: "Goose skills directory",
    },
    KnownTarget {
        name: "gemini-cli",
        display: "Gemini CLI",
        default_path: ".gemini/skills",
        path_prompt: "Gemini CLI skills directory",
    },
    KnownTarget {
        name: "amp",
        display: "Amp",
        default_path: ".config/amp/skills",
        path_prompt: "Amp skills directory",
    },
    KnownTarget {
        name: "opencode",
        display: "OpenCode",
        default_path: ".config/opencode/skills",
        path_prompt: "OpenCode skills directory",
    },
    KnownTarget {
        name: "copilot",
        display: "VS Code Copilot",
        default_path: ".copilot/skills",
        path_prompt: "Copilot skills directory",
    },
];

fn configure_targets() -> Result<BTreeMap<TargetName, TargetConfig>> {
    step_divider("Step 3: Distribution targets");

    let home = dirs::home_dir().context("could not determine home directory")?;

    let labels: Vec<String> = KNOWN_TARGETS
        .iter()
        .map(|t| format!("{} (~/{}/)", t.display, t.default_path))
        .collect();
    let selections = MultiSelect::new()
        .with_prompt("Which tools should receive skills?\n  (space to toggle, enter to confirm)")
        .items(&labels)
        .interact()?;

    println!();
    println!("  Confirm the skills directory for each selected tool.");
    println!("  Press enter to accept the default, or type a custom path.");
    println!();

    let mut targets = BTreeMap::new();

    for idx in selections {
        let known = &KNOWN_TARGETS[idx];
        let default_path = home.join(known.default_path);
        let path: String = Input::new()
            .with_prompt(known.path_prompt)
            .default(default_path.display().to_string())
            .interact_text()?;

        targets.insert(
            TargetName::new(known.name)?,
            TargetConfig {
                enabled: true,
                method: TargetMethod::Symlink {
                    skills_dir: expand_tilde(&PathBuf::from(path))?,
                },
            },
        );
    }

    // Offer custom targets
    loop {
        let name: String = Input::new()
            .with_prompt("Add a custom target? (name or Enter to skip)")
            .default(String::new())
            .allow_empty(true)
            .interact_text()?;

        if name.is_empty() {
            break;
        }

        let path: String = Input::new()
            .with_prompt("Skills directory")
            .interact_text()?;

        targets.insert(
            TargetName::new(name)?,
            TargetConfig {
                enabled: true,
                method: TargetMethod::Symlink {
                    skills_dir: expand_tilde(&PathBuf::from(path))?,
                },
            },
        );
    }

    println!();
    Ok(targets)
}

fn configure_exclusions(
    skills: &[crate::discover::DiscoveredSkill],
) -> Result<std::collections::BTreeSet<crate::discover::SkillName>> {
    step_divider("Step 4: Exclusions");

    if skills.is_empty() {
        println!("  (no skills discovered yet — exclusions can be added manually to config)");
        println!();
        return Ok(std::collections::BTreeSet::new());
    }

    let labels: Vec<String> = skills.iter().map(|s| s.name.to_string()).collect();
    // Cap visible rows to terminal height minus some overhead for prompt/chrome
    let max_rows = Term::stderr().size().0.saturating_sub(6).max(5) as usize;
    let selections = MultiSelect::new()
        .with_prompt("Select skills to exclude (space to toggle, enter to confirm)")
        .items(&labels)
        .defaults(&vec![false; labels.len()])
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

/// Well-known skill locations: (name, relative path from $HOME, source type).
const KNOWN_SOURCES: &[(&str, &str, SourceType)] = &[
    (
        "claude-plugins",
        ".claude/plugins",
        SourceType::ClaudePlugins,
    ),
    ("claude-skills", ".claude/skills", SourceType::Directory),
    ("codex-skills", ".codex/skills", SourceType::Directory),
    (
        "antigravity-skills",
        ".gemini/antigravity/skills",
        SourceType::Directory,
    ),
    (
        "goose-skills",
        ".config/goose/skills",
        SourceType::Directory,
    ),
    ("gemini-cli-skills", ".gemini/skills", SourceType::Directory),
    ("amp-skills", ".config/amp/skills", SourceType::Directory),
    (
        "opencode-skills",
        ".config/opencode/skills",
        SourceType::Directory,
    ),
    ("copilot-skills", ".copilot/skills", SourceType::Directory),
    ("agents-skills", ".agents/skills", SourceType::Directory),
];

/// Check if any source path matches a target's skills directory.
///
/// Returns `(source_name, overlapping_path)` pairs for each conflict.
fn find_source_target_overlaps(
    sources: &[Source],
    targets: &BTreeMap<TargetName, TargetConfig>,
) -> Vec<(String, PathBuf)> {
    let target_paths: Vec<PathBuf> = targets
        .values()
        .map(|config| config.skills_dir().to_path_buf())
        .collect();

    sources
        .iter()
        .filter(|source| {
            target_paths.iter().any(|tp| {
                // Try canonicalize for symlink-resolved comparison, fall back to exact match
                match (source.path.canonicalize(), tp.canonicalize()) {
                    (Ok(src), Ok(tgt)) => src == tgt,
                    _ => source.path == *tp,
                }
            })
        })
        .map(|source| (source.name.clone(), source.path.clone()))
        .collect()
}

/// Scan well-known locations for existing skills.
fn find_known_sources() -> Result<Vec<Source>> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    find_known_sources_in(&home)
}

/// Scan well-known locations relative to `home` for existing skills.
///
/// Uses `std::fs::metadata()` instead of `path.is_dir()` so that permission
/// errors surface as warnings rather than being silently swallowed.
fn find_known_sources_in(home: &Path) -> Result<Vec<Source>> {
    let mut sources = Vec::new();

    for (name, rel_path, source_type) in KNOWN_SOURCES {
        let path = home.join(rel_path);
        match std::fs::metadata(&path) {
            Ok(meta) if meta.is_dir() => {
                sources.push(Source {
                    name: (*name).into(),
                    path,
                    source_type: source_type.clone(),
                });
            }
            Ok(_) => {} // exists but not a directory — skip
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {} // expected — skip
            Err(e) => {
                eprintln!("warning: could not check {}: {}", path.display(), e);
            }
        }
    }

    Ok(sources)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn find_known_sources_in_empty_home_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let sources = find_known_sources_in(tmp.path()).unwrap();
        assert!(sources.is_empty());
    }

    #[test]
    fn find_known_sources_in_discovers_existing_dirs() {
        let tmp = TempDir::new().unwrap();

        // Create one of the known source directories
        let skills_dir = tmp.path().join(".claude/skills");
        std::fs::create_dir_all(&skills_dir).unwrap();

        let sources = find_known_sources_in(tmp.path()).unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].name, "claude-skills");
        assert_eq!(sources[0].path, skills_dir);
        assert_eq!(sources[0].source_type, SourceType::Directory);
    }

    #[test]
    fn find_known_sources_in_skips_files_with_same_name() {
        let tmp = TempDir::new().unwrap();

        // Create a file (not a directory) at a known source path
        let claude_dir = tmp.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(claude_dir.join("skills"), "not a directory").unwrap();

        let sources = find_known_sources_in(tmp.path()).unwrap();
        // The file should be skipped — only directories are included
        assert!(
            sources.is_empty(),
            "expected no sources when path is a file, got: {sources:?}"
        );
    }

    #[test]
    fn detects_source_target_overlap() {
        let sources = vec![Source {
            name: "antigravity-skills".into(),
            path: PathBuf::from("/home/user/.gemini/antigravity/skills"),
            source_type: SourceType::Directory,
        }];

        let targets = BTreeMap::from([(
            TargetName::new("antigravity").unwrap(),
            TargetConfig {
                enabled: true,
                method: TargetMethod::Symlink {
                    skills_dir: PathBuf::from("/home/user/.gemini/antigravity/skills"),
                },
            },
        )]);

        let overlaps = find_source_target_overlaps(&sources, &targets);
        assert_eq!(overlaps.len(), 1);
        assert_eq!(overlaps[0].0, "antigravity-skills");
        assert_eq!(
            overlaps[0].1,
            PathBuf::from("/home/user/.gemini/antigravity/skills")
        );
    }

    #[test]
    fn no_overlap_when_paths_differ() {
        let sources = vec![Source {
            name: "claude-skills".into(),
            path: PathBuf::from("/home/user/.claude/skills"),
            source_type: SourceType::Directory,
        }];

        let targets = BTreeMap::from([(
            TargetName::new("antigravity").unwrap(),
            TargetConfig {
                enabled: true,
                method: TargetMethod::Symlink {
                    skills_dir: PathBuf::from("/home/user/.gemini/antigravity/skills"),
                },
            },
        )]);

        let overlaps = find_source_target_overlaps(&sources, &targets);
        assert!(overlaps.is_empty());
    }

    #[test]
    fn detects_claude_source_target_overlap() {
        let sources = vec![Source {
            name: "claude-skills".into(),
            path: PathBuf::from("/home/user/.claude/skills"),
            source_type: SourceType::Directory,
        }];

        let targets = BTreeMap::from([(
            TargetName::new("claude").unwrap(),
            TargetConfig {
                enabled: true,
                method: TargetMethod::Symlink {
                    skills_dir: PathBuf::from("/home/user/.claude/skills"),
                },
            },
        )]);

        let overlaps = find_source_target_overlaps(&sources, &targets);
        assert_eq!(overlaps.len(), 1);
        assert_eq!(overlaps[0].0, "claude-skills");
    }
}
