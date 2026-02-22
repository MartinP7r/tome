//! Interactive `tome init` setup wizard using dialoguer. Auto-discovers known source locations.

use anyhow::{Context, Result};
use console::style;
use dialoguer::{Confirm, Input, MultiSelect, Select};
use std::path::{Path, PathBuf};

use crate::config::{
    Config, Source, SourceType, TargetConfig, TargetMethod, Targets, default_config_path,
    expand_tilde,
};

/// Run the interactive setup wizard.
pub fn run(dry_run: bool) -> Result<Config> {
    println!();
    println!("{}", style("Welcome to tome setup!").bold().cyan());
    println!("This wizard will help you configure skill sources and targets.");
    println!();

    println!("{}", style("How it works:").bold());
    println!("  Tome uses symlinks — your original files are never moved or copied.");
    println!("  The library and targets contain links pointing back to where your skills");
    println!("  actually live. Removing tome leaves all your original files untouched.");
    println!();

    // Step 1: Discover and select sources
    let sources = configure_sources()?;

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
    let exclude = configure_exclusions()?;

    let config = Config {
        library_dir,
        exclude,
        sources,
        targets,
    };

    // Step 5: Save config
    let config_path = default_config_path()?;
    println!();
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
    }

    Ok(config)
}

fn configure_sources() -> Result<Vec<Source>> {
    println!("{}", style("Step 1: Skill sources").bold());

    let known_sources = find_known_sources()?;
    let mut sources = Vec::new();

    if !known_sources.is_empty() {
        let labels: Vec<String> = known_sources
            .iter()
            .map(|s| format!("{} ({})", s.path.display(), s.source_type))
            .collect();

        let selections = MultiSelect::new()
            .with_prompt("Found skills in these locations — select sources to include")
            .items(&labels)
            .defaults(&vec![true; known_sources.len()])
            .report(false)
            .interact()?;

        for idx in &selections {
            sources.push(known_sources[*idx].clone());
        }

        println!(
            "  {} {} source(s) selected",
            style("✓").green(),
            selections.len()
        );
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
    println!("{}", style("Step 2: Library location").bold());

    let default = dirs::home_dir()
        .context("could not determine home directory")?
        .join(".local/share/tome/skills");

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

fn configure_targets() -> Result<Targets> {
    println!("{}", style("Step 3: Distribution targets").bold());

    let home = dirs::home_dir().context("could not determine home directory")?;

    let tools = &["Antigravity", "Codex (via MCP)", "OpenClaw (via MCP)"];
    let selections = MultiSelect::new()
        .with_prompt("Which tools should receive skills?")
        .items(tools)
        .interact()?;

    let mut targets = Targets::default();

    for idx in selections {
        match idx {
            0 => {
                let default_path = home.join(".gemini/antigravity/skills");
                let path: String = Input::new()
                    .with_prompt("Antigravity skills directory")
                    .default(default_path.display().to_string())
                    .interact_text()?;
                targets.antigravity = Some(TargetConfig {
                    enabled: true,
                    method: TargetMethod::Symlink {
                        skills_dir: expand_tilde(&PathBuf::from(path))?,
                    },
                });
            }
            1 => {
                let default_path = home.join(".codex/.mcp.json");
                let path: String = Input::new()
                    .with_prompt("Codex MCP config path")
                    .default(default_path.display().to_string())
                    .interact_text()?;
                targets.codex = Some(TargetConfig {
                    enabled: true,
                    method: TargetMethod::Mcp {
                        mcp_config: expand_tilde(&PathBuf::from(path))?,
                    },
                });
            }
            2 => {
                let default_path = home.join(".openclaw/.mcp.json");
                let path: String = Input::new()
                    .with_prompt("OpenClaw MCP config path")
                    .default(default_path.display().to_string())
                    .interact_text()?;
                targets.openclaw = Some(TargetConfig {
                    enabled: true,
                    method: TargetMethod::Mcp {
                        mcp_config: expand_tilde(&PathBuf::from(path))?,
                    },
                });
            }
            _ => {
                eprintln!("warning: unexpected target index {idx}, skipping");
            }
        }
    }

    println!();
    Ok(targets)
}

fn configure_exclusions() -> Result<Vec<String>> {
    println!("{}", style("Step 4: Exclusions").bold());

    let input: String = Input::new()
        .with_prompt("Exclude any skills? (comma-separated names, or Enter for none)")
        .default(String::new())
        .allow_empty(true)
        .interact_text()?;

    let exclude: Vec<String> = input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    println!();
    Ok(exclude)
}

/// Well-known skill locations: (name, relative path from $HOME, source type).
const KNOWN_SOURCES: &[(&str, &str, SourceType)] = &[
    (
        "claude-plugins",
        ".claude/plugins/cache",
        SourceType::ClaudePlugins,
    ),
    ("claude-skills", ".claude/skills", SourceType::Directory),
    ("codex-skills", ".codex/skills", SourceType::Directory),
    (
        "antigravity-skills",
        ".gemini/antigravity/skills",
        SourceType::Directory,
    ),
];

/// Check if any source path matches a symlink target path.
///
/// Returns `(source_name, overlapping_path)` pairs for each conflict.
/// Only compares against `Symlink` targets (not `Mcp`), since MCP config
/// files are JSON configs, not skills directories.
fn find_source_target_overlaps(sources: &[Source], targets: &Targets) -> Vec<(String, PathBuf)> {
    let target_paths: Vec<PathBuf> = targets
        .iter()
        .filter_map(|(_, config)| config.skills_dir().map(|p| p.to_path_buf()))
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

        let targets = Targets {
            antigravity: Some(TargetConfig {
                enabled: true,
                method: TargetMethod::Symlink {
                    skills_dir: PathBuf::from("/home/user/.gemini/antigravity/skills"),
                },
            }),
            codex: None,
            openclaw: None,
        };

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

        let targets = Targets {
            antigravity: Some(TargetConfig {
                enabled: true,
                method: TargetMethod::Symlink {
                    skills_dir: PathBuf::from("/home/user/.gemini/antigravity/skills"),
                },
            }),
            codex: None,
            openclaw: None,
        };

        let overlaps = find_source_target_overlaps(&sources, &targets);
        assert!(overlaps.is_empty());
    }

    #[test]
    fn no_overlap_with_mcp_targets() {
        let sources = vec![Source {
            name: "codex-skills".into(),
            path: PathBuf::from("/home/user/.codex/.mcp.json"),
            source_type: SourceType::Directory,
        }];

        let targets = Targets {
            antigravity: None,
            codex: Some(TargetConfig {
                enabled: true,
                method: TargetMethod::Mcp {
                    mcp_config: PathBuf::from("/home/user/.codex/.mcp.json"),
                },
            }),
            openclaw: None,
        };

        // MCP targets should not be compared — mcp_config is a JSON file, not a skills dir
        let overlaps = find_source_target_overlaps(&sources, &targets);
        assert!(overlaps.is_empty());
    }
}
