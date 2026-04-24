# Phase 8: Safety Refactors (Partial-Failure Visibility & Cross-Platform) - Research

**Researched:** 2026-04-24
**Domain:** Rust CLI bug-fix refactor across three narrow surfaces (`remove.rs`, `browse/*`, `relocate.rs`)
**Confidence:** HIGH — decisions are fully locked in CONTEXT.md; research confirms file:line refs, the reference-PR warning format, `arboard` current state, and CI config.

## Summary

Phase 8 closes three P0/P1 safety findings from the v0.7 `/pr-review-toolkit` audit. All twenty implementation decisions (D-01..D-20) are locked in `08-CONTEXT.md`; this research is surgical verification of the external facts the planner needs to translate those decisions into plan tasks.

**Key verifications:**

- All CONTEXT.md file:line references resolve (some drifted ±2–5 lines but within the same block — noted per section below).
- Issue #449 references "PR #417"; that issue number does not exist on GitHub. The actual reference fix landed in **PR #448** (commit `d6e9080`) which closed issues #415, #417, #418. The canonical warning format is at `lib.rs:687-693`: `eprintln!("warning: could not read HEAD sha for '{}' cache at {}: {e}", name, cache_dir.display())` — note the `'{}' … at {}` shape and the `{e}` interpolation. D-13 should mirror this word-for-word shape.
- `arboard` 3.6.1 is current (published 2025-08-23), MIT OR Apache-2.0. Default features include `image-data` which pulls the heavy `image` crate — **tome wants `default-features = false`** for a text-only use case. No built-in test/mock hook exists, confirming D-19's "otherwise skip" fallback.
- `ubuntu-latest` GitHub runner does **not** pre-install `xdg-utils`. Because D-17/D-19 pick platform-agnostic unit tests (no direct `xdg-open` invocation), this is benign — the planner just needs to know the Linux branch's *runtime* path is un-exercised on CI, only the *compile* path is.
- `chmod 0o000` fixture pattern for an "unremovable" integration test is already in `tests/cli.rs:2232-2249` (`edge_permission_denied_on_target`) — SAFE-01 integration test can be a direct clone of that shape.
- `gag` crate is **not** currently a dev-dependency. No existing test captures stderr from in-process code. D-20 should use observable side-effects (assert `source_path` stays `None`) instead of stderr capture, or add `gag` to dev-deps — CONTEXT.md D-20 already states "if adding a test helper is heavy, assert via observable side-effect".

**Primary recommendation:** Plans should execute the locked decisions verbatim. No decision needs revisiting. The two research-driven adjustments to propagate:
1. `arboard` entry must be `arboard = { version = "3", default-features = false }` to avoid pulling `image` (~20 transitive deps).
2. For D-20, default to the observable-side-effect assertion (`source_path.is_none()`) and skip `gag` — adding a dev-dep for one warning assertion is heavier than the signal.

## User Constraints (from CONTEXT.md)

### Locked Decisions

**SAFE-01: `remove::execute` Partial-Failure Aggregation**

- **D-01:** `RemoveResult.failures: Vec<RemoveFailure>` where `RemoveFailure { path: PathBuf, op: FailureKind, error: io::Error }` and `FailureKind` is an enum with variants `Symlink`, `LibraryDir`, `LibrarySymlink`, `GitCache`.
- **D-02:** Keep existing success counts (`symlinks_removed`, `library_entries_removed`, `git_cache_removed`) alongside `failures`. Drop `#[allow(dead_code)]` on `git_cache_removed`.
- **D-03:** Drop the existing `eprintln!("warning: ...")` lines inside each `execute` loop body. Caller is single source of warning output.
- **D-04:** `Command::Remove` in `lib.rs`: on non-empty `result.failures`, print the existing `✓ Removed directory 'X': ...` line AND a follow-up `⚠ K operations failed — run tome doctor` line (grouped by `FailureKind` with per-path detail), then return `Err(anyhow::anyhow!("remove completed with K failures"))`.
- **D-05:** Error-line format groups by `FailureKind` first, then lists per-path entries. `⚠` in yellow via `console::style`. Paths rendered via `paths::collapse_home()`.
- **D-06:** Keep current save order: save config → save manifest → regenerate and save lockfile → print summary → return Err if failures.

**SAFE-02: Cross-Platform Browse Actions + Status Bar Feedback**

- **D-07:** Add `arboard` as a direct dependency (latest 3.x). Replace `sh -c "echo -n '${path}' | pbcopy"` with `arboard::Clipboard::new()?.set_text(path)?`.
- **D-08:** Replace `Command::new("open").arg(&path).spawn()` with `cfg!`-based dispatch: `let binary = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };`.
- **D-09:** Add `status_message: Option<String>` field to `browse::app::App`.
- **D-10:** Clear `status_message` at the top of `handle_key` **before** dispatching to mode-specific handlers.
- **D-11:** `ui.rs` bottom-bar render: when `app.status_message` is `Some`, render the message *in place of* the static keybind line. Works identically in `Mode::Normal` and `Mode::Detail`.
- **D-12:** Message styling: Success `✓ Copied: ~/.tome/skills/my-skill` in `theme.accent`; Failure `⚠ Could not open: ...` in a yellow-family color (reuse `theme.alert` if it exists, else add `theme.warning`).

**SAFE-03: Relocate Symlink-Read Warning**

- **D-13:** Replace `std::fs::read_link(&link_path).ok()` at `relocate.rs:93` with explicit `match` that on `Err(e)` emits `eprintln!("warning: could not read symlink at {}: {e}", link_path.display())` and returns `None`. No `!cli.quiet` gate.
- **D-14:** SAFE-03 is the only site changed. Do NOT touch `theme.rs:115-117` env-parse fallback or `git.rs:69` unused-variable suppression.

**Packaging & Tests**

- **D-15:** Three plans, one per SAFE-XX requirement (08-01 SAFE-01, 08-02 SAFE-02, 08-03 SAFE-03).
- **D-16:** No cross-plan ordering constraint.
- **D-17:** **No test abstraction traits.** CI matrix (`ubuntu-latest` + `macos-latest`) exercises platform branches.
- **D-18:** SAFE-01 tests: unit test (failure injection in `remove.rs` tests) + integration test (`tests/cli.rs`).
- **D-19:** SAFE-02 tests: unit tests for `status_message` lifecycle + `execute_action` error handling (if feasible). No `#[cfg(target_os = "linux")]` direct tests.
- **D-20:** SAFE-03 test: unit test in `relocate.rs` tests module; assert via observable side-effect (`source_path` stays `None`) if stderr capture is heavy.

### Claude's Discretion

- Exact text of `⚠ K operations failed` summary line.
- Whether `FailureKind` enum lives in `remove.rs` (default) or moves to shared module.
- Whether `arboard` version pin is loose (`"3"`) or tight (`"3.4"`). Default: loose per repo convention.
- Specific color choice for failure message if `theme` doesn't expose a warning color. Default: reuse `theme.alert` (yellow) which already exists.
- Whether to also touch `DetailAction::Disable`/`Enable` silent-noop path. Default: leave alone.
- Alphabetical placement of `arboard` in `Cargo.toml`.

### Deferred Ideas (OUT OF SCOPE)

- Backport eprintln-warning pattern to every `.ok()` site in the codebase.
- `tome doctor` integration for `RemoveFailure`.
- `--quiet` / `--verbose` flag handling for SAFE-03's warning.
- Windows support.
- `DetailAction::Disable`/`Enable` machine.toml wiring.
- Cross-machine portability (PORT-01..04) — deferred to v0.9.
- Snapshot-testing the new `⚠` summary line.
- Refactoring `lib.rs::Command::Remove`'s 50-line body.

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SAFE-01 (#413) | `remove::execute` aggregates partial-cleanup failures into `RemoveResult`; caller surfaces them via non-zero exit + distinct "⚠ N operations failed" summary | All 4 failure loops in `remove.rs:192-253` verified; `Command::Remove` handler at `lib.rs:362-414` verified; `chmod 0o000` pattern for integration test exists at `tests/cli.rs:2232-2249`; existing tome-remove test scaffold at `tests/cli.rs:3171+` (helper `remove_test_env`) |
| SAFE-02 (#414) | Browse UI `ViewSource` + `CopyPath` cross-platform via `arboard` + `xdg-open`; failures surface in TUI status bar | `arboard 3.6.1` is current, MIT OR Apache-2.0, use `default-features = false`; `execute_action` at `app.rs:198-224` verified; theme already has `alert: Color::Yellow` (`theme.rs:15`) — no need to add `theme.warning`; Normal-mode status bar is rendered by `render_status_bar` (`ui.rs:332+`), Detail-mode status bar is inline at `ui.rs:310-329` |
| SAFE-03 (#449) | `relocate.rs:93` surfaces `read_link` failures via eprintln warning (mirrors PR #448 pattern) | `relocate.rs:89-100` verified; canonical warning format confirmed at `lib.rs:687-693`; wrapping `if link_path.is_symlink()` block means `read_link` on a pure file path never runs — the only failure mode is a deleted/corrupted symlink between `is_symlink()` and `read_link()` |

## Project Constraints (from CLAUDE.md)

- **Rust edition 2024**, `rust-version = "1.85.0"`. Resolver v3.
- **Strict clippy:** `cargo clippy --all-targets -- -D warnings` blocks the build. Treat all warnings as errors.
- **`anyhow::Result<T>` everywhere.** `thiserror` is NOT used in this repo — do not introduce it.
- **Co-located unit tests** via `#[cfg(test)] mod tests { }` at end of each `src/*.rs` file.
- **Integration tests** in `crates/tome/tests/cli.rs` (single monolithic file, currently 4485 lines).
- **Unix-only** (`std::os::unix::fs::symlink`). No Windows support.
- **CI runs on `ubuntu-latest` + `macos-latest`** per `.github/workflows/ci.yml`.
- **`cargo deny` enforced** — license allowlist includes MIT, Apache-2.0 (both variants), BSD-2/3-Clause, ISC, MPL-2.0, Zlib, Unicode-3.0/DFS, Unlicense, 0BSD, BSL-1.0, CC0-1.0. `multiple-versions = "warn"` (non-fatal). `unknown-registry = "warn"` (non-fatal).
- **`cargo machete`** detects unused deps — any added workspace dep must be used in `crates/tome/Cargo.toml` or machete fails.
- **Alphabetical ordering** of `[dependencies]` / `[workspace.dependencies]` sections.
- **Non-interactive `cp`/`mv`/`rm`** flags (`-f`, `-rf`) — avoid interactive prompts. Applies to any shell commands the plan tasks might run.
- **`make release` is user-triggered** — do NOT run `cargo dist init`, `make release`, or push git tags.

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `arboard` | 3.6.1 (latest 2025-08-23) | Cross-platform clipboard (macOS/Linux text set) | Dominant cross-platform clipboard crate for Rust. Handles X11/Wayland/macOS internally. Maintained by 1Password. License MIT OR Apache-2.0. |
| `anyhow` | 1 (already pinned via workspace) | Error propagation with context | Existing repo convention. `anyhow::anyhow!()` + `.with_context()` used throughout. |
| `console` | 0.16 (already in workspace) | ANSI color output via `.yellow()`, `.green()`, `.bold()` | Existing repo convention. Used for `✓`/`⚠` glyphs across `lib.rs`, `remove.rs`, `reassign.rs`. |
| `ratatui` | 0.30 (already in workspace) | TUI rendering | Already in use. No new ratatui features needed; `Line::from(vec![Span::styled(...)])` is the existing pattern. |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `tempfile` | 3 (already dev-dep) | `TempDir` for filesystem fixtures | SAFE-01 unit + integration tests; SAFE-03 unit test. |
| `assert_cmd` | 2 (already dev-dep) | Run `tome` binary + assert on stdout/stderr/exit | SAFE-01 integration test only (`tests/cli.rs`). |
| `predicates` | 3 (already dev-dep) | `predicate::str::contains(...)` assertions on stderr | SAFE-01 integration test (assert `⚠` marker + `remove completed with` substring in stderr). |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `arboard` | Hand-rolled `wl-copy`/`xclip`/`pbcopy` dispatch via `Command::new` | ~200 LoC of fallback chain + stdin piping. Maintenance burden is the whole point of D-07. Rejected per CONTEXT.md. |
| `arboard` default features | `arboard` with `default-features = false` | **Strongly recommend `default-features = false`** — the `image-data` default feature pulls `image` 0.25 which has ~15+ transitive deps. Text-only use case doesn't need it. |
| `gag` for stderr capture | Observable side-effect assertion | Per D-20, prefer side-effect (`source_path.is_none()`). `gag` is not currently a dev-dep and adding it for one test is over-scoped. |

**Installation:**

```toml
# Root Cargo.toml [workspace.dependencies] — insert alphabetically after `anyhow`, before `clap`
arboard = { version = "3", default-features = false }

# crates/tome/Cargo.toml [dependencies] — insert alphabetically after `anyhow.workspace = true`
arboard = { workspace = true }
```

**Version verification:** `arboard` latest stable is **3.6.1** (published 2025-08-23) per crates.io. Loose pin `"3"` per repo convention (see `anyhow = "1"`, `clap = "4"`, `serde = "1"` in current `Cargo.toml`).

### arboard specifics (HIGH confidence — verified from upstream `Cargo.toml` + README)

- **License:** `MIT OR Apache-2.0` (compliant with `deny.toml` allowlist).
- **Default features:** `["image-data"]`. The `image-data` feature pulls `image` crate + `objc2-core-graphics` + `objc2-core-foundation` + `windows-sys` + `core-graphics`. For text-only tome use, set `default-features = false` to avoid all these.
- **Linux transitives (text-only, with `default-features = false`):** `log`, `x11rb`, `parking_lot`, `percent-encoding`. ~4 direct + ~10 transitive. All MIT/Apache-2.0.
- **Wayland support:** Off by default. Tome does NOT need to enable `wayland-data-control` — arboard's default X11 path works on both X11 and XWayland. Users on pure Wayland compositors without XWayland are a minority; if someone reports it, adding the feature flag is a one-line follow-up.
- **Error type — `arboard::Error` `Display` output** (verified verbatim from upstream `src/common.rs`):
  - `ContentNotAvailable`: *"The clipboard contents were not available in the requested format or the clipboard is empty."*
  - `ClipboardNotSupported`: *"The selected clipboard is not supported with the current system configuration."*
  - `ClipboardOccupied`: *"The native clipboard is not accessible due to being held by another party."*
  - `ConversionFailure`: *"The image or the text that was about the be transferred to/from the clipboard could not be converted to the appropriate format."*
  - `Unknown`: *"Unknown error while interacting with the clipboard: {description}"*
- **SSH / headless behavior:** `Clipboard::new()` returns `Error::ClipboardNotSupported` when neither X11 nor Wayland is available. The error surfaces cleanly via `?` — format into `status_message` with `format!("⚠ Could not copy: {e}")` and it reads as `"⚠ Could not copy: The selected clipboard is not supported with the current system configuration."` — acceptable UX for a fallback.
- **Test hook / mock feature flag:** **None.** arboard does not expose a test-time stub. Per D-19, skip the forced-failure unit test in favor of the success-path test + CI matrix coverage. Do NOT introduce an abstraction trait per D-17.

### xdg-open specifics (HIGH confidence)

- Standard binary on Linux desktops (GNOME, KDE, XFCE). De-facto standard.
- Missing behavior: when `xdg-open` is not installed, `Command::new("xdg-open").spawn()` returns `Err(io::Error { kind: NotFound })`. Error surfaces via `?` and will be rendered into `status_message`.
- **`ubuntu-latest` GitHub runner does NOT pre-install `xdg-utils`** (verified 2026-04 via Ubuntu 24.04 README). This means the CI matrix exercises the `cfg!` *compile* branch on Linux but does NOT actually invoke `xdg-open` at runtime. This is fine because D-19 does not call `execute_action` against the real binary — the tests exercise `status_message` lifecycle only.
- Wayland-vs-X11 behavior: `xdg-open` delegates to `gio open`, `gnome-open`, `kde-open5`, etc., which in turn use the xdg-desktop-portal on modern systems. tome does not need to care — the binary handles desktop env detection.

## Architecture Patterns

### Recommended Project Structure (unchanged — minor edits only)

```
crates/tome/
├── src/
│   ├── remove.rs       # +RemoveFailure struct + FailureKind enum, modify 4 loops, drop in-loop eprintln
│   ├── lib.rs          # Modify Command::Remove handler (lines 362-414)
│   ├── browse/
│   │   ├── app.rs      # +status_message field, rewrite execute_action, clear status_message in handle_key
│   │   ├── ui.rs       # Conditional status-bar render (2 sites: render_status_bar + render_detail's inline block)
│   │   └── theme.rs    # (no change — reuse theme.alert for yellow; theme.accent for green)
│   └── relocate.rs     # Line 93: .ok() → explicit match with eprintln
└── tests/cli.rs        # +1 integration test for tome remove partial-failure
```

### Pattern 1: Aggregated Partial-Failure Struct (new for SAFE-01)

**What:** Per-loop failures are pushed into `Vec<RemoveFailure>` instead of `eprintln!`'d inline. Caller groups + surfaces.

**When to use:** Destructive commands with multiple independent cleanup loops where per-loop errors are recoverable (the loop continues) but the command-level outcome is "partial failure" that must not read as success.

**Example** (extrapolated from CONTEXT.md D-01/D-02/D-03 + existing `remove.rs` shape):

```rust
// remove.rs — additive types
#[derive(Debug)]
pub(crate) enum FailureKind {
    Symlink,        // distribution-dir symlinks (step 1)
    LibraryDir,     // local library directories (step 2a)
    LibrarySymlink, // managed-skill library symlinks (step 2b)
    GitCache,       // git repo cache (step 4)
}

#[derive(Debug)]
pub(crate) struct RemoveFailure {
    pub path: PathBuf,
    pub op: FailureKind,
    pub error: std::io::Error,
}

pub(crate) struct RemoveResult {
    pub symlinks_removed: usize,
    pub library_entries_removed: usize,
    pub git_cache_removed: bool, // drop #[allow(dead_code)] per D-02
    pub failures: Vec<RemoveFailure>,
}

// Inside execute(): replace each eprintln! with:
//   Err(e) => result.failures.push(RemoveFailure {
//       path: symlink.clone(),
//       op: FailureKind::Symlink,
//       error: e,
//   }),
```

**Caller pattern** in `lib.rs::Command::Remove` after line 413:

```rust
println!("\n{} Removed directory '{}': {} library entries, {} symlinks", ...);

if !result.failures.is_empty() {
    let k = result.failures.len();
    println!(
        "{} {} operations failed — run `{}`:",
        style("⚠").yellow(),
        k,
        style("tome doctor").bold(),
    );
    // Group by FailureKind, then print "  {kind-label} ({count}):\n    {path}: {err}" per group
    // ...
    return Err(anyhow::anyhow!("remove completed with {k} failures"));
}
```

### Pattern 2: eprintln warning with anonymous `{e}` interpolation (SAFE-03, verified)

**What:** Replace `.ok()` with explicit match; on error, print to stderr and return None.

**Reference pattern from PR #448** (`lib.rs:687-693` — the verified canonical shape):

```rust
match git::read_head_sha(cache_dir) {
    Ok(sha) => Some(sha),
    Err(e) => {
        if !quiet {
            eprintln!(
                "warning: could not read HEAD sha for '{}' cache at {}: {e}",
                name,
                cache_dir.display()
            );
        }
        None
    }
}
```

**D-13's target shape** (matches PR #448 vocabulary exactly — no `quiet` gate per D-13):

```rust
// relocate.rs:89-100 — replace the current block
let source_path = if entry.managed {
    let link_path = old_library_dir.join(name.as_str());
    if link_path.is_symlink() {
        match std::fs::read_link(&link_path) {
            Ok(raw_target) => Some(resolve_symlink_target(&link_path, &raw_target)),
            Err(e) => {
                eprintln!(
                    "warning: could not read symlink at {}: {e}",
                    link_path.display()
                );
                None
            }
        }
    } else {
        None
    }
} else {
    None
};
```

Note: Current code has `raw_target.map(|t| resolve_symlink_target(&link_path, &t))` which is a `Option::map` on the `.ok()`. The explicit match flattens that to a single expression — cleaner and preserves behavior.

### Pattern 3: TUI Status-Bar Feedback (new for SAFE-02)

**What:** Ephemeral message shown in the status bar after an action, cleared on next keypress.

**Reference code:** There is no existing pattern — `status_message` is a brand-new concept in the codebase. Closest analogue is the `?` help-overlay's "any-key-dismisses" pattern at `app.rs:125-128`:

```rust
Mode::Help => {
    // Any key dismisses help overlay
    self.mode = self.previous_mode;
}
```

**Target shape for D-09/D-10:**

```rust
// app.rs App struct (append after line 88)
pub status_message: Option<String>,

// App::new() (append after line 112, before apply_sort)
status_message: None,

// handle_key top-of-function (line 119-129) — insert BEFORE the mode match
pub fn handle_key(&mut self, key: KeyEvent) {
    self.status_message = None;
    match self.mode { ... }
}

// execute_action rewrite (lines 198-224)
fn execute_action(&mut self, action: DetailAction) {
    match action {
        DetailAction::ViewSource => {
            if let Some((_, _, path)) = self.selected_row_meta() {
                let binary = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
                match std::process::Command::new(binary).arg(&path).spawn() {
                    Ok(_) => self.status_message = Some(format!("✓ Opened: {}", path)),
                    Err(e) => self.status_message = Some(format!("⚠ Could not open: {e}")),
                }
            }
        }
        DetailAction::CopyPath => {
            if let Some((_, _, path)) = self.selected_row_meta() {
                match arboard::Clipboard::new().and_then(|mut c| c.set_text(path.clone())) {
                    Ok(()) => self.status_message = Some(format!("✓ Copied: {}", path)),
                    Err(e) => self.status_message = Some(format!("⚠ Could not copy: {e}")),
                }
            }
        }
        DetailAction::Disable | DetailAction::Enable => { self.mode = Mode::Normal; }
        DetailAction::Back => { self.mode = Mode::Normal; }
    }
}
```

**`ui.rs` conditional render (D-11)** — two sites:

1. **Normal mode status bar** is built in `render_status_bar` at `ui.rs:332+`. The function currently assembles `hint_pairs`; the D-11 conditional goes AT THE START of the function:
   ```rust
   if let Some(msg) = &app.status_message {
       let line = Line::from(vec![
           Span::styled(
               format!(" {} ", msg),
               Style::default().fg(theme.accent).bg(theme.status_bar_bg),
           ),
       ]);
       frame.render_widget(Paragraph::new(line), area);
       return;
   }
   ```
   Style color: success `theme.accent` (cyan/green-family), failure `theme.alert` (yellow). Detect by `msg.starts_with('⚠')` or (cleaner) store the kind alongside the message — but D-12 says "styled ✓" / "styled ⚠" so simple startsWith-check is fine and avoids API churn.

2. **Detail mode status bar** is inline at `ui.rs:310-329`. Apply the same conditional before the existing `Line::from(vec![Span::styled(" Detail ", ...), ...])` assembly.

### Anti-Patterns to Avoid

- **Do NOT introduce `trait Opener` / `trait ClipboardBackend`** — D-17 explicitly rules this out. The `cfg!` branch is 4 lines; abstraction cost exceeds signal.
- **Do NOT add `#[cfg(target_os = "linux")]` direct tests** — D-19 explicitly rules this out. CI matrix handles platform branch compile coverage; behavior tests are platform-agnostic.
- **Do NOT gate D-13's warning on `!cli.quiet`** — D-13 explicitly forbids this. `relocate.rs::plan()` does not have a `cli` handle.
- **Do NOT touch `theme.rs:114-118`** (env-parse `.ok()` fallback) or `git.rs:69` (unused-variable) — D-14 explicitly forbids.
- **Do NOT add `gag` crate dev-dep for stderr capture** — D-20 prefers observable side-effect. `gag` is currently not in `[dev-dependencies]`.
- **Do NOT keep the in-loop `eprintln!("warning: failed to remove ...")` lines in `remove.rs`** — D-03 explicitly drops them (caller is single source).
- **Do NOT use `arboard` default features** — `default-features = false` avoids the `image` crate + 10+ transitive deps for a text-only use case.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Cross-platform clipboard copy | Fallback chain `pbcopy` → `wl-copy` → `xclip` via `Command` + `Stdio::piped()` + `write_all` | `arboard::Clipboard::new()?.set_text(path)?` | ~200 LoC of brittle piping logic, stdin escaping, no Wayland support. D-07 picks arboard. |
| Platform detection for "open" binary | Runtime env-var inspection (`XDG_CURRENT_DESKTOP`, etc.) | `cfg!(target_os = "macos")` compile-time dispatch | D-08 picks cfg. Compile-time is deterministic; runtime adds no value for this case. |
| Stderr capture in unit tests | Shell out, redirect stderr, diff captured text | Observable side-effect (assert `source_path.is_none()`) per D-20 | Adds `gag` dep + `unsafe` blocks on some platforms; brittle. D-20 prefers behavior-assertion. |
| Typed failure aggregation | Untyped `Vec<(PathBuf, io::Error)>` (issue #413's first proposal) | `Vec<RemoveFailure>` with typed `FailureKind` enum per D-01 | Enables grouped summary, clean test assertions, future `tome doctor` integration. |

**Key insight:** All three SAFE requirements are cases where the repo is one `?` + `match` away from correctness — there is no architectural work. Trust CONTEXT.md's locked decisions and resist the urge to generalize.

## Common Pitfalls

### Pitfall 1: Forgetting `default-features = false` on `arboard`

**What goes wrong:** `cargo deny` may flare `multiple-versions = "warn"` on transitive deps shared with `ratatui`/`crossterm` (e.g., `log`, `parking_lot`). More importantly, build time balloons from pulling `image 0.25` + PNG decoder stack for a feature we don't use (the clipboard image API).

**Why it happens:** Default features are implicit. Copy-pasting `cargo add arboard` yields full defaults.

**How to avoid:** Explicit `arboard = { version = "3", default-features = false }` in `[workspace.dependencies]`. Verify with `cargo tree -p tome -e normal -i arboard` after add.

**Warning signs:** If `cargo build` time increases noticeably or `cargo machete` flags `image`, features are wrong.

### Pitfall 2: `chmod 0o000` fixture leaks into `TempDir::drop`

**What goes wrong:** An integration test that removes permissions but panics or exits before restoring them causes `TempDir` cleanup to fail with "Permission denied" — test output gets confusing diagnostic noise.

**Why it happens:** `TempDir` deletes on drop; if a sub-dir is `0o000` the delete fails.

**How to avoid:** Always `chmod 0o755` BEFORE asserting, like the existing `edge_permission_denied_on_target` test (`tests/cli.rs:2243-2249`):
```rust
std::fs::set_permissions(target, std::fs::Permissions::from_mode(0o000)).unwrap();
let output = env.cmd().arg("sync").output().unwrap();
// RESTORE FIRST — before any assertions
std::fs::set_permissions(target, std::fs::Permissions::from_mode(0o755)).unwrap();
assert!(!output.status.success(), ...);
```

**Warning signs:** Test passes locally but CI cleanup phase complains.

### Pitfall 3: `is_symlink()` + `read_link()` race

**What goes wrong:** In SAFE-03, the current code pattern is `if link_path.is_symlink() { read_link(...).ok() }`. The explicit-match version `match read_link(...)` preserves this two-step pattern; on paper `is_symlink()` already passed so `read_link()` should succeed. But between the two syscalls a race is possible (another process rewrites the symlink). The new `Err(e)` branch catches this.

**Why it happens:** Filesystem syscalls are racy. D-13's whole point is to surface this instead of silently dropping.

**How to avoid:** Not applicable — the whole phase exists to handle this. The warning format mirrors PR #448's pattern.

**Warning signs:** If the test fixture needs to trigger `read_link` failure, the cleanest way is to replace the symlink with a regular file between `is_symlink()` check and `read_link()` call — but that's mid-test racing. Simpler fixture: **never create the symlink at all**, have the `is_symlink()` check fail, and the test ends up in the fallback `else { None }` branch — which means the new error-branch test needs a different fixture: create a symlink with the `is_symlink()` check passing, then remove the target file such that `read_link` itself fails. OR: create a broken symlink that resolves to nothing — `is_symlink()` returns true, `read_link()` returns Ok with the raw (broken) path. Neither naturally triggers `Err`. **Practical approach:** use `std::os::unix::fs::symlink` to create a symlink, then `chmod` its parent dir to `0o000` so `read_link` returns `EACCES`. Remember to restore permissions before drop (Pitfall 2).

### Pitfall 4: Clearing `status_message` on *every* key vs. only on meaningful keys

**What goes wrong:** If `handle_key` clears `status_message` only when dispatching to a non-no-op handler, the message lifecycle becomes mode-dependent. If it clears unconditionally at the top, a user viewing the message can't distinguish "press a key to dismiss" from "this key did something."

**Why it happens:** Two-layer key handling (mode switch + mode-specific match). Unconditional clear at top is simpler.

**How to avoid:** Clear unconditionally at `handle_key` top per D-10. The `?` help-overlay's any-key-dismisses pattern (`app.rs:125-128`) sets the precedent.

**Warning signs:** Unit test for `status_message` lifecycle needs to feed any key (e.g., `KeyEvent` for `Char('h')`) and verify message clears — not just dispatched keys.

### Pitfall 5: Theme has `alert` (yellow) already — don't add `warning`

**What goes wrong:** Introducing `theme.warning` duplicates `theme.alert` (both yellow-family). The theme already has:
- `accent: Color::Cyan` (dark) / `Color::Indexed(30)` (light) — used for "success" highlights per the overall palette
- `alert: Color::Yellow` (dark) / `Color::Indexed(136)` (light) — used for `match_highlight()`, `group_header()` per `theme.rs:99-106`

D-12's "success in `theme.accent`, failure in yellow-family" maps cleanly: `theme.accent` for `✓` messages, `theme.alert` for `⚠` messages. **No new `theme.warning` field needed.**

**Why it happens:** D-12 left it as discretion ("add a `theme.warning` field only if nothing fits"). The `alert` field fits.

**How to avoid:** Reuse `theme.alert` directly. Add a one-line comment if needed: `// theme.alert is already the yellow-family warning color`.

**Warning signs:** If a plan task includes "add `warning: Color` to `Theme` struct," push back.

## Runtime State Inventory

> This phase is a bug-fix refactor, not a rename/migration. No runtime state rewrite needed.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — none of SAFE-01/02/03 change data formats or stored keys | None |
| Live service config | None — no external services | None |
| OS-registered state | None — no OS-level registrations | None |
| Secrets/env vars | None — no env var renames | None |
| Build artifacts | `target/` cache will rebuild when `arboard` is added; no stale artifacts to clean | `cargo build` will handle it |

## Code Examples

Verified patterns from existing codebase and upstream sources.

### Canonical eprintln warning format (from PR #448, `lib.rs:687-693`)

```rust
// Source: crates/tome/src/lib.rs:685-697 (HEAD-sha read fix from PR #448)
match git::read_head_sha(cache_dir) {
    Ok(sha) => Some(sha),
    Err(e) => {
        if !quiet {
            eprintln!(
                "warning: could not read HEAD sha for '{}' cache at {}: {e}",
                name,
                cache_dir.display()
            );
        }
        None
    }
}
```

Key format elements D-13 must mirror: `"warning: could not {verb} at {}: {e}"` — anonymous `{e}` placement, `{path}.display()` for the path, lowercase "warning:" prefix.

### Existing `chmod 0o000` integration test fixture (verified `tests/cli.rs:2230-2256`)

```rust
// Source: crates/tome/tests/cli.rs:2230-2256 (edge_permission_denied_on_target)
#[cfg(unix)]
#[test]
fn edge_permission_denied_on_target() {
    use std::os::unix::fs::PermissionsExt;

    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("my-skill", "local")
        .build();

    // Make target dir unwritable
    let target = env.target_dir("test-tool");
    std::fs::set_permissions(target, std::fs::Permissions::from_mode(0o000)).unwrap();

    // Sync should fail or produce an error
    let output = env.cmd().arg("sync").output().unwrap();

    // Restore permissions so TempDir can clean up
    std::fs::set_permissions(target, std::fs::Permissions::from_mode(0o755)).unwrap();

    // Verify: sync should have failed
    assert!(
        !output.status.success() || !String::from_utf8_lossy(&output.stderr).is_empty(),
        "sync should fail or warn when target is unwritable"
    );
}
```

SAFE-01's integration test follows this shape exactly — the only differences are `"remove"` instead of `"sync"` as the subcommand and asserting the `⚠` marker + `operations failed` substring in stderr.

### Existing `remove_test_env` helper (verified `tests/cli.rs:3173-3188`)

```rust
// Source: crates/tome/tests/cli.rs:3173-3188
fn remove_test_env(tmp: &TempDir, directories_toml: &str) -> PathBuf {
    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    let config_path = tmp.path().join("tome.toml");
    std::fs::write(
        &config_path,
        format!(
            "library_dir = \"{}\"\n{}",
            library_dir.display(),
            directories_toml,
        ),
    )
    .unwrap();
    config_path
}
```

SAFE-01 integration test will extend this fixture or clone it into a "permission-denied variant" that `chmod 0o000`s one of the distribution target dirs after initial `sync`, then runs `remove`.

### Existing remove-test pattern (verified `tests/cli.rs:3218-3240+`)

```rust
// Source: crates/tome/tests/cli.rs:3218+
#[test]
fn test_remove_local_directory() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let target_dir = tmp.path().join("target");
    std::fs::create_dir_all(&target_dir).unwrap();

    remove_test_env(
        &tmp,
        &format!(
            "[directories.local]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n\n\
             [directories.test-target]\npath = \"{}\"\ntype = \"directory\"\nrole = \"target\"\n",
            skills_dir.display(),
            target_dir.display()
        ),
    );

    // First sync to populate library and targets
    tome().args([...]).arg("sync").assert().success();

    // Then remove and assert
    tome().args([...]).arg("remove").arg("local").arg("--force").assert().success();
}
```

### arboard minimal text-set pattern (verified from upstream README)

```rust
// Source: arboard README (1Password/arboard)
use arboard::Clipboard;

let mut clipboard = Clipboard::new()?;  // returns Result<Clipboard, arboard::Error>
clipboard.set_text("Hello, world!")?;   // returns Result<(), arboard::Error>
```

SAFE-02's `execute_action` uses `arboard::Clipboard::new().and_then(|mut c| c.set_text(path))` to chain the two error paths into a single match arm.

### Verify CONTEXT.md line references (drift report)

| CONTEXT.md Ref | Actual Location | Drift | Impact |
|----------------|-----------------|-------|--------|
| `remove.rs:46-51` (RemoveResult) | 46-51 exact | 0 | ✅ Match |
| `remove.rs:181-265` execute body | 181-265 exact | 0 | ✅ Match |
| `remove.rs:192-204` symlinks loop | 191-204 (opens at 191) | −1 | ✅ Same block |
| `remove.rs:207-231` library dirs loop | 206-231 (opens at 206) | −1 | ✅ Same block |
| `remove.rs:241-253` git cache | 240-253 | −1 | ✅ Same block |
| `remove.rs:267-404` tests | End of file; tests mod starts at unknown line but file ends at 404 | varies | ✅ Tests mod exists |
| `lib.rs:362-414` Command::Remove | 362-414 exact | 0 | ✅ Match |
| `browse/app.rs:41-61` DetailAction | 41-61 exact | 0 | ✅ Match |
| `browse/app.rs:71-89` App struct | 71-89 exact | 0 | ✅ Match |
| `browse/app.rs:91-117` App::new | 91-117 exact | 0 | ✅ Match |
| `browse/app.rs:119-163` handle_key | 119-163 exact | 0 | ✅ Match |
| `browse/app.rs:198-225` execute_action | 198-224 (ends at 224, not 225) | +1 | ✅ Same block |
| `browse/ui.rs:190-192` Line::from(spans) | 180-192 (highlight_name fn) | ~−10 | Minor — the referenced `Line::from(spans)` line IS at 191, but it's the final line of the `highlight_name` helper, NOT a status-bar line. **CONTEXT.md's ref appears to be slightly mistargeted** — the actual status-bar render sites are `render_status_bar` (line 332+) for Normal mode and inline at `ui.rs:310-328` for Detail mode. Planner should target these locations. |
| `browse/ui.rs:197-200` Detail-mode layout | 197-204 exact | 0 | ✅ Match (layout split; actual status bar inline at 310-329) |
| `browse/theme.rs:21-27` Theme colors | 12-34 (struct fields), 21-27 partial middle | small | ✅ Struct lives at 11-34; fields 14-33 |
| `browse/theme.rs:115-117` `.ok()` env parse | 114-118 (is_light_terminal fn body) | +2 | ✅ Same fn |
| `relocate.rs:93` `.ok()` site | 93 exact | 0 | ✅ Match |
| `relocate.rs:477-818` tests | Tests mod starts at 477; file ends at 818 | 0 | ✅ Match |

**Summary:** Only **one drift matters**: CONTEXT.md's `ui.rs:190-200` pointer for the bottom-bar line assembly is actually inside `highlight_name` (a fuzzy-match helper) — the real D-11 edit sites are `render_status_bar` at 332+ and the inline `Line::from(...)` at 310-329. Planner should use the locations identified above, not the exact line numbers from CONTEXT.md.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `sh -c "echo -n '${path}' | pbcopy"` | `arboard::Clipboard::new()?.set_text(path)?` | — (this phase) | Eliminates `sh -c` command-injection surface; cross-platform Linux support. |
| `Command::new("open")` (macOS-only) | `cfg!(target_os = "macos") ? "open" : "xdg-open"` | — (this phase) | Linux TUI action works; failures surface. |
| `let _ = Command::new(...).spawn();` (silent drop) | `match Command::new(...).spawn() { Ok => ..., Err(e) => self.status_message = Some(...) }` | — (this phase) | Users see success + failure in status bar. |
| `std::fs::read_link(..).ok()` (silent drop) | Explicit `match` + `eprintln!("warning: ...")` | — (this phase, mirrors PR #448 2026-04-22) | Failures surface to stderr; `tome relocate` no longer records fake "no provenance". |
| In-loop `eprintln!("warning: ...")` during partial-failure loops | Aggregate into `Vec<RemoveFailure>`; caller emits single grouped summary | — (this phase) | Single source of user-facing output; exit ≠ 0 on partial failure. |

**Deprecated/outdated:**
- `sh -c` subshelling for clipboard — security footgun, rejected.
- `#[allow(dead_code)]` on `git_cache_removed` — unwrapped per D-02.
- Silent `.ok()` + `None` fallback on destructive-op IO errors — explicit warn + continue is the new norm (PR #448 established, SAFE-03 extends).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo` | Build | ✓ | Rust 1.85.0+ (workspace rust-version) | — |
| `arboard` (new crate) | SAFE-02 clipboard | ✓ (crates.io) | 3.6.1 (2025-08-23) | — |
| `xdg-open` (Linux) | SAFE-02 runtime on Linux | ✗ on `ubuntu-latest` CI runners (not pre-installed) | — | Error surfaces to `status_message` as "xdg-open not found" — acceptable UX; CI does not actually invoke the binary (D-19 tests are platform-agnostic) |
| `open` (macOS) | SAFE-02 runtime on macOS | ✓ (built into macOS) | — | — |
| `gag` crate | SAFE-03 test stderr capture (optional) | ✗ (not in dev-deps) | — | Use observable side-effect assertion (`source_path.is_none()`) per D-20 |

**Missing dependencies with no fallback:** None — all phase requirements have working paths.

**Missing dependencies with fallback:**
- `xdg-open` on CI: fallback is "unit tests do not call execute_action against the real binary" — D-17/D-19 already handle this.
- `gag` dev-dep: fallback is observable-side-effect test per D-20.

## Open Questions

1. **Does CONTEXT.md's `ui.rs:190-200` refer to the wrong lines?**
   - What we know: Lines 180-192 are `highlight_name` (fuzzy match helper). The actual status-bar render sites are `render_status_bar` (line 332+) and the Detail-mode inline block (lines 310-329).
   - What's unclear: Whether CONTEXT.md's author intended these or something else.
   - Recommendation: Planner should target the two real status-bar sites. Plan tasks should include both locations explicitly.

2. **Should `FailureKind` `Display` use "Distribution symlinks" or "Symlinks"?**
   - What we know: D-05's example uses `"Distribution symlinks (1):"` as the group header.
   - What's unclear: Whether the enum's `Display` impl does the labeling or the caller does it inline.
   - Recommendation: Claude's Discretion per CONTEXT.md — default to a caller-side `match failure.op { FailureKind::Symlink => "Distribution symlinks", ... }` helper local to `lib.rs`, keep `FailureKind` `Debug`-only.

3. **Does `arboard::Clipboard::new()?.set_text(path)?` work at `App` method level?**
   - What we know: `arboard::Clipboard` is `!Send` on some platforms (Linux X11 holds an internal `Rc`).
   - What's unclear: Whether the TUI event loop holds the `App` across any await/thread boundary.
   - Recommendation: LOW risk — the `execute_action` method is synchronous and runs in the main thread per the current ratatui event loop pattern. If a clippy warning about `!Send` appears, gate the `Clipboard::new()` scope to the function body (which it already is).

## Sources

### Primary (HIGH confidence)

- **arboard crates.io API** — https://crates.io/api/v1/crates/arboard — current version 3.6.1, license MIT OR Apache-2.0, published 2025-08-23
- **arboard upstream Cargo.toml** — https://raw.githubusercontent.com/1Password/arboard/master/Cargo.toml — features, target-specific deps, default feature list
- **arboard upstream README** — https://github.com/1Password/arboard — Wayland support notes, SetExtLinux::wait, Clipboard::new() example
- **arboard Error Display** — src/common.rs in the repo — each variant's Display output string
- **tome internal references** (verified by `Read`):
  - `crates/tome/src/remove.rs:40-253` — `RemoveResult` struct + `execute` fn with four partial-failure loops
  - `crates/tome/src/lib.rs:362-414` — `Command::Remove` handler
  - `crates/tome/src/lib.rs:685-697` — canonical PR #448 eprintln warning pattern (reference for SAFE-03)
  - `crates/tome/src/browse/app.rs:40-224` — `DetailAction`, `App` struct, `handle_key`, `execute_action`
  - `crates/tome/src/browse/ui.rs:15-329` — render flow, status bar sites (normal: 332+, detail: 310-329)
  - `crates/tome/src/browse/theme.rs:11-34` — Theme struct with existing `accent`/`alert` fields
  - `crates/tome/src/relocate.rs:89-100` — SAFE-03 target block with `.ok()` at line 93
  - `crates/tome/tests/cli.rs:2230-2256` — `chmod 0o000` fixture pattern (integration test template)
  - `crates/tome/tests/cli.rs:3171-3240+` — existing `tome remove` integration tests + `remove_test_env` helper
  - `Cargo.toml` workspace file — dep list, alphabetical ordering, `default-features = false` is NOT currently used for any dep (ratatui, crossterm, nucleo-matcher use workspace defaults)
  - `.github/workflows/ci.yml` — `ubuntu-latest` + `macos-latest` matrix, clippy `-D warnings`, cargo-deny, cargo-machete
  - `deny.toml` — license allowlist (MIT/Apache/BSD/ISC/MPL2/Zlib/Unicode/Unlicense/0BSD/BSL/CC0), `multiple-versions = "warn"`
  - `CHANGELOG.md` lines 1-47 — Unreleased section is bare-minimum; v0.7.0 entry shows format for this release style (uses `### Fixed`, `### Added`, `### Changed` subsections with `(#123)` issue refs)

### Secondary (MEDIUM confidence)

- **GitHub runner-images Ubuntu 24.04 README** — confirmed `xdg-utils` is NOT pre-installed on `ubuntu-latest` runners. Source: https://raw.githubusercontent.com/actions/runner-images/main/images/ubuntu/Ubuntu2404-Readme.md (search for "xdg" returns no match)
- **gh CLI output** for issues #413, #414, #449 — body text verified verbatim against CONTEXT.md references. (Issue "#417" referenced in #449 does NOT exist in the repo; the sibling-pattern fix landed as PR #448 commit d6e9080 closing #415, #417, #418.)

### Tertiary (LOW confidence)

- None — all findings verified against either upstream source or local codebase.

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — `arboard` verified from crates.io + upstream Cargo.toml; all other deps already in workspace
- Architecture: HIGH — all file:line refs verified; one CONTEXT.md reference (`ui.rs:190-200`) identified as slightly mistargeted; corrected locations provided
- Pitfalls: HIGH — drawn from existing test fixtures, upstream arboard behavior docs, and CONTEXT.md D-03/D-10/D-14 guardrails
- Tests: HIGH — existing `chmod 0o000` pattern at `tests/cli.rs:2232+` is a direct template for SAFE-01 integration test; stderr capture path correctly identified as "use observable side-effect"
- CI/Deny: HIGH — `.github/workflows/ci.yml` and `deny.toml` read directly; `xdg-utils` not in ubuntu-latest verified via upstream README

**Research date:** 2026-04-24
**Valid until:** 2026-05-24 (30 days — arboard major versions are infrequent; CI runner images rotate monthly but xdg-utils absence is long-standing)

---

*Phase: 08-safety-refactors-partial-failure-visibility-cross-platform*
*Research gathered: 2026-04-24 (Claude Opus 4.7, research mode)*
