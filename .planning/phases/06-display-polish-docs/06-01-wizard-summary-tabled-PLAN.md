---
phase: 06-display-polish-docs
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - Cargo.toml
  - crates/tome/Cargo.toml
  - crates/tome/src/wizard.rs
autonomous: true
requirements:
  - WHARD-07
must_haves:
  truths:
    - "Running `tome init` renders the directory summary as a bordered tabled table with NAME / TYPE / ROLE / PATH columns (per D-01, D-02)"
    - "Header row is bold via Modify + Format::content (per D-03), matching the shape of status.rs:185-191"
    - "PATH cells are rendered via paths::collapse_home() so `/Users/martin/...` becomes `~/...` (per D-06)"
    - "When the assembled table exceeds terminal width, Width::truncate(term_cols).priority(PriorityMax::right()) shrinks the widest column first, appending the tabled default ellipsis (per D-04)"
    - "When terminal width cannot be detected (non-TTY, piped, CI), the code falls back to 80 columns (per D-05)"
    - "The empty-directories branch still prints the literal string `(no directories configured)` with no tabled rendering (per D-07)"
    - "`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` all pass"
  artifacts:
    - path: "Cargo.toml"
      provides: "workspace dependency declaration for terminal_size 0.4"
      contains: "terminal_size ="
    - path: "crates/tome/Cargo.toml"
      provides: "crate-level terminal_size dependency pulling from workspace"
      contains: "terminal_size.workspace = true"
    - path: "crates/tome/src/wizard.rs"
      provides: "show_directory_summary() rewritten to use tabled::Table with Style::rounded() and Width::truncate truncation"
      contains: "Style::rounded()"
  key_links:
    - from: "crates/tome/src/wizard.rs::show_directory_summary"
      to: "tabled::Table + Style::rounded() + Width::truncate(...).priority(PriorityMax::right())"
      via: "Table::from_iter + chained .with(...) calls (mirrors status.rs:185-193 template)"
      pattern: "Table::from_iter.*Style::rounded"
    - from: "crates/tome/src/wizard.rs::show_directory_summary"
      to: "terminal_size::terminal_size()"
      via: "direct call with .map(|(w, _)| w.0 as usize).unwrap_or(80) fallback"
      pattern: "terminal_size\\(\\)"
    - from: "crates/tome/src/wizard.rs::show_directory_summary"
      to: "crate::paths::collapse_home"
      via: "applied to each PATH cell before width calculation"
      pattern: "collapse_home"
---

<objective>
Migrate `wizard::show_directory_summary()` (`crates/tome/src/wizard.rs:413-436`) from manual `println!` column formatting to the `tabled` crate, using `Style::rounded()` borders and terminal-width-aware truncation via `Width::truncate(..).priority(PriorityMax::right())`. Mirror the structure of `status.rs:185-193` but diverge intentionally on the border style (rounded vs blank) per D-01.

Purpose: Implement WHARD-07. Long paths (especially `~/.tome/repos/<sha256>/...` from git sources) render without breaking column alignment — tabled handles truncation with an ellipsis rather than letting cells wrap or overflow. Terminal-width detection is new — introduces the `terminal_size` crate as a direct workspace dependency.

Output: `wizard::show_directory_summary` rewritten (~20 LoC body), `Cargo.toml` workspace dep and `crates/tome/Cargo.toml` crate dep both updated with `terminal_size = "0.4"`, `make ci` green.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/phases/06-display-polish-docs/06-CONTEXT.md
@crates/tome/src/wizard.rs
@crates/tome/src/status.rs

<interfaces>
<!-- Key code the executor needs. Copy-ready excerpts so no discovery is needed. -->

Current wizard.rs:413-436 (to be replaced):
```rust
fn show_directory_summary(directories: &BTreeMap<DirectoryName, DirectoryConfig>) {
    if directories.is_empty() {
        println!("  (no directories configured)");
        return;
    }
    // Header
    println!(
        "  {:<20} {:<35} {:<16} {}",
        style("Name").bold(),
        style("Path").bold(),
        style("Type").bold(),
        style("Role").bold(),
    );
    for (name, cfg) in directories {
        println!(
            "  {:<20} {:<35} {:<16} {}",
            name,
            cfg.path.display(),
            cfg.directory_type,
            cfg.role().description(),
        );
    }
    println!();
}
```

Reference implementation from status.rs:185-193 (template to mirror, minus SKILLS column, minus blank style):
```rust
let table = tabled::Table::from_iter(rows)
    .with(Style::blank())
    .with(
        Modify::new(Rows::first()).with(tabled::settings::Format::content(|s| {
            style(s).bold().to_string()
        })),
    )
    .to_string();
println!("{table}");
```

Existing wizard.rs imports (top of file) — uses `console::{Term, style}` at line 8.
Existing status.rs imports to mirror: `use tabled::settings::{Modify, Style, object::Rows};` (line 6).

terminal_size 0.4 API (verified via docs.rs):
```rust
use terminal_size::{Width, terminal_size};
let cols: usize = terminal_size().map(|(Width(w), _)| w as usize).unwrap_or(80);
```

tabled 0.20 truncation API (verified via docs.rs):
```rust
use tabled::settings::Width;
use tabled::settings::peaker::PriorityMax;
.with(Width::truncate(cols).priority(PriorityMax::right()))
```
PriorityMax has `::left()` and `::right()` constructors; `::right()` is the standard "shrink widest" choice when the longest overflow is on the right side (PATH column in our case).

Column ordering per D-02: NAME / TYPE / ROLE / PATH (matches status.rs minus SKILLS).
Header row bolding per D-03: `Modify::new(Rows::first()).with(Format::content(|s| style(s).bold().to_string()))`.
PATH cells run through `crate::paths::collapse_home(Path::new(&cfg.path))` per D-06 — returns a String.
ROLE cell content: `cfg.role().description()` (plain-english parenthetical from `DirectoryRole::description()`).
TYPE cell content: `cfg.directory_type.to_string()` (lowercase-hyphen form via `Display`).
Empty-directories branch per D-07: KEEP the current `if directories.is_empty() { println!("  (no directories configured)"); return; }` guard verbatim — no tabled rendering when empty.

Cargo.toml workspace deps are alphabetically ordered. Insert `terminal_size = "0.4"` in alphabetical order. Current alphabetical neighborhood is:
```
tabled = "0.20"
toml = "1"
```
Insert between these: `tabled` → `terminal_size` → `toml`.

crates/tome/Cargo.toml `[dependencies]` section uses `.workspace = true` pattern. Insert `terminal_size.workspace = true` alphabetically between `tabled.workspace = true` and `toml.workspace = true`.
</interfaces>
</context>

<tasks>

<task type="auto" tdd="false">
  <name>Task 1: Add terminal_size crate as workspace + crate dependency</name>
  <files>Cargo.toml, crates/tome/Cargo.toml</files>
  <read_first>
    - /Users/martin/dev/opensource/tome/Cargo.toml (workspace manifest — see current [workspace.dependencies] block, alphabetical ordering)
    - /Users/martin/dev/opensource/tome/crates/tome/Cargo.toml (crate manifest — see current [dependencies] block, all using `.workspace = true`)
    - /Users/martin/dev/opensource/tome/.planning/phases/06-display-polish-docs/06-CONTEXT.md (D-04 rationale for terminal_size; D-06 for `paths::collapse_home`)
  </read_first>
  <action>
    Add the `terminal_size` crate at version `"0.4"` as a workspace dependency and propagate to the `tome` crate.

    **Edit 1: `/Users/martin/dev/opensource/tome/Cargo.toml`**
    In the `[workspace.dependencies]` table (currently lines 13-28), insert `terminal_size = "0.4"` in alphabetical order between `tabled = "0.20"` (line 24) and `toml = "1"` (line 25). Resulting ordering in that neighborhood:
    ```
    tabled = "0.20"
    terminal_size = "0.4"
    toml = "1"
    ```

    **Edit 2: `/Users/martin/dev/opensource/tome/crates/tome/Cargo.toml`**
    In the `[dependencies]` section (currently lines 14-29), insert `terminal_size.workspace = true` in alphabetical order between `tabled.workspace = true` (line 26) and `toml.workspace = true` (line 27). Resulting ordering:
    ```
    tabled.workspace = true
    terminal_size.workspace = true
    toml.workspace = true
    ```

    Do NOT touch `[dev-dependencies]`, `[profile.*]`, `[[bin]]`, or any other section. Do NOT add features flags — the crate is used with default features.

    After writing, run `cargo check -p tome` to verify the dep resolves and `Cargo.lock` updates cleanly.
  </action>
  <verify>
    <automated>grep -E '^terminal_size = "0\.4"' Cargo.toml &amp;&amp; grep -E '^terminal_size\.workspace = true' crates/tome/Cargo.toml &amp;&amp; cargo check -p tome</automated>
  </verify>
  <acceptance_criteria>
    - `grep 'terminal_size = "0.4"' /Users/martin/dev/opensource/tome/Cargo.toml` returns exactly one match.
    - `grep 'terminal_size.workspace = true' /Users/martin/dev/opensource/tome/crates/tome/Cargo.toml` returns exactly one match.
    - In `Cargo.toml`, the line `terminal_size = "0.4"` appears between the lines containing `tabled = "0.20"` and `toml = "1"` (alphabetical order preserved).
    - In `crates/tome/Cargo.toml`, the line `terminal_size.workspace = true` appears between the lines containing `tabled.workspace = true` and `toml.workspace = true`.
    - `cargo check -p tome` exits 0 with no new warnings.
    - `Cargo.lock` contains a `[[package]]` entry for `terminal_size` at version 0.4.x (generated automatically by cargo).
  </acceptance_criteria>
  <done>terminal_size 0.4 is a resolvable dependency of the tome crate; `cargo check -p tome` passes.</done>
</task>

<task type="auto" tdd="false">
  <name>Task 2: Rewrite show_directory_summary using tabled with rounded borders and width-aware truncation</name>
  <files>crates/tome/src/wizard.rs</files>
  <read_first>
    - /Users/martin/dev/opensource/tome/crates/tome/src/wizard.rs (the file under edit — see current `show_directory_summary` at lines 413-436 and current imports at lines 7-16)
    - /Users/martin/dev/opensource/tome/crates/tome/src/status.rs (reference template — see imports at line 6 and `Table::from_iter` usage at lines 185-193)
    - /Users/martin/dev/opensource/tome/crates/tome/src/paths.rs (confirm `collapse_home` signature — `pub fn collapse_home(path: &amp;Path) -> String`)
    - /Users/martin/dev/opensource/tome/crates/tome/src/config.rs (confirm `DirectoryRole::description()` returns `&amp;'static str` around lines 142-186, confirm `DirectoryType: Display`)
    - /Users/martin/dev/opensource/tome/.planning/phases/06-display-polish-docs/06-CONTEXT.md (decisions D-01..D-07)
  </read_first>
  <action>
    Replace the body of `show_directory_summary` at `crates/tome/src/wizard.rs:413-436` with a tabled-based implementation and add the necessary imports.

    **Step 1 — Add imports.** At the top of `crates/tome/src/wizard.rs`, add the following use statements (grouped with existing `use` lines, not inside the function):
    ```rust
    use tabled::Table;
    use tabled::settings::{Modify, Style, Width, object::Rows, peaker::PriorityMax};
    use tabled::settings::Format;
    use terminal_size::{Width as TermWidth, terminal_size};
    ```
    Group them logically with the existing `use` block. Do not remove or reorder existing imports. If `Format` and other items can be combined into a single `use tabled::settings::{...}` line, prefer that shape for consistency with `status.rs:6`. Example consolidated form:
    ```rust
    use tabled::Table;
    use tabled::settings::{Format, Modify, Style, Width, object::Rows, peaker::PriorityMax};
    use terminal_size::{Width as TermWidth, terminal_size};
    ```

    **Step 2 — Rewrite the function.** Replace lines 413-436 verbatim with:
    ```rust
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
            .with(
                Modify::new(Rows::first())
                    .with(Format::content(|s| style(s).bold().to_string())),
            )
            .with(Width::truncate(term_cols).priority(PriorityMax::right()))
            .to_string();
        println!("{table}");
        println!();
    }
    ```

    **Step 3 — Keep all three call sites untouched.** Do NOT modify `wizard.rs:181-183`, `wizard.rs:231-233`, `wizard.rs:297-299`, or `wizard.rs:306-322`. The function signature is unchanged (still `fn show_directory_summary(directories: &BTreeMap<DirectoryName, DirectoryConfig>)`), so callers continue to work as-is.

    **Step 4 — No new tests required.** Per the CONTEXT.md "Test Coverage Expectations" section: tabled is a third-party crate producing strings, and substring-level assertions are Phase 5 precedent (D-09). Skip snapshot tests and skip adding dedicated unit tests for `show_directory_summary` — they'd be tautological wiring tests. Existing integration tests (e.g., `tests/cli.rs` splitting stdout on `Generated config:`) continue to work because the tabled-rendered summary sits before that marker.

    **Step 5 — Verify.** Run in sequence:
    ```
    cargo fmt
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo test
    ```
    All four must exit 0. If `cargo clippy` flags the `Format::content` closure (e.g., unnecessary closure), follow the clippy suggestion — but prefer the closure form since it matches `status.rs` line-for-line.
  </action>
  <verify>
    <automated>cargo fmt --check &amp;&amp; cargo clippy --all-targets -- -D warnings &amp;&amp; cargo test</automated>
  </verify>
  <acceptance_criteria>
    - `grep 'Style::rounded()' crates/tome/src/wizard.rs` returns at least one match.
    - `grep 'Width::truncate' crates/tome/src/wizard.rs` returns at least one match.
    - `grep 'PriorityMax::right' crates/tome/src/wizard.rs` returns at least one match.
    - `grep 'collapse_home' crates/tome/src/wizard.rs` returns at least one match (D-06 wiring).
    - `grep 'terminal_size()' crates/tome/src/wizard.rs` returns at least one match (D-04 wiring).
    - `grep 'unwrap_or(80)' crates/tome/src/wizard.rs` returns at least one match (D-05 fallback).
    - `grep '"  (no directories configured)"' crates/tome/src/wizard.rs` still returns a match (D-07 empty-state preserved verbatim).
    - `grep 'Modify::new(Rows::first())' crates/tome/src/wizard.rs` returns a match (D-03 header bolding shape).
    - `cargo fmt --check` exits 0.
    - `cargo clippy --all-targets -- -D warnings` exits 0.
    - `cargo test` exits 0 — all existing tests, including `tests/cli.rs` integration tests, still pass.
    - `cargo run -- --help` succeeds (no runtime wiring regressions).
  </acceptance_criteria>
  <done>`show_directory_summary` renders via `tabled` with `Style::rounded()`, NAME/TYPE/ROLE/PATH columns, bold header, `collapse_home`-transformed PATH cells, and terminal-width-aware truncation with an 80-col fallback. Empty-directory branch still prints the literal placeholder. All CI gates pass.</done>
</task>

</tasks>

<verification>
Run the project CI pipeline end-to-end:
```
make ci
```
This runs `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` (unit + integration). All must pass.

Additionally, sanity-test the rendered output interactively (not required for pass, but useful for Phase 6 UAT):
```
cargo run -- init --dry-run --no-input
```
Expected: A rounded-border table with four columns (NAME / TYPE / ROLE / PATH), bold headers, and paths starting with `~/` instead of `/Users/…`. If the terminal is narrow, the PATH column is truncated with `…`.
</verification>

<success_criteria>
- WHARD-07 Requirement met: `show_directory_summary` uses `tabled` with `Style::rounded()`, handles long paths via `Width::truncate(..).priority(PriorityMax::right())`, and respects terminal width (with 80-col fallback).
- `terminal_size = "0.4"` is a resolved workspace + crate dependency.
- `make ci` passes on the working tree.
- Zero behavior change for existing wizard flow, integration tests, or call sites — only the rendered summary output changes.
</success_criteria>

<output>
After completion, create `.planning/phases/06-display-polish-docs/06-01-wizard-summary-tabled-SUMMARY.md` using the standard summary template. Highlights to capture:
- Exact line range replaced in `wizard.rs`
- Final shape of the tabled pipeline (Style::rounded → Modify header bold → Width::truncate with PriorityMax::right)
- terminal_size dep version resolved
- Any clippy adjustments made
- Confirmation that all three call sites (lines 181/231/297) and the `--dry-run` branch continue to work untouched
</output>
