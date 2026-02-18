use anyhow::Result;
use console::style;
use dialoguer::{Confirm, Input, MultiSelect, Select};
use std::path::PathBuf;

use crate::config::{
    Config, DistributionMethod, Source, SourceType, TargetConfig, Targets, default_config_path,
    expand_tilde,
};

/// Run the interactive setup wizard.
pub fn run(dry_run: bool) -> Result<Config> {
    println!();
    println!("{}", style("Welcome to skync setup!").bold().cyan());
    println!("This wizard will help you configure skill sources and targets.");
    println!();

    // Step 1: Discover and select sources
    let sources = configure_sources()?;

    // Step 2: Choose library location
    let library_dir = configure_library()?;

    // Step 3: Configure targets
    let targets = configure_targets()?;

    // Step 4: Exclusions
    let exclude = configure_exclusions()?;

    let config = Config {
        library_dir,
        exclude,
        sources,
        targets,
    };

    // Step 5: Save config
    let config_path = default_config_path();
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

    let known_sources = find_known_sources();
    let mut sources = Vec::new();

    if !known_sources.is_empty() {
        println!("Found skills in these locations:");
        let labels: Vec<String> = known_sources
            .iter()
            .map(|s| format!("{} ({})", s.path.display(), s.source_type))
            .collect();

        let selections = MultiSelect::new()
            .with_prompt("Select sources to include")
            .items(&labels)
            .defaults(&vec![true; known_sources.len()])
            .interact()?;

        for idx in selections {
            sources.push(known_sources[idx].clone());
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
            path: expand_tilde(&PathBuf::from(&custom)),
            source_type: SourceType::Directory,
        });
    }

    println!();
    Ok(sources)
}

fn configure_library() -> Result<PathBuf> {
    println!("{}", style("Step 2: Library location").bold());

    let default = dirs::home_dir()
        .expect("could not determine home directory")
        .join(".local/share/skync/skills");

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
        expand_tilde(&PathBuf::from(custom))
    };

    println!();
    Ok(path)
}

fn configure_targets() -> Result<Targets> {
    println!("{}", style("Step 3: Distribution targets").bold());

    let tools = &["Antigravity", "Codex (via MCP)", "OpenClaw (via MCP)"];
    let selections = MultiSelect::new()
        .with_prompt("Which tools should receive skills?")
        .items(tools)
        .interact()?;

    let mut targets = Targets::default();

    for idx in selections {
        match idx {
            0 => {
                let default_path = dirs::home_dir().unwrap().join(".gemini/antigravity/skills");
                let path: String = Input::new()
                    .with_prompt("Antigravity skills directory")
                    .default(default_path.display().to_string())
                    .interact_text()?;
                targets.antigravity = Some(TargetConfig {
                    enabled: true,
                    method: DistributionMethod::Symlink,
                    skills_dir: Some(expand_tilde(&PathBuf::from(path))),
                    mcp_config: None,
                });
            }
            1 => {
                let default_path = dirs::home_dir().unwrap().join(".codex/.mcp.json");
                let path: String = Input::new()
                    .with_prompt("Codex MCP config path")
                    .default(default_path.display().to_string())
                    .interact_text()?;
                targets.codex = Some(TargetConfig {
                    enabled: true,
                    method: DistributionMethod::Mcp,
                    skills_dir: None,
                    mcp_config: Some(expand_tilde(&PathBuf::from(path))),
                });
            }
            2 => {
                let default_path = dirs::home_dir().unwrap().join(".openclaw/.mcp.json");
                let path: String = Input::new()
                    .with_prompt("OpenClaw MCP config path")
                    .default(default_path.display().to_string())
                    .interact_text()?;
                targets.openclaw = Some(TargetConfig {
                    enabled: true,
                    method: DistributionMethod::Mcp,
                    skills_dir: None,
                    mcp_config: Some(expand_tilde(&PathBuf::from(path))),
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

/// Scan well-known locations for existing skills.
fn find_known_sources() -> Vec<Source> {
    let home = dirs::home_dir().expect("could not determine home directory");
    let mut sources = Vec::new();

    // Claude plugins cache
    let claude_plugins = home.join(".claude/plugins/cache");
    if claude_plugins.is_dir() {
        sources.push(Source {
            name: "claude-plugins".into(),
            path: claude_plugins,
            source_type: SourceType::ClaudePlugins,
        });
    }

    // Claude standalone skills
    let claude_skills = home.join(".claude/skills");
    if claude_skills.is_dir() {
        sources.push(Source {
            name: "claude-skills".into(),
            path: claude_skills,
            source_type: SourceType::Directory,
        });
    }

    // Codex skills
    let codex_skills = home.join(".codex/skills");
    if codex_skills.is_dir() {
        sources.push(Source {
            name: "codex-skills".into(),
            path: codex_skills,
            source_type: SourceType::Directory,
        });
    }

    sources
}
