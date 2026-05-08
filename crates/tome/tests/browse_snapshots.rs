//! HARD-12 — ratatui `TestBackend` + `insta` snapshot tests for `browse/ui.rs`.
//!
//! Renders `App` fixtures into a fixed-size `TestBackend`, captures the
//! buffer as a plain-text grid, and compares against committed `.snap`
//! files. Locks the visual regression contract for the browse TUI.
//!
//! Coverage scope (per Plan 15-05 Task 1 `<behavior>`):
//!
//! - status dashboard (default state, dark + light themes)
//! - skill list (default, empty, fuzzy-filtered, source-grouped)
//! - detail pane (managed, local, unowned)
//! - help overlay
//! - post-toggle status surface (Task 2 / HARD-21 wiring smoke)
//!
//! ## Why a fixed 120x40 backend
//!
//! All snapshots render into a `TestBackend::new(120, 40)`. The width is
//! wide enough to keep the path column from wrapping for typical
//! `~/.tome/library/<name>` shapes; the height fits the help overlay
//! (18 rows) plus the surrounding chrome (status bar, header) without
//! awkward truncation. Fixing one canonical size keeps every `.snap`
//! diff focused on layout/text changes rather than terminal-size noise.
//!
//! ## Why snapshots, not assertion-style tests
//!
//! The browse module is render-heavy: layout + theming + fuzzy match
//! highlighting + markdown preview all interact at the buffer level.
//! Spot assertions (`buf.contains("foo")`) miss column-alignment, color
//! swaps, and overlay-layering regressions; full-buffer snapshots catch
//! those by construction. The trade-off — easy "accept on diff" review
//! becoming a foot-gun — is mitigated by reviewing every `.snap.new`
//! diff in the PR before accepting.

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;

use tome::browse::app::{App, DetailAction, SkillRow};
use tome::browse::theme::Theme;
use tome::browse::ui;
use tome::config::DirectoryName;
use tome::machine::MachinePrefs;

/// Canonical terminal size for every snapshot. See module-level rationale.
const W: u16 = 120;
const H: u16 = 40;

fn render_to_string(app: &App) -> String {
    let backend = TestBackend::new(W, H);
    let mut terminal = Terminal::new(backend).expect("Terminal::new");
    // ratatui 0.30: `terminal.draw(...)` returns a CompletedFrame whose
    // `.buffer` we re-borrow via `terminal.backend().buffer()` below.
    terminal.draw(|frame| ui::render(frame, app)).expect("draw");
    buf_to_string(terminal.backend().buffer())
}

/// Flatten a ratatui Buffer into a string of rows separated by `\n`.
/// Trailing whitespace per row is preserved so column alignment is
/// part of the snapshot — that's the whole point.
fn buf_to_string(buf: &Buffer) -> String {
    let mut out = String::new();
    let area = buf.area;
    for y in 0..area.height {
        for x in 0..area.width {
            // ratatui 0.30: index by `Position` for the Buffer accessor.
            let cell = &buf[(x, y)];
            out.push_str(cell.symbol());
        }
        // Right-trim per-row trailing spaces so the snapshot doesn't
        // bake in a 120-wide rectangle of whitespace; the end-of-row
        // position is implicit from the line break.
        let trimmed = out.trim_end_matches(' ').to_string();
        out = trimmed;
        out.push('\n');
    }
    out
}

// === Fixtures ===

fn skill_row(name: &str, source: &str, path: &str, managed: bool, synced_at: &str) -> SkillRow {
    SkillRow {
        name: name.to_string(),
        source: source.to_string(),
        path: path.to_string(),
        managed,
        synced_at: synced_at.to_string(),
        source_directory: None,
    }
}

/// Five-skill default fixture: mixes managed + local sources so the
/// status-dashboard / skill-list snapshots cover both visual paths.
fn five_skill_fixture() -> Vec<SkillRow> {
    vec![
        skill_row(
            "alpha-helpers",
            "claude-plugins",
            "~/.tome/library/alpha-helpers",
            true,
            "2026-05-08T00:00:00Z",
        ),
        skill_row(
            "beta-tools",
            "local",
            "~/.tome/library/beta-tools",
            false,
            "2026-05-07T12:30:00Z",
        ),
        skill_row(
            "foo-bar",
            "local",
            "~/.tome/library/foo-bar",
            false,
            "2026-05-06T09:15:00Z",
        ),
        skill_row(
            "fixture-skill",
            "claude-plugins",
            "~/.tome/library/fixture-skill",
            true,
            "2026-05-05T18:45:00Z",
        ),
        skill_row(
            "zeta-utility",
            "local",
            "~/.tome/library/zeta-utility",
            false,
            "2026-05-04T08:00:00Z",
        ),
    ]
}

// === Snapshot tests ===

#[test]
fn snapshot_status_dashboard_default() {
    // Status row at the bottom of the browse view in default state
    // (5 skills, no filter, dark theme). Locks the status-bar layout
    // (count badge + key/label hint pairs).
    let app = App::for_snapshot(five_skill_fixture(), Theme::dark(), None);
    let out = render_to_string(&app);
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_skill_list_default() {
    // Default browse view: 5 skills, dark theme, no filter. The first
    // row is selected (highlighted background) and the right pane shows
    // the preview header for the selected skill.
    let app = App::for_snapshot(five_skill_fixture(), Theme::dark(), None);
    let out = render_to_string(&app);
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_skill_list_empty() {
    // Empty-state: zero skills. Verifies the "No matching skill." preview
    // copy and that the body table renders without a row.
    let app = App::for_snapshot(Vec::new(), Theme::dark(), None);
    let out = render_to_string(&app);
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_skill_list_filtered() {
    // Fuzzy filter "fo" should match "foo-bar" (and possibly other rows
    // by subsequence match). The filter text appears in the status bar
    // and the matched characters in the skill name are highlighted.
    let app = App::for_snapshot(five_skill_fixture(), Theme::dark(), Some("fo"));
    let out = render_to_string(&app);
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_skill_list_grouped_by_source() {
    // Tab toggles `group_by_source`. Combined with SortMode::Source it
    // inserts a group-header row per source. Locks that visual.
    let mut app = App::for_snapshot(five_skill_fixture(), Theme::dark(), None);
    app.sort_mode = tome::browse::app::SortMode::Source;
    app.group_by_source = true;
    // Re-run apply_sort via the public refilter() so the new sort_mode
    // takes effect in the rendered table.
    app.refilter_for_snapshot();
    let out = render_to_string(&app);
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_detail_pane_managed_skill() {
    // Detail pane for the first row (managed skill). Layout: title +
    // metadata block (Source/Type/Path/Synced) on the left, action list
    // below it, preview on the right.
    let mut app = App::for_snapshot(five_skill_fixture(), Theme::dark(), None);
    app.enter_detail_mode_for_snapshot();
    let out = render_to_string(&app);
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_detail_pane_local_skill() {
    // Detail pane with a local-source row selected. The "Type:" line
    // should read "local"; the action list is identical to the managed
    // case but the metadata differs.
    let mut app = App::for_snapshot(five_skill_fixture(), Theme::dark(), None);
    // Move selection to row 1 (beta-tools, local) before entering detail.
    app.selected = 1;
    app.enter_detail_mode_for_snapshot();
    let out = render_to_string(&app);
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_detail_pane_unowned_skill() {
    // Phase 14 D-C1: an Unowned library entry has no source directory.
    // The `source` field on the SkillRow surfaces as the previous-source
    // string; managed = false so type label reads "local" (Unowned skills
    // are local-shaped from the browse module's perspective — the browse
    // module doesn't care about ownership state, only display fields).
    // This snapshot locks the rendering for that fixture shape.
    let rows = vec![skill_row(
        "unowned-skill",
        "(unowned)",
        "~/.tome/library/unowned-skill",
        false,
        "2026-05-08T00:00:00Z",
    )];
    let mut app = App::for_snapshot(rows, Theme::dark(), None);
    app.enter_detail_mode_for_snapshot();
    let out = render_to_string(&app);
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_help_overlay_default() {
    // Help overlay overlaid on the default browse view. The overlay is
    // a centered popup with all keyboard shortcuts; the underlying
    // skill list shows through outside the popup.
    let mut app = App::for_snapshot(five_skill_fixture(), Theme::dark(), None);
    app.enter_help_mode_for_snapshot();
    let out = render_to_string(&app);
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_theme_light_status_dashboard() {
    // Light theme exercises the indexed-color palette. Status bar
    // background + count badge swap to the light-mode variants.
    let app = App::for_snapshot(five_skill_fixture(), Theme::light(), None);
    let out = render_to_string(&app);
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_theme_light_skill_list() {
    // Light-theme skill-list. Selected-row background uses
    // Color::Indexed(254) instead of DarkGray.
    let app = App::for_snapshot(five_skill_fixture(), Theme::light(), None);
    let out = render_to_string(&app);
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_detail_pane_after_disable_toggle() {
    // HARD-21 D-BROWSE-3 step 4: post-toggle snapshot. After pressing
    // Disable in detail mode, the action label flips to "Enable on this
    // machine" and the status bar surfaces "Disabled foo on this machine".
    // This snapshot locks both visual signals.
    //
    // Fixture: a single skill row anchored to directory "bar" (no per-dir
    // blocklist/allowlist set), so the toggle scope falls to Global.
    let mut row = skill_row(
        "foo",
        "bar",
        "~/.tome/library/foo",
        false,
        "2026-05-08T00:00:00Z",
    );
    row.source_directory = Some(DirectoryName::new("bar").unwrap());

    let tmp = tempfile::TempDir::new().expect("tempdir");
    let machine_path = tmp.path().join("machine.toml");
    let app = App::for_snapshot(vec![row], Theme::dark(), None);
    let mut app = app.with_machine_prefs(MachinePrefs::default(), machine_path);
    // Enter detail mode (materializes the action list with `Disable`
    // in slot 2 since machine_prefs has nothing disabled yet). Then
    // route through `execute_action_for_snapshot` to mirror the
    // production keyflow: apply_toggle + refresh_detail_actions both
    // fire, so the rendered slot 2 flips from Disable → Enable and
    // the status bar carries the Success body.
    app.enter_detail_mode_for_snapshot();
    app.execute_action_for_snapshot(DetailAction::Disable);
    let out = render_to_string(&app);
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_theme_light_filtered() {
    // Light theme + active fuzzy filter: match-highlight color in the
    // skill name swaps to Color::Indexed(136) (dark yellow).
    let app = App::for_snapshot(five_skill_fixture(), Theme::light(), Some("fix"));
    let out = render_to_string(&app);
    insta::assert_snapshot!(out);
}
