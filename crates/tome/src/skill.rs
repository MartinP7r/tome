//! SKILL.md frontmatter parsing + GUI-facing skill detail aggregate.
//!
//! Two surfaces live here:
//!
//! 1. **`SkillFrontmatter` + `parse`** — the existing CLI/TUI helpers for
//!    extracting YAML frontmatter from a `SKILL.md` blob.
//! 2. **`SkillDetail` + `collect_detail`** — the GUI's right-pane payload
//!    (Phase 26 plan 26-03 / VIEW-03 / D-05). Aggregates the manifest entry,
//!    machine-prefs disabled state, parsed frontmatter (as a specta-gated
//!    `SkillFrontmatterView` projection), and the post-frontmatter markdown
//!    body. Body length capped at 1 MB (RESEARCH Security §"Markdown body
//!    size").

use anyhow::{Context, bail};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Parsed SKILL.md frontmatter fields.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
#[allow(dead_code)]
pub struct SkillFrontmatter {
    pub name: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub compatibility: Option<String>,
    pub metadata: Option<BTreeMap<String, serde_yaml::Value>>,
    pub allowed_tools: Option<String>,
    // Claude Code extensions
    pub user_invocable: Option<bool>,
    pub argument_hint: Option<String>,
    pub context: Option<String>,
    pub agent: Option<String>,
    // Capture unknown/non-standard fields
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

/// Extract frontmatter YAML block from SKILL.md content.
/// Returns (yaml_content, body) or None if no valid frontmatter delimiters.
pub fn extract_frontmatter(content: &str) -> Option<(&str, &str)> {
    let content = content.trim_start();
    if !content.starts_with("---") {
        return None;
    }
    let after_first = &content[3..];
    // Skip the newline after opening ---
    let after_first = after_first.strip_prefix('\n').unwrap_or(after_first);

    // Handle empty frontmatter: closing --- immediately follows opening
    if let Some(rest) = after_first.strip_prefix("---") {
        let body = rest.strip_prefix('\n').unwrap_or(rest);
        return Some(("", body));
    }

    let end = after_first.find("\n---")?;
    let yaml = &after_first[..end];
    let body = &after_first[end + 4..];
    // Strip optional newline after closing ---
    let body = body.strip_prefix('\n').unwrap_or(body);
    Some((yaml.trim(), body))
}

/// Parse SKILL.md content into frontmatter + body.
///
/// # Errors
///
/// Returns an `anyhow::Error` describing the parse failure. Callers can
/// chain `.context(...)` without `.map_err(anyhow::anyhow!)` boilerplate.
pub fn parse(content: &str) -> anyhow::Result<(SkillFrontmatter, String)> {
    match extract_frontmatter(content) {
        Some((yaml, body)) => {
            let fm: SkillFrontmatter =
                serde_yaml::from_str(yaml).context("invalid YAML frontmatter")?;
            Ok((fm, body.to_string()))
        }
        None => bail!("no frontmatter found (expected --- delimiters)"),
    }
}

// ---------------------------------------------------------------------------
// GUI-facing surface (Phase 26 plan 26-03 / VIEW-03)
// ---------------------------------------------------------------------------

/// Hard cap on the markdown body length surfaced to the GUI (RESEARCH
/// Security §"Markdown body size"). A pathological SKILL.md cannot DoS the
/// webview's render path; anything beyond 1 MiB is truncated with a
/// marker line so the user can tell.
//
// `dead_code` allow on the CLI build only — `tome-desktop` consumes the
// constant via `collect_detail` under feature `bindings`. The bare CLI
// build never references it. Keep this attr; SKILL_BODY_MAX_BYTES is
// shared API surface even when bindings are off.
#[allow(dead_code)]
pub const SKILL_BODY_MAX_BYTES: usize = 1_048_576;

/// Specta-friendly projection of [`SkillFrontmatter`] crossing the GUI IPC
/// boundary.
///
/// The CLI-side [`SkillFrontmatter`] type carries unstructured `metadata` and
/// `extra` maps keyed by `String → serde_yaml::Value`. specta's `serde_yaml`
/// support is not enabled in this crate (would add a heavier transitive dep
/// for ad-hoc YAML), and `serde_json::Value` is recursive and can't be
/// inlined by specta's TypeScript exporter. So the GUI ships **JSON-encoded
/// string blobs** for the unstructured fields — the JS side `JSON.parse`s
/// them on demand. The known-typed fields (name / description / license /
/// ...) keep their native Rust types.
///
/// Phase 26 alpha (plan 26-03) doesn't actually render `metadata` / `extra`
/// in the DetailHeader — those fields ride along for plan 26-04 (markdown
/// body) and beyond. The string-encoded shape lands a clean TS schema; the
/// JS side parses values only when a downstream UI surface needs them.
#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
#[allow(dead_code)]
pub struct SkillFrontmatterView {
    pub name: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub compatibility: Option<String>,
    pub allowed_tools: Option<String>,
    /// Claude Code extension — `user-invocable: false` flag.
    pub user_invocable: Option<bool>,
    pub argument_hint: Option<String>,
    pub context: Option<String>,
    pub agent: Option<String>,
    /// Structured `metadata` map (YAML→JSON, then each value serialized as a
    /// JSON string blob — avoids specta's "recursive inline type" panic on
    /// `serde_json::Value`). `None` when the frontmatter has no `metadata`
    /// key. The JS side `JSON.parse`s individual entries on demand.
    pub metadata: Option<BTreeMap<String, String>>,
    /// Unknown / non-standard top-level frontmatter fields. Same
    /// YAML→JSON→string encoding as `metadata`. Empty map when the
    /// frontmatter is fully canonical.
    pub extra: BTreeMap<String, String>,
}

#[allow(dead_code)]
impl SkillFrontmatterView {
    /// Project a [`SkillFrontmatter`] into the GUI-facing view.
    ///
    /// Known-typed fields pass through verbatim. `metadata` / `extra` values
    /// are coerced YAML→JSON, then serialized as JSON strings (see the
    /// struct docs for why). YAML constructs that don't round-trip cleanly
    /// (tags, anchors, non-string keys) collapse into the YAML-Debug
    /// representation as a last-resort string so the UI still gets a value.
    pub fn from_frontmatter(fm: &SkillFrontmatter) -> Self {
        SkillFrontmatterView {
            name: fm.name.clone(),
            description: fm.description.clone(),
            license: fm.license.clone(),
            compatibility: fm.compatibility.clone(),
            allowed_tools: fm.allowed_tools.clone(),
            user_invocable: fm.user_invocable,
            argument_hint: fm.argument_hint.clone(),
            context: fm.context.clone(),
            agent: fm.agent.clone(),
            metadata: fm.metadata.as_ref().map(|m| {
                m.iter()
                    .map(|(k, v)| (k.clone(), yaml_to_json_string(v)))
                    .collect()
            }),
            extra: fm
                .extra
                .iter()
                .map(|(k, v)| (k.clone(), yaml_to_json_string(v)))
                .collect(),
        }
    }
}

/// Coerce a `serde_yaml::Value` into a JSON-encoded string. YAML constructs
/// without a clean JSON equivalent fall back to the YAML-Debug repr wrapped
/// in a JSON string literal.
#[allow(dead_code)]
fn yaml_to_json_string(value: &serde_yaml::Value) -> String {
    let yaml_str = match serde_yaml::to_string(value) {
        Ok(s) => s,
        Err(_) => return "null".to_string(),
    };
    match serde_yaml::from_str::<serde_json::Value>(&yaml_str) {
        Ok(json) => {
            serde_json::to_string(&json).unwrap_or_else(|_| format!("{json:?}"))
        }
        Err(_) => serde_json::to_string(&format!("{value:?}"))
            .unwrap_or_else(|_| "\"<unrepresentable>\"".to_string()),
    }
}

/// GUI-facing aggregate of everything a single skill exposes (Phase 26 plan
/// 26-03 / VIEW-03 / D-05).
///
/// Owned by the right-pane DetailHeader + MarkdownBody in the React UI. The
/// shape is intentionally projection-flat: no nested manifest type leaks
/// across the IPC edge.
#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
#[allow(dead_code)]
pub struct SkillDetail {
    pub name: crate::discover::SkillName,
    /// Resolved source-of-truth path (manifest source for Owned skills,
    /// library-canonical copy for Unowned). Computed by
    /// [`crate::actions::resolve_source_path`].
    pub source_path: PathBuf,
    /// SHA-256 of the source directory at the last sync.
    pub content_hash: crate::validation::ContentHash,
    /// ISO-8601 timestamp of the last sync; the manifest tracks this per
    /// entry. Always populated for entries that exist in the manifest.
    pub last_sync: Option<String>,
    /// Whether the skill's source directory is managed by a package
    /// manager (true) or a local directory (false).
    pub managed: bool,
    /// Whether the skill is currently in the **global** `disabled` set in
    /// `machine.toml` on this machine.
    pub disabled: bool,
    /// Parsed frontmatter (specta-friendly projection).
    pub frontmatter: SkillFrontmatterView,
    /// Post-frontmatter markdown body. Capped at [`SKILL_BODY_MAX_BYTES`]
    /// (1 MiB) with an inline truncation marker beyond that.
    pub body: String,
}

/// Aggregate the right-pane payload for a single skill.
///
/// Reads the library manifest (for the source path / content hash / sync
/// timestamp / managed flag), the on-disk `SKILL.md` (for the frontmatter +
/// body), and per-machine prefs (for the disabled flag). The body is
/// always read from the library-canonical copy at
/// `<library_dir>/<name>/SKILL.md` — that's the v0.10 contract.
///
/// # Errors
///
/// Returns an error if:
/// - the manifest can't be loaded;
/// - no manifest entry exists for `name` (the right-pane caller is expected
///   to have a list-row index, so a missing entry is a user-visible "skill
///   not found" condition);
/// - `<library_dir>/<name>/SKILL.md` doesn't exist or can't be read;
/// - the SKILL.md has malformed YAML frontmatter (rare — the library copy
///   is consolidated from a sync that already linted the source);
/// - the machine prefs can't be loaded.
#[allow(dead_code)]
pub fn collect_detail(
    name: &crate::discover::SkillName,
    config: &crate::config::Config,
    paths: &crate::TomePaths,
) -> anyhow::Result<SkillDetail> {
    use anyhow::Context as _;

    let manifest = crate::manifest::load(paths.config_dir())
        .with_context(|| format!("failed to load manifest while building detail for '{name}'"))?;
    let entry = manifest
        .get(name.as_str())
        .ok_or_else(|| anyhow::anyhow!("skill '{name}' not found in manifest"))?;

    // Resolve the source path through the same code path the GUI's
    // `open_source_folder` / `copy_path` commands use, so all three reads
    // agree (#26-03 D-07).
    let source_path = crate::actions::resolve_source_path(name, config, paths)
        .with_context(|| format!("failed to resolve source path for '{name}'"))?;

    // The library-canonical copy is always at `<library_dir>/<name>/SKILL.md`
    // (v0.10 library-canonical model). Read it directly here — we don't go
    // through `source_path` because Owned managed skills carry a
    // source_path that points at the upstream package-manager-owned dir,
    // and SKILL.md there may not exist if the manager has gated visibility.
    let skill_md = paths.library_dir().join(name.as_str()).join("SKILL.md");
    let raw = std::fs::read_to_string(&skill_md)
        .with_context(|| format!("failed to read {}", skill_md.display()))?;
    let (frontmatter, mut body) = crate::skill::parse(&raw)
        .with_context(|| format!("failed to parse SKILL.md for '{name}'"))?;

    // Cap the body so a pathological file doesn't blow the webview render
    // budget (RESEARCH Security §"Markdown body size"). Use a char-boundary
    // truncate so we don't split a multi-byte UTF-8 codepoint.
    if body.len() > SKILL_BODY_MAX_BYTES {
        let mut cut = SKILL_BODY_MAX_BYTES;
        while cut > 0 && !body.is_char_boundary(cut) {
            cut -= 1;
        }
        body.truncate(cut);
        body.push_str("\n\n[... truncated ...]");
    }

    let machine_path = crate::machine::default_machine_path()
        .context("failed to resolve default machine.toml path")?;
    let prefs = crate::machine::load(&machine_path)
        .with_context(|| format!("failed to load machine prefs from {}", machine_path.display()))?;

    Ok(SkillDetail {
        name: name.clone(),
        source_path,
        content_hash: entry.content_hash.clone(),
        last_sync: Some(entry.synced_at.clone()),
        managed: entry.managed,
        disabled: prefs.is_disabled(name.as_str()),
        frontmatter: SkillFrontmatterView::from_frontmatter(&frontmatter),
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_frontmatter() {
        let content = "---\nname: my-skill\ndescription: A test skill\n---\n# Body";
        let (fm, body) = parse(content).unwrap();
        assert_eq!(fm.name.as_deref(), Some("my-skill"));
        assert_eq!(fm.description.as_deref(), Some("A test skill"));
        assert_eq!(body, "# Body");
    }

    #[test]
    fn parse_with_extra_fields() {
        let content = "---\nname: test\nversion: 1.0\ncategory: tools\n---\nbody";
        let (fm, _) = parse(content).unwrap();
        assert!(fm.extra.contains_key("version"));
        assert!(fm.extra.contains_key("category"));
    }

    #[test]
    fn parse_missing_frontmatter() {
        let content = "# Just a heading\nNo frontmatter here";
        assert!(parse(content).is_err());
    }

    #[test]
    fn parse_malformed_yaml() {
        let content = "---\n: invalid yaml [[\n---\nbody";
        assert!(parse(content).is_err());
    }

    #[test]
    fn parse_empty_frontmatter() {
        let content = "---\n---\nbody";
        let (fm, body) = parse(content).unwrap();
        assert!(fm.name.is_none());
        assert_eq!(body, "body");
    }

    #[test]
    fn parse_claude_code_extensions() {
        let content = "---\nname: test\nuser-invocable: false\ncontext: fork\n---\nbody";
        let (fm, _) = parse(content).unwrap();
        assert_eq!(fm.user_invocable, Some(false));
        assert_eq!(fm.context.as_deref(), Some("fork"));
    }

    #[test]
    fn extract_no_frontmatter_returns_none() {
        assert!(extract_frontmatter("just text").is_none());
    }

    #[test]
    fn extract_unclosed_frontmatter_returns_none() {
        assert!(extract_frontmatter("---\nname: test\n").is_none());
    }

    // -- HARD-01: anyhow::Result migration tests --

    #[test]
    fn parse_missing_frontmatter_error_describes_failure() {
        // Returns an anyhow::Error whose Display preserves the existing message
        // text. Tests `Err(anyhow::Error)` shape rather than `Err(String)`.
        let err = parse("# Just a heading").unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("no frontmatter found"),
            "unexpected error message: {msg}"
        );
    }

    #[test]
    fn parse_invalid_yaml_chains_serde_context() {
        // serde_yaml's failure becomes the underlying cause, with our context
        // wrapper "invalid YAML frontmatter" on top. Display alternate ({:#})
        // renders the chain.
        let err = parse("---\n: invalid yaml [[\n---\nbody").unwrap_err();
        let chained = format!("{err:#}");
        assert!(
            chained.contains("invalid YAML frontmatter"),
            "expected context wrapper in chain: {chained}"
        );
    }

    #[test]
    fn parse_error_can_be_contexted_without_map_err_anyhow() {
        // Caller-side ergonomics: callers can chain `.context(...)` without
        // `.map_err(anyhow::anyhow!)` boilerplate.
        use anyhow::Context;
        let result: anyhow::Result<()> = parse("# no frontmatter")
            .context("while linting fixture")
            .map(|_| ());
        let err = result.unwrap_err();
        let chained = format!("{err:#}");
        assert!(chained.contains("while linting fixture"));
        assert!(chained.contains("no frontmatter"));
    }

    // ---- SkillFrontmatterView projection (Phase 26 plan 26-03) ----

    #[test]
    fn frontmatter_view_round_trips_known_fields() {
        let yaml = "---\nname: my-skill\ndescription: hello\nuser-invocable: false\n---\nbody";
        let (fm, _) = parse(yaml).unwrap();
        let view = SkillFrontmatterView::from_frontmatter(&fm);
        assert_eq!(view.name.as_deref(), Some("my-skill"));
        assert_eq!(view.description.as_deref(), Some("hello"));
        assert_eq!(view.user_invocable, Some(false));
        assert!(view.metadata.is_none());
    }

    #[test]
    fn frontmatter_view_passes_extra_through_as_json_strings() {
        let yaml = "---\nname: x\nversion: \"1.2\"\nrole: assistant\n---\nbody";
        let (fm, _) = parse(yaml).unwrap();
        let view = SkillFrontmatterView::from_frontmatter(&fm);
        // `version` and `role` are non-canonical → land in `extra` as
        // JSON-encoded string blobs.
        let version = view.extra.get("version").expect("version present");
        let version_json: serde_json::Value = serde_json::from_str(version).unwrap();
        assert_eq!(version_json.as_str(), Some("1.2"));
        let role = view.extra.get("role").expect("role present");
        let role_json: serde_json::Value = serde_json::from_str(role).unwrap();
        assert_eq!(role_json.as_str(), Some("assistant"));
    }

    #[test]
    fn frontmatter_view_projects_metadata_map() {
        let yaml = "---\nname: m\nmetadata:\n  tags:\n    - swift\n    - ios\n  level: 3\n---\nbody";
        let (fm, _) = parse(yaml).unwrap();
        let view = SkillFrontmatterView::from_frontmatter(&fm);
        let meta = view.metadata.expect("metadata present");
        let tags = meta.get("tags").expect("tags key");
        let tags_json: serde_json::Value = serde_json::from_str(tags).unwrap();
        let arr = tags_json.as_array().expect("tags is an array");
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0].as_str(), Some("swift"));
        let level = meta.get("level").expect("level key");
        let level_json: serde_json::Value = serde_json::from_str(level).unwrap();
        assert_eq!(level_json.as_i64(), Some(3));
    }

    // ---- collect_detail integration ----

    use crate::TomePaths;
    use crate::config::{Config, DirectoryName};
    use crate::discover::SkillName;
    use crate::manifest::{self, Manifest, SkillEntry};
    use crate::validation::test_hash;
    use std::fs;
    use tempfile::TempDir;

    /// Build a temp `TomePaths` + Config + canonical library layout. Returns
    /// (tempdir, config, paths) — the tempdir is kept alive in the test.
    fn temp_paths_with_library() -> (TempDir, Config, TomePaths) {
        let tmp = TempDir::new().unwrap();
        let tome_home = tmp.path().to_path_buf();
        let library_dir = tome_home.join("skills");
        fs::create_dir_all(&library_dir).unwrap();
        let paths = TomePaths::new(tome_home, library_dir).unwrap();
        let config = Config::default();
        (tmp, config, paths)
    }

    /// Write a SKILL.md at the library-canonical location for `name`.
    fn write_library_skill(paths: &TomePaths, name: &str, content: &str) {
        let dir = paths.library_dir().join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SKILL.md"), content).unwrap();
    }

    #[test]
    fn collect_detail_aggregates_manifest_frontmatter_and_body() {
        let (_tmp, config, paths) = temp_paths_with_library();
        let name = SkillName::new("aggregator").unwrap();

        // Library copy with a real SKILL.md.
        write_library_skill(
            &paths,
            "aggregator",
            "---\nname: aggregator\ndescription: pulls things together\n---\n# Aggregator\n\nBody.",
        );
        // Manifest entry.
        let mut manifest = Manifest::default();
        manifest.insert(
            name.clone(),
            SkillEntry::new(
                paths.library_dir().join("aggregator"),
                DirectoryName::new("dotfiles").unwrap(),
                test_hash("aggregator"),
                false,
            ),
        );
        manifest::save(&manifest, paths.config_dir()).unwrap();

        let detail = collect_detail(&name, &config, &paths).unwrap();
        assert_eq!(detail.name, name);
        assert!(detail.source_path.ends_with("aggregator"));
        assert_eq!(detail.content_hash, test_hash("aggregator"));
        assert!(detail.last_sync.is_some());
        assert!(!detail.managed);
        // disabled state is read from the user's real ~/.config/tome/machine.toml;
        // a clean test run may or may not have that file. The Bool just has
        // to deserialize; assert the contract by checking the field exists
        // and the function returned Ok.
        assert_eq!(
            detail.frontmatter.name.as_deref(),
            Some("aggregator"),
            "frontmatter projection must carry the name field"
        );
        assert!(
            detail.body.contains("# Aggregator"),
            "body must carry the markdown post-frontmatter content; got: {}",
            detail.body
        );
    }

    #[test]
    fn collect_detail_truncates_body_beyond_1mb() {
        let (_tmp, config, paths) = temp_paths_with_library();
        let name = SkillName::new("verbose").unwrap();

        // 1.5 MiB of body text — must trigger the truncation marker.
        let big_body = "a".repeat(1_500_000);
        let content = format!("---\nname: verbose\n---\n{big_body}");
        write_library_skill(&paths, "verbose", &content);

        let mut manifest = Manifest::default();
        manifest.insert(
            name.clone(),
            SkillEntry::new(
                paths.library_dir().join("verbose"),
                DirectoryName::new("dotfiles").unwrap(),
                test_hash("verbose"),
                false,
            ),
        );
        manifest::save(&manifest, paths.config_dir()).unwrap();

        let detail = collect_detail(&name, &config, &paths).unwrap();
        assert!(
            detail.body.contains("[... truncated ...]"),
            "body beyond {}B must carry the truncation marker; got len={}",
            SKILL_BODY_MAX_BYTES,
            detail.body.len()
        );
        // 1 MiB cap + marker tail.
        assert!(
            detail.body.len() <= SKILL_BODY_MAX_BYTES + 32,
            "body length must be capped; got {}",
            detail.body.len()
        );
    }

    #[test]
    fn collect_detail_errors_when_skill_missing_from_manifest() {
        let (_tmp, config, paths) = temp_paths_with_library();
        let name = SkillName::new("ghost").unwrap();
        let err = collect_detail(&name, &config, &paths).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("ghost") && msg.contains("not found"),
            "missing-skill error must name the skill; got: {msg}"
        );
    }
}
