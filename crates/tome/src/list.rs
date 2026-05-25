//! `tome list` domain computation — discover every skill and return it as a
//! structured [`ListReport`].
//!
//! This is the CORE-01 / D-GUI-08 extraction for the `list` command: the
//! presentation (text table / JSON) stays inline in `lib.rs::cmd_list`, while
//! the *computation* (discover + sort + collect warnings) moves here behind a
//! `pub fn` returning a structured type. The GUI (`tome-desktop`, later phases)
//! calls [`collect`] directly and renders the [`ListReport`] without going
//! through any CLI formatting — mirroring the `status::gather` / `status::show`
//! split that is the CORE-01 template.

use std::collections::BTreeMap;

use anyhow::Result;

use crate::config::Config;
use crate::discover::{self, DiscoveredSkill};

/// The structured result of `tome list`: every discovered skill (sorted by
/// name) plus any non-fatal discovery warnings.
///
/// Field shapes are deliberately the same `DiscoveredSkill` / `String` types
/// the rest of the crate already uses — no list-specific wrapper types — so the
/// GUI consumes the same vocabulary the CLI does (the library-canonical types
/// are the contract, STATE.md).
pub struct ListReport {
    /// Discovered skills, sorted alphabetically by skill name.
    pub skills: Vec<DiscoveredSkill>,
    /// Non-fatal warnings emitted during discovery (naming-convention hints,
    /// deduplication notices). The CLI prints these to stderr unless `--quiet`;
    /// the GUI can surface them in a diagnostics view.
    pub warnings: Vec<String>,
}

/// Discover all skills for `tome list` and return them as a structured
/// [`ListReport`].
///
/// Discovery uses an empty `resolved_paths` map (git directories are listed at
/// their config URL, matching the previous inline `list()` behavior — listing
/// does not clone). Skills are sorted by name so both the CLI table and the GUI
/// list get a stable order.
pub fn collect(config: &Config) -> Result<ListReport> {
    let mut warnings = Vec::new();
    let mut skills = discover::discover_all(config, &BTreeMap::new(), &mut warnings)?;
    skills.sort_by(|a, b| a.name.as_str().cmp(b.name.as_str()));
    Ok(ListReport { skills, warnings })
}
