//! Auto-install missing managed plugins from the lockfile.
//!
//! Compares lockfile entries (desired state) against `installed_plugins.json`
//! (actual state) to find managed plugins that need installing. Runs
//! `claude plugin install <registry_id>` for each approved plugin.

use std::collections::HashSet;
use std::path::Path;

use anyhow::{Context, Result};

use crate::config::{Config, SourceType};
use crate::lockfile::Lockfile;

/// A managed plugin that's in the lockfile but not installed locally.
#[derive(Debug)]
pub(crate) struct MissingPlugin {
    pub registry_id: String,
    pub version: Option<String>,
}

/// Find managed plugins from the lockfile that aren't in `installed_plugins.json`.
pub(crate) fn find_missing(
    lockfile: &Lockfile,
    installed_plugins_path: &Path,
) -> Result<Vec<MissingPlugin>> {
    let installed_ids = parse_installed_registry_ids(installed_plugins_path)?;

    let missing = lockfile
        .skills
        .values()
        .filter_map(|entry| {
            let registry_id = entry.registry_id.as_ref()?;
            if installed_ids.contains(registry_id.as_str()) {
                None
            } else {
                Some(MissingPlugin {
                    registry_id: registry_id.clone(),
                    version: entry.version.clone(),
                })
            }
        })
        .collect();

    Ok(missing)
}

/// Install a plugin via `claude plugin install <registry_id>`.
///
/// Returns `Ok(true)` on success, `Ok(false)` if the `claude` CLI wasn't found.
pub(crate) fn install_plugin(registry_id: &str) -> Result<bool> {
    let output = match std::process::Command::new("claude")
        .args(["plugin", "install", registry_id])
        .output()
    {
        Ok(output) => output,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => {
            return Err(anyhow::anyhow!(e).context(format!(
                "failed to run `claude plugin install {registry_id}`"
            )));
        }
    };

    if output.status.success() {
        Ok(true)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "`claude plugin install {registry_id}` failed: {}",
            stderr.trim()
        );
    }
}

/// Interactive reconciliation: find missing plugins, prompt user, install approved ones.
///
/// Returns the number of plugins installed.
pub(crate) fn reconcile(
    lockfile: &Lockfile,
    installed_plugins_path: &Path,
    dry_run: bool,
    quiet: bool,
    no_input: bool,
) -> Result<usize> {
    let missing = find_missing(lockfile, installed_plugins_path)?;
    if missing.is_empty() {
        return Ok(0);
    }

    // Deduplicate by registry_id (multiple skills can come from one plugin)
    let mut seen = HashSet::new();
    let unique: Vec<&MissingPlugin> = missing
        .iter()
        .filter(|p| seen.insert(p.registry_id.as_str()))
        .collect();

    if unique.is_empty() {
        return Ok(0);
    }

    if !quiet {
        println!(
            "{}",
            console::style("Missing managed plugins (from lockfile):").bold()
        );
        for plugin in &unique {
            let version = plugin
                .version
                .as_deref()
                .map(|v| format!(" (v{v})"))
                .unwrap_or_default();
            println!(
                "  {} {}{}",
                console::style("•").dim(),
                plugin.registry_id,
                console::style(version).dim()
            );
        }
    }

    if dry_run {
        if !quiet {
            println!("Would install {} plugin(s)", unique.len());
        }
        return Ok(unique.len());
    }

    // Prompt for confirmation (TTY only, unless --no-input)
    if !no_input && std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        let confirm = dialoguer::Confirm::new()
            .with_prompt(format!("Install {} plugin(s)?", unique.len()))
            .default(true)
            .interact()?;
        if !confirm {
            return Ok(0);
        }
    } else {
        // Non-interactive: skip installation
        if !quiet {
            eprintln!(
                "info: {} missing plugin(s) detected — run `tome sync` interactively to install",
                unique.len()
            );
        }
        return Ok(0);
    }

    let mut installed_count = 0;
    for plugin in &unique {
        if !quiet {
            print!("  Installing {}... ", plugin.registry_id);
        }
        match install_plugin(&plugin.registry_id) {
            Ok(true) => {
                if !quiet {
                    println!("{}", console::style("ok").green());
                }
                installed_count += 1;
            }
            Ok(false) => {
                if !quiet {
                    println!("{}", console::style("skipped").yellow());
                }
            }
            Err(e) => {
                if !quiet {
                    println!("{}", console::style("failed").red());
                    eprintln!("    {e}");
                }
            }
        }
    }

    Ok(installed_count)
}

/// Find the path to `installed_plugins.json` by scanning config sources.
///
/// Returns the first path found for a `ClaudePlugins` source, or `None`.
pub(crate) fn find_installed_plugins_json(config: &Config) -> Option<std::path::PathBuf> {
    for source in &config.sources {
        if source.source_type != SourceType::ClaudePlugins {
            continue;
        }
        // Same search logic as discover_claude_plugins
        let mut candidates = vec![source.path.join("installed_plugins.json")];
        if let Some(parent) = source.path.parent() {
            candidates.push(parent.join("installed_plugins.json"));
        }
        for candidate in &candidates {
            if candidate.exists() {
                return Some(candidate.clone());
            }
        }
    }
    None
}

/// Parse `installed_plugins.json` and return the set of installed registry IDs.
fn parse_installed_registry_ids(json_path: &Path) -> Result<HashSet<String>> {
    let content = std::fs::read_to_string(json_path)
        .with_context(|| format!("failed to read {}", json_path.display()))?;

    let plugins: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse {}", json_path.display()))?;

    let mut ids = HashSet::new();

    // v2 format: { "version": 2, "plugins": { "name@registry": [...] } }
    if let Some(obj) = plugins.get("plugins").and_then(|v| v.as_object()) {
        for key in obj.keys() {
            ids.insert(key.clone());
        }
    }
    // v1 format: no registry IDs available, can't match

    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_installed_plugins_v2(dir: &Path, plugins: &[&str]) -> std::path::PathBuf {
        let mut obj = serde_json::Map::new();
        for &name in plugins {
            obj.insert(name.to_string(), serde_json::json!([]));
        }
        let json = serde_json::json!({ "version": 2, "plugins": obj });
        let path = dir.join("installed_plugins.json");
        std::fs::write(&path, serde_json::to_string_pretty(&json).unwrap()).unwrap();
        path
    }

    fn make_lockfile(entries: &[(&str, Option<&str>)]) -> Lockfile {
        use crate::discover::SkillName;
        use crate::lockfile::LockEntry;
        use crate::validation::ContentHash;

        let mut skills = std::collections::BTreeMap::new();
        for (name, registry_id) in entries {
            let skill_name = SkillName::new(*name).unwrap();
            skills.insert(
                skill_name,
                LockEntry {
                    source_name: "test".to_string(),
                    content_hash: ContentHash::new("a".repeat(64)).unwrap(),
                    registry_id: registry_id.map(|s| s.to_string()),
                    version: registry_id.map(|_| "1.0.0".to_string()),
                    git_commit_sha: None,
                },
            );
        }
        Lockfile { version: 1, skills }
    }

    #[test]
    fn find_missing_detects_absent_plugin() {
        let tmp = TempDir::new().unwrap();
        let json = write_installed_plugins_v2(tmp.path(), &["other-plugin@npm"]);
        let lockfile = make_lockfile(&[("my-skill", Some("my-plugin@npm")), ("local-skill", None)]);

        let missing = find_missing(&lockfile, &json).unwrap();
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].registry_id, "my-plugin@npm");
    }

    #[test]
    fn find_missing_ignores_local_skills() {
        let tmp = TempDir::new().unwrap();
        let json = write_installed_plugins_v2(tmp.path(), &[]);
        let lockfile = make_lockfile(&[("local-skill", None)]);

        let missing = find_missing(&lockfile, &json).unwrap();
        assert!(missing.is_empty());
    }

    #[test]
    fn find_missing_ignores_installed_plugins() {
        let tmp = TempDir::new().unwrap();
        let json = write_installed_plugins_v2(tmp.path(), &["my-plugin@npm"]);
        let lockfile = make_lockfile(&[("my-skill", Some("my-plugin@npm"))]);

        let missing = find_missing(&lockfile, &json).unwrap();
        assert!(missing.is_empty());
    }

    #[test]
    fn reconcile_empty_lockfile_returns_zero() {
        let tmp = TempDir::new().unwrap();
        let json = write_installed_plugins_v2(tmp.path(), &[]);
        let lockfile = Lockfile {
            version: 1,
            skills: std::collections::BTreeMap::new(),
        };

        let count = reconcile(&lockfile, &json, true, true, false).unwrap();
        assert_eq!(count, 0);
    }
}
