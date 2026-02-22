//! MCP server implementation using rmcp. Exposes `list_skills` and `read_skill` tools over stdio.

use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt, handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters, model::*, schemars, tool, tool_handler, tool_router,
    transport::stdio,
};

use crate::config::Config;
use crate::discover;

#[derive(Debug, Clone)]
pub(crate) struct TomeServer {
    skills: Vec<discover::DiscoveredSkill>,
    tool_router: ToolRouter<Self>,
}

impl TomeServer {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        let skills = discover::discover_all(&config)?;
        Ok(Self {
            skills,
            tool_router: Self::tool_router(),
        })
    }
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct ReadSkillRequest {
    #[schemars(description = "The skill name (directory name) to read")]
    pub name: String,
}

#[tool_router]
impl TomeServer {
    #[tool(description = "List all skills available in the tome library")]
    fn list_skills(&self) -> Result<CallToolResult, McpError> {
        let skills = &self.skills;

        if skills.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                "No skills found. Run `tome init` to configure sources.",
            )]));
        }

        let mut lines = Vec::with_capacity(skills.len() + 1);
        lines.push(format!("{} skill(s) found:\n", skills.len()));
        for skill in skills {
            lines.push(format!(
                "- {} (source: {}, path: {})",
                skill.name,
                skill.source_name,
                skill.path.display()
            ));
        }

        Ok(CallToolResult::success(vec![Content::text(
            lines.join("\n"),
        )]))
    }

    #[tool(description = "Read the SKILL.md content of a skill by name")]
    fn read_skill(
        &self,
        Parameters(ReadSkillRequest { name }): Parameters<ReadSkillRequest>,
    ) -> Result<CallToolResult, McpError> {
        let skill = self.skills.iter().find(|s| s.name.as_str() == name);

        match skill {
            Some(skill) => {
                let skill_md = skill.path.join("SKILL.md");
                let content = std::fs::read_to_string(&skill_md).map_err(|e| {
                    McpError::internal_error(
                        format!("failed to read {}: {e}", skill_md.display()),
                        None,
                    )
                })?;
                Ok(CallToolResult::success(vec![Content::text(content)]))
            }
            None => Ok(CallToolResult::error(vec![Content::text(format!(
                "Skill '{}' not found. Use list_skills to see available skills.",
                name
            ))])),
        }
    }
}

#[tool_handler]
impl ServerHandler for TomeServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Tome MCP server — exposes discovered AI coding skills for reading".into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "tome-mcp".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

/// Start the MCP server on stdio.
pub async fn serve(config: Config) -> anyhow::Result<()> {
    let server = TomeServer::new(config)?;
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Source, SourceType, Targets};
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn test_config(source_path: PathBuf) -> Config {
        Config {
            library_dir: PathBuf::from("/tmp/unused-library"),
            exclude: Vec::new(),
            sources: vec![Source {
                name: "test".into(),
                path: source_path,
                source_type: SourceType::Directory,
            }],
            targets: Targets::default(),
        }
    }

    fn extract_text(result: &CallToolResult) -> String {
        result.content[0]
            .as_text()
            .expect("expected text content")
            .text
            .clone()
    }

    #[test]
    fn list_skills_with_no_sources() {
        let config = Config {
            library_dir: PathBuf::from("/tmp/unused"),
            exclude: Vec::new(),
            sources: Vec::new(),
            targets: Targets::default(),
        };
        let server = TomeServer::new(config).unwrap();
        let result = server.list_skills().unwrap();
        let text = extract_text(&result);
        assert!(text.contains("No skills found"), "unexpected: {text}");
    }

    #[test]
    fn list_skills_returns_skills() {
        let tmp = TempDir::new().unwrap();
        let skill_dir = tmp.path().join("my-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "# My Skill").unwrap();

        let server = TomeServer::new(test_config(tmp.path().to_path_buf())).unwrap();
        let result = server.list_skills().unwrap();
        let text = extract_text(&result);
        assert!(text.contains("my-skill"), "unexpected: {text}");
        assert!(text.contains("1 skill(s) found"), "unexpected: {text}");
    }

    #[test]
    fn read_skill_returns_content() {
        let tmp = TempDir::new().unwrap();
        let skill_dir = tmp.path().join("my-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "# My Skill\nSome content.").unwrap();

        let server = TomeServer::new(test_config(tmp.path().to_path_buf())).unwrap();
        let result = server
            .read_skill(Parameters(ReadSkillRequest {
                name: "my-skill".into(),
            }))
            .unwrap();
        let text = extract_text(&result);
        assert!(text.contains("Some content."), "unexpected: {text}");
    }

    #[test]
    fn read_skill_not_found() {
        let tmp = TempDir::new().unwrap();
        let server = TomeServer::new(test_config(tmp.path().to_path_buf())).unwrap();
        let result = server
            .read_skill(Parameters(ReadSkillRequest {
                name: "nonexistent".into(),
            }))
            .unwrap();
        assert_eq!(result.is_error, Some(true));
        let text = extract_text(&result);
        assert!(text.contains("not found"), "unexpected: {text}");
    }
}
