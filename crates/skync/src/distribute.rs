use anyhow::{Context, Result};
use std::os::unix::fs as unix_fs;
use std::path::Path;

use crate::config::{DistributionMethod, TargetConfig};
use crate::paths::symlink_points_to;

/// Result of distributing skills to a single target.
#[derive(Debug, Default)]
pub struct DistributeResult {
    pub linked: usize,
    pub unchanged: usize,
    pub target_name: String,
}

/// Distribute skills from the library to a target tool.
pub fn distribute_to_target(
    library_dir: &Path,
    target_name: &str,
    target: &TargetConfig,
    dry_run: bool,
) -> Result<DistributeResult> {
    if !target.enabled {
        return Ok(DistributeResult {
            target_name: target_name.to_string(),
            ..Default::default()
        });
    }

    match target.method {
        DistributionMethod::Symlink => {
            distribute_symlinks(library_dir, target_name, target, dry_run)
        }
        DistributionMethod::Mcp => distribute_mcp(library_dir, target_name, target, dry_run),
    }
}

/// Distribute via directory-level symlinks.
fn distribute_symlinks(
    library_dir: &Path,
    target_name: &str,
    target: &TargetConfig,
    dry_run: bool,
) -> Result<DistributeResult> {
    let skills_dir = target.skills_dir.as_ref().with_context(|| {
        format!(
            "target '{}' uses symlink method but has no skills_dir",
            target_name
        )
    })?;

    if !dry_run {
        std::fs::create_dir_all(skills_dir)
            .with_context(|| format!("failed to create target dir {}", skills_dir.display()))?;
    }

    let mut result = DistributeResult {
        target_name: target_name.to_string(),
        ..Default::default()
    };

    // Read all entries in library (these are symlinks to skill dirs)
    let entries = std::fs::read_dir(library_dir)
        .with_context(|| format!("failed to read library dir {}", library_dir.display()))?;

    for entry in entries {
        let entry =
            entry.with_context(|| format!("failed to read entry in {}", library_dir.display()))?;
        let skill_name = entry.file_name();
        let library_skill_path = entry.path();
        let target_link = skills_dir.join(&skill_name);

        if target_link.is_symlink() {
            if symlink_points_to(&target_link, &library_skill_path) {
                result.unchanged += 1;
                continue;
            }
            // Update stale link
            if !dry_run {
                std::fs::remove_file(&target_link).with_context(|| {
                    format!("failed to remove stale symlink {}", target_link.display())
                })?;
            }
        } else if target_link.exists() {
            eprintln!(
                "warning: {} exists in target and is not a symlink, skipping",
                target_link.display()
            );
            continue;
        }

        if !dry_run {
            unix_fs::symlink(&library_skill_path, &target_link).with_context(|| {
                format!(
                    "failed to symlink {} -> {}",
                    target_link.display(),
                    library_skill_path.display()
                )
            })?;
        }
        result.linked += 1;
    }

    Ok(result)
}

/// Distribute via MCP config (write server entry into .mcp.json).
fn distribute_mcp(
    _library_dir: &Path,
    target_name: &str,
    target: &TargetConfig,
    dry_run: bool,
) -> Result<DistributeResult> {
    let mcp_config_path = target.mcp_config.as_ref().with_context(|| {
        format!(
            "target '{}' uses mcp method but has no mcp_config",
            target_name
        )
    })?;

    let mut result = DistributeResult {
        target_name: target_name.to_string(),
        ..Default::default()
    };

    // Read or create .mcp.json
    let mut mcp_doc: serde_json::Value = if mcp_config_path.exists() {
        let content = std::fs::read_to_string(mcp_config_path)
            .with_context(|| format!("failed to read {}", mcp_config_path.display()))?;
        serde_json::from_str(&content)
            .with_context(|| format!("failed to parse {}", mcp_config_path.display()))?
    } else {
        serde_json::json!({})
    };

    let servers = mcp_doc
        .as_object_mut()
        .context("mcp config is not an object")?
        .entry("mcpServers")
        .or_insert_with(|| serde_json::json!({}));

    // Check if skync entry already exists and is correct
    if let Some(existing) = servers.get("skync")
        && existing.get("command").and_then(|v| v.as_str()) == Some("skync-mcp")
    {
        result.unchanged = 1;
        return Ok(result);
    }

    // Add/update the skync MCP server entry
    servers
        .as_object_mut()
        .context("mcpServers is not a JSON object")?
        .insert(
            "skync".into(),
            serde_json::json!({
                "command": "skync-mcp",
                "args": [],
                "env": {}
            }),
        );

    if !dry_run {
        if let Some(parent) = mcp_config_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create dir {}", parent.display()))?;
        }
        let content = serde_json::to_string_pretty(&mcp_doc)?;
        std::fs::write(mcp_config_path, content)
            .with_context(|| format!("failed to write {}", mcp_config_path.display()))?;
    }

    result.linked = 1;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DistributionMethod;
    use tempfile::TempDir;

    fn setup_library(dir: &Path, skill_names: &[&str]) {
        for name in skill_names {
            let skill_dir = dir.join(name);
            std::fs::create_dir_all(&skill_dir).unwrap();
            std::fs::write(skill_dir.join("SKILL.md"), "# test").unwrap();
        }
    }

    #[test]
    fn distribute_symlinks_creates_links() {
        let library = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();
        setup_library(library.path(), &["skill-a", "skill-b"]);

        let target = TargetConfig {
            enabled: true,
            method: DistributionMethod::Symlink,
            skills_dir: Some(target_dir.path().to_path_buf()),
            mcp_config: None,
        };

        let result = distribute_to_target(library.path(), "test", &target, false).unwrap();
        assert_eq!(result.linked, 2);
        assert!(target_dir.path().join("skill-a").is_symlink());
        assert!(target_dir.path().join("skill-b").is_symlink());
    }

    #[test]
    fn distribute_symlinks_idempotent() {
        let library = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();
        setup_library(library.path(), &["skill-a"]);

        let target = TargetConfig {
            enabled: true,
            method: DistributionMethod::Symlink,
            skills_dir: Some(target_dir.path().to_path_buf()),
            mcp_config: None,
        };

        distribute_to_target(library.path(), "test", &target, false).unwrap();
        let result = distribute_to_target(library.path(), "test", &target, false).unwrap();
        assert_eq!(result.linked, 0);
        assert_eq!(result.unchanged, 1);
    }

    #[test]
    fn distribute_idempotent_with_canonicalized_paths() {
        use std::os::unix::fs as unix_fs;

        let tmp = TempDir::new().unwrap();
        let lib_dir = tmp.path().join("library");
        let target_dir = tmp.path().join("target");
        std::fs::create_dir_all(&lib_dir).unwrap();
        std::fs::create_dir_all(&target_dir).unwrap();

        // Create a library entry
        let skill_src = tmp.path().join("source/skill-a");
        std::fs::create_dir_all(&skill_src).unwrap();
        std::fs::write(skill_src.join("SKILL.md"), "# test").unwrap();
        unix_fs::symlink(&skill_src, lib_dir.join("skill-a")).unwrap();

        // Manually create a relative symlink in target: ../library/skill-a
        unix_fs::symlink(
            std::path::Path::new("../library/skill-a"),
            target_dir.join("skill-a"),
        )
        .unwrap();

        let target = TargetConfig {
            enabled: true,
            method: DistributionMethod::Symlink,
            skills_dir: Some(target_dir.clone()),
            mcp_config: None,
        };

        let result = distribute_to_target(&lib_dir, "test", &target, false).unwrap();
        assert_eq!(
            result.unchanged, 1,
            "relative symlink should be recognized as matching"
        );
        assert_eq!(result.linked, 0);
    }

    #[test]
    fn distribute_disabled_target_is_noop() {
        let library = TempDir::new().unwrap();
        let target = TargetConfig {
            enabled: false,
            method: DistributionMethod::Symlink,
            skills_dir: None,
            mcp_config: None,
        };

        let result = distribute_to_target(library.path(), "test", &target, false).unwrap();
        assert_eq!(result.linked, 0);
    }

    #[test]
    fn distribute_mcp_creates_config() {
        let library = TempDir::new().unwrap();
        let mcp_dir = TempDir::new().unwrap();
        let mcp_path = mcp_dir.path().join(".mcp.json");

        let target = TargetConfig {
            enabled: true,
            method: DistributionMethod::Mcp,
            skills_dir: None,
            mcp_config: Some(mcp_path.clone()),
        };

        let result = distribute_to_target(library.path(), "codex", &target, false).unwrap();
        assert_eq!(result.linked, 1);

        let content = std::fs::read_to_string(&mcp_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed["mcpServers"]["skync"]["command"].as_str() == Some("skync-mcp"));
    }

    #[test]
    fn distribute_mcp_preserves_existing_servers() {
        let library = TempDir::new().unwrap();
        let mcp_dir = TempDir::new().unwrap();
        let mcp_path = mcp_dir.path().join(".mcp.json");

        // Pre-populate with an existing server entry
        let existing = serde_json::json!({
            "mcpServers": {
                "other-server": {
                    "command": "other-cmd",
                    "args": ["--flag"]
                }
            }
        });
        std::fs::write(&mcp_path, serde_json::to_string_pretty(&existing).unwrap()).unwrap();

        let target = TargetConfig {
            enabled: true,
            method: DistributionMethod::Mcp,
            skills_dir: None,
            mcp_config: Some(mcp_path.clone()),
        };

        let result = distribute_to_target(library.path(), "codex", &target, false).unwrap();
        assert_eq!(result.linked, 1);

        // Verify both entries exist
        let content = std::fs::read_to_string(&mcp_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed["mcpServers"]["other-server"]["command"].as_str() == Some("other-cmd"));
        assert!(parsed["mcpServers"]["skync"]["command"].as_str() == Some("skync-mcp"));

        // Run again — should be idempotent, other-server still there
        let result2 = distribute_to_target(library.path(), "codex", &target, false).unwrap();
        assert_eq!(result2.unchanged, 1);
        let content2 = std::fs::read_to_string(&mcp_path).unwrap();
        let parsed2: serde_json::Value = serde_json::from_str(&content2).unwrap();
        assert!(parsed2["mcpServers"]["other-server"]["command"].as_str() == Some("other-cmd"));
    }

    #[test]
    fn distribute_mcp_rejects_non_object_mcp_servers() {
        let library = TempDir::new().unwrap();
        let mcp_dir = TempDir::new().unwrap();
        let mcp_path = mcp_dir.path().join(".mcp.json");

        // mcpServers is a string, not an object
        std::fs::write(&mcp_path, r#"{ "mcpServers": "not-an-object" }"#).unwrap();

        let target = TargetConfig {
            enabled: true,
            method: DistributionMethod::Mcp,
            skills_dir: None,
            mcp_config: Some(mcp_path),
        };

        let result = distribute_to_target(library.path(), "test", &target, false);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("not a JSON object"),
            "unexpected error: {err_msg}"
        );
    }

    #[test]
    fn distribute_symlinks_dry_run_doesnt_create_dir() {
        let library = TempDir::new().unwrap();
        let tmp = TempDir::new().unwrap();
        let nonexistent_target = tmp.path().join("does-not-exist");
        setup_library(library.path(), &["skill-a"]);

        let target = TargetConfig {
            enabled: true,
            method: DistributionMethod::Symlink,
            skills_dir: Some(nonexistent_target.clone()),
            mcp_config: None,
        };

        let result = distribute_to_target(library.path(), "test", &target, true).unwrap();
        assert_eq!(result.linked, 1); // counted but not created
        assert!(!nonexistent_target.exists());
    }

    #[test]
    fn distribute_symlink_errors_without_skills_dir() {
        let library = TempDir::new().unwrap();
        let target = TargetConfig {
            enabled: true,
            method: DistributionMethod::Symlink,
            skills_dir: None,
            mcp_config: None,
        };

        let result = distribute_to_target(library.path(), "test", &target, false);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("has no skills_dir"),
            "unexpected error: {err_msg}"
        );
    }

    #[test]
    fn distribute_mcp_errors_without_mcp_config() {
        let library = TempDir::new().unwrap();
        let target = TargetConfig {
            enabled: true,
            method: DistributionMethod::Mcp,
            skills_dir: None,
            mcp_config: None,
        };

        let result = distribute_to_target(library.path(), "test", &target, false);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("has no mcp_config"),
            "unexpected error: {err_msg}"
        );
    }

    #[test]
    fn distribute_symlinks_skips_non_symlink_collision() {
        let library = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();
        setup_library(library.path(), &["skill-a"]);

        // Pre-create a regular file at the target link path (collision)
        std::fs::write(target_dir.path().join("skill-a"), "not a symlink").unwrap();

        let target = TargetConfig {
            enabled: true,
            method: DistributionMethod::Symlink,
            skills_dir: Some(target_dir.path().to_path_buf()),
            mcp_config: None,
        };

        let result = distribute_to_target(library.path(), "test", &target, false).unwrap();
        assert_eq!(result.linked, 0);
        assert_eq!(result.unchanged, 0);

        // The regular file should be unchanged
        let content = std::fs::read_to_string(target_dir.path().join("skill-a")).unwrap();
        assert_eq!(content, "not a symlink");
    }
}
