# Phase 25: Rust core extraction + Tauri integration spike - Pattern Map

**Mapped:** 2026-05-25
**Files analyzed:** 11 (5 NEW, 6 MODIFY) + the net-new `tome-desktop` scaffold (treated as one unit)
**Analogs found:** 8 / 9 in-repo Rust files (the Tauri scaffold + TS frontend have NO in-repo analog — flagged explicitly)

> **Scope note:** This phase is architectural plumbing, not feature work. Most structured types already exist (`StatusReport`, `RemovePlan`, `SkillEntry`). The two hardest patterns — *typed-sentinel-through-anyhow downcast* and *plan/render/execute* — are already live in-repo. The high-value output here is concrete current-shape excerpts (derives, serde attrs, presenter shape, downcast site) so the planner can diff precisely. The Tauri/TS side is genuinely net-new; do not force a misleading analog onto it.

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/tome/src/progress.rs` (NEW) | trait + event vocabulary (domain) | event-driven | `lint.rs` (LintFailed marker type pattern) + the `spinner()`/`sp.finish_and_clear()` sites in `lib.rs` | role-match (no existing trait-as-injection-point; sink call sites exist as `println!`/spinner smell) |
| `crates/tome-desktop/src/error.rs` → `TomeError`/`ErrorCode`/`From<anyhow::Error>` (NEW) | error boundary mapper | transform (request-response) | `main.rs` lines 36-43 (downcast at boundary) + `lint.rs::LintFailed` + `lint.rs::lint_failed_downcast_through_anyhow` test | **exact** (pattern is already proven in-repo; D-13/D-14 generalize it) |
| `crates/tome/src/manifest.rs` (MODIFY — `source_name: Option<DirectoryName>` → `ownership: SkillOwnership`) | model / serde migration | transform (serde) | itself — current `SkillEntry` + `#[serde(default, skip_serializing_if)]` + the `deserialize_old_shape_*` tests | **exact** (in-place evolution of an existing migration-tolerant type) |
| `crates/tome/src/status.rs` (MODIFY — `+#[cfg_attr(feature="bindings", derive(specta::Type))]`) | model (cross-boundary type) | request-response | itself — current `#[derive(serde::Serialize)]` on `StatusReport`/`DirectoryStatus`/`CountOrError` | **exact** |
| `crates/tome/src/remove.rs` (MODIFY — specta gate; `io::Error` field gotcha) | plan model (cross-boundary type) | transform | itself — current `RemovePlan` / `RemoveFailure { error: std::io::Error }` | **exact** (with a known blocker — see Pitfall A) |
| `crates/tome/src/lib.rs` (MODIFY — decompose `run()`/`cmd_*` into thin presenters; thread `sink` + `cancel`) | CLI presenter layer | request-response | itself — current `cmd_status`/`cmd_sync` presenters + `sync()` signature + `SyncOptions` | **exact** (the decomposition keeps the existing presenter shape; do NOT create a `presenters/` module — D-Discretion) |
| `crates/tome/Cargo.toml` (MODIFY — `+specta` optional dep, `+bindings` feature) | config | — | itself — existing `[features] test-support = []` block | role-match (only an empty feature precedent exists; no optional-dep precedent) |
| `Cargo.toml` (workspace MODIFY — pick up `tome-desktop`; cargo-dist opt-out) | config | — | itself — `members = ["crates/*"]` glob + `[workspace.metadata.dist]` | role-match (glob already auto-includes new crate; the *new* work is the dist opt-out) |
| `crates/tome-desktop/src/{main.rs,commands.rs,sink.rs}` + `build.rs` + `tauri.conf.json` + `src/bin/gen-bindings.rs` (NEW) | Tauri IPC shell + sinks + bindings export | request-response + event-driven | **NONE in this repo** | **NO ANALOG** — net-new; see "No Analog Found" |
| `crates/tome-desktop/ui/**` (NEW — React/Solid/Svelte spikes + committed `bindings.ts`) | frontend | request-response | **NONE in this repo** | **NO ANALOG** — net-new; see "No Analog Found" |

---

## Pattern Assignments

### `crates/tome-desktop/src/error.rs` — `TomeError` boundary (CORE-05, D-13/14/16)

**Analog:** `crates/tome/src/main.rs` (downcast-at-boundary) + `crates/tome/src/lint.rs` (sentinel marker type + round-trip test). This is the single most important "copy this exactly" pattern in the phase — the repo already proves it works.

**The proven downcast-at-boundary site** (`main.rs:31-46`) — D-13/D-14 generalize this:
```rust
match tome::run(cli) {
    Ok(()) => ExitCode::SUCCESS,
    Err(e) => {
        // HARD-04: typed exit-code mapping via downcast.
        if let Some(lint_failed) = e.downcast_ref::<tome::LintFailed>() {
            eprintln!("error: {lint_failed}");
            return ExitCode::FAILURE;
        }
        if let Some(migration_failed) = e.downcast_ref::<tome::MigrationPartialOrFailed>() {
            eprintln!("error: {migration_failed}");
            return ExitCode::FAILURE;
        }
        eprintln!("error: {e:#}");
        ExitCode::FAILURE
    }
}
```

**The current sentinel marker type** (`lint.rs:21-32`) — note: hand-rolled `Display` + `std::error::Error`, NOT `thiserror` yet. D-14's `DomainErrorKind` should use `thiserror` (new dep), but this is the existing idiom it generalizes:
```rust
#[derive(Debug)]
pub struct LintFailed {
    pub violations: usize,
}
impl std::fmt::Display for LintFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "lint failed: {} violation(s)", self.violations)
    }
}
impl std::error::Error for LintFailed {}
```

**The round-trip test to mirror** (`lint.rs:560-567`) — copy this shape for `tome-desktop`'s `error::tests` (Wave 0 gap, per RESEARCH test map):
```rust
#[test]
fn lint_failed_downcast_through_anyhow() {
    let err: anyhow::Error = LintFailed { violations: 7 }.into();
    let recovered = err.downcast_ref::<LintFailed>();
    assert!(recovered.is_some(), "anyhow downcast must round-trip");
    assert_eq!(recovered.unwrap().violations, 7);
}
```

**How it bubbles today** (`lib.rs:606-613`) — the `anyhow::bail!(marker)` shape the domain uses to attach a sentinel:
```rust
if report.has_errors() {
    anyhow::bail!(lint::LintFailed { violations: report.error_count() });
}
```

**Planner deltas vs analog:**
- D-14 uses `thiserror` (NEW dep) for `enum DomainErrorKind` — the existing `LintFailed` hand-rolls `Display`/`Error`. Both are downcastable; thiserror is the recommended target for the new enum.
- The boundary lives in `tome-desktop`, not `main.rs` — but the `From<anyhow::Error> for TomeError` impl iterates `err.chain()` + `downcast_ref` (RESEARCH Code Examples, lines 408-423) rather than the flat two-`if-let` shape in `main.rs:36-43`, because a sentinel may sit *under* further `.context()` calls.
- `LintFailed`/`MigrationPartialOrFailed` stay as-is (CLI exit-code mapping); they are NOT replaced by `DomainErrorKind`.

---

### `crates/tome/src/manifest.rs` — `SkillEntry` → `SkillOwnership` migration (CORE-01 / #542 / D-08)

**Analog:** itself. This is an in-place evolution of an existing migration-tolerant type. The migration mechanism (`#[serde(default, skip_serializing_if)]` round-trip tolerance) is already load-bearing — preserve it.

**Current `SkillEntry` shape** (`manifest.rs:125-157`) — the `source_name` + `previous_source` pair is what D-08 lifts into the enum:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEntry {
    pub source_path: PathBuf,
    /// `None` if the skill is **Unowned** (source removed from tome.toml, library copy preserved per LIB-04).
    /// Old manifests with `"source_name": "foo"` parse as `Some(DirectoryName::new("foo")?)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_name: Option<DirectoryName>,
    /// Last directory that owned this skill before transition to Unowned. Per D-C1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_source: Option<DirectoryName>,
    pub content_hash: ContentHash,
    pub synced_at: String,
    #[serde(default)]
    pub managed: bool,
}
```

**The Owned/Unowned semantics already exist as two constructors** (`manifest.rs:159-208`) — `new()` sets `source_name: Some(_), previous_source: None`; `new_unowned()` sets `source_name: None, previous_source`. D-08 collapses these two states into one enum:
```rust
pub fn new(source_path, source_name, content_hash, managed) -> Self {
    Self { source_name: Some(source_name), previous_source: None, /* ... */ }
}
pub fn new_unowned(source_path, content_hash, managed, previous_source) -> Self {
    Self { source_name: None, previous_source, /* ... */ }
}
```

**Existing migration tests to mirror for the new enum shape** (`manifest.rs:646-721`) — these are the regression gate; add equivalents for `Owned`/`Unowned`:
- `deserialize_old_shape_with_source_name_string` — `"source_name":"foo"` → `Some(...)`
- `deserialize_new_shape_with_null_source_name` — `"source_name":null` → `None`
- `deserialize_new_shape_missing_source_name` — key absent → `None`
- `serialize_unowned_entry_omits_source_name_key` — round-trip omission
- `deserialize_old_shape_without_previous_source_key` — backward tolerance

**Planner deltas vs analog (from RESEARCH Code Examples lines 428-473):**
- Add `#[serde(from = "SkillEntryRepr")]` on `SkillEntry`; introduce a deserialize-only `SkillEntryRepr` mirroring the *old* flat fields; map `Some(source)` → `Owned{source}`, `None`(+`previous_source`) → `Unowned{last_owner}`.
- New `enum SkillOwnership { Owned { source }, Unowned { last_owner } }` carries `#[serde(tag="kind", rename_all="lowercase")]` for a TS discriminated union, plus `#[cfg_attr(feature="bindings", derive(specta::Type))]`.
- **NAME COLLISION (D-08 corrected):** the enum is `SkillOwnership`, NOT `SkillProvenance` — see Shared Pattern "Naming collision" below.
- `Serialize` stays derived directly on `SkillEntry` (asymmetric serde: deserialize-via-repr, serialize-direct). Verify round-trip in a unit test.

---

### `crates/tome/src/status.rs` — `StatusReport` specta gate (CORE-01/03, D-06)

**Analog:** itself. Already a clean structured type with a `gather()` (pure, returns the report) + `show()` (presenter) split — exactly the CORE-01 target shape. The only change is adding the gated specta derive.

**Current derives** (`status.rs:15,48,71`) — three cross-boundary types, all `#[derive(serde::Serialize)]`, all need the `cfg_attr`:
```rust
#[derive(serde::Serialize)]
pub struct CountOrError { pub count: Option<usize>, #[serde(skip_serializing_if = "Option::is_none")] pub error: Option<String> }

#[derive(serde::Serialize)]
pub struct DirectoryStatus { pub name: String, pub directory_type: String, pub role: crate::config::DirectoryRole, pub role_description: String, pub path: String, pub skill_count: CountOrError, pub warnings: Vec<String>, pub override_applied: bool }

#[derive(serde::Serialize)]
pub struct StatusReport { pub configured: bool, pub library_dir: PathBuf, pub library_count: CountOrError, pub last_sync: Option<String>, pub directories: Vec<DirectoryStatus>, pub unowned: Vec<crate::summary::SkillSummary>, pub health: CountOrError }
```

**The gate to add** (D-06 / RESEARCH Pattern 2) — apply to every type above (and transitively `DirectoryRole`, `SkillSummary`):
```rust
#[derive(serde::Serialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
pub struct StatusReport { /* ... */ }
```

**The presenter split already exists** (`status.rs:95` `gather` / `status.rs:215-224` `show`) — `gather()` is the domain fn `tome-desktop` calls; `show()` is CLI-only. This is the CORE-01 template:
```rust
pub fn gather(config: &Config, paths: &TomePaths) -> Result<StatusReport> { /* pure */ }
pub fn show(config: &Config, paths: &TomePaths, json: bool) -> Result<()> {
    let report = gather(config, paths)?;
    if json { println!("{}", serde_json::to_string_pretty(&report)?); } else { render_status(&report); }
    Ok(())
}
```
**Note:** `StatusReport.directories[].path` is `String` already (not `PathBuf`) — GUI-friendly. `library_dir: PathBuf` will need specta verification (PathBuf → TS `string`).

**Cross-boundary transitive deps to also gate:** `crate::config::DirectoryRole` (enum on `DirectoryStatus.role`) and `crate::summary::SkillSummary` (on `unowned`). The planner must walk the field graph of every command's report and gate every reachable type.

---

### `crates/tome/src/remove.rs` — `RemovePlan` specta gate + `io::Error` blocker (CORE-01/03)

**Analog:** itself. `RemovePlan` is the canonical plan/render/execute type (D-09 cites remove/reassign/relocate/eject as the pattern). It carries the **known specta blocker** flagged in RESEARCH Pitfall 2.

**Current `RemovePlan`** (`remove.rs:26-39`) — currently `pub(crate)` and `#[derive(Debug)]` only; crossing the boundary needs `pub` + `Serialize` + gated `specta::Type`:
```rust
#[derive(Debug)]
pub(crate) struct RemovePlan {
    pub directory_name: DirectoryName,
    pub skills: Vec<String>,
    pub symlinks_to_remove: Vec<PathBuf>,
    pub library_paths: Vec<PathBuf>,
    pub git_cache_path: Option<PathBuf>,
}
```

**THE BLOCKER** (`remove.rs:118-124` and `remove.rs:212-216`) — `io::Error` is neither `Serialize` nor `specta::Type`:
```rust
#[derive(Debug)]
pub(crate) struct RemoveFailure {
    pub path: PathBuf,
    pub kind: FailureKind,
    pub error: std::io::Error,   // ← BLOCKS Serialize + specta::Type derive
}
// identical issue: RemoveSkillFailure { ..., pub error: std::io::Error }  (remove.rs:212-216)
```

**Planner fix (RESEARCH Pitfall 2):** either (a) `#[serde(skip)]` + `#[cfg_attr(feature="bindings", specta(skip))]` the `error` field and add a sibling `error_message: String` from `error.to_string()`, or (b) change the field type to `String` outright. Option (b) is cleaner for a boundary type but is a field-shape change — flag as a deliberate sub-decision (RESEARCH recommends flagging). The `RemoveFailure::new(kind, path, error)` constructor (`remove.rs:139`) and the four `execute()` call sites already have the live `io::Error` in hand, so stringifying at construction is low-cost.

**Also note:** `FailureKind`/`RemoveSkillFailureKind` are simple `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` C-like enums (`remove.rs:63,162`) with compile-time exhaustiveness guards (`ALL` arrays + `const _` asserts at `remove.rs:104-116`) — they specta-derive cleanly as TS string unions; preserve the exhaustiveness guards.

---

### `crates/tome/src/progress.rs` — `ProgressSink` trait + `ProgressEvent` (CORE-04, D-09/10/11/12)

**Analog:** weak. No existing trait-as-injection-point in the crate. Two in-repo anchors:
1. **The marker-type idiom** (`lint.rs::LintFailed`) — for how the crate defines small public types with derives (the new `ProgressEvent`/`SyncStage` enums follow this).
2. **The presentation-in-domain smell that `IndicatifSink` re-homes** — the `spinner()` helper and its `finish_and_clear()` call sites:

**Current `spinner()` helper** (`lib.rs:147-158`) — moves into `IndicatifSink`:
```rust
fn spinner(msg: &str) -> ProgressBar {
    let sp = ProgressBar::new_spinner();
    sp.set_style(ProgressStyle::default_spinner().template("{spinner:.cyan} {msg}").expect("valid template"));
    sp.set_message(msg.to_string());
    sp.enable_steady_tick(std::time::Duration::from_millis(80));
    sp
}
```

**Current call sites inside `sync()`** (`lib.rs:1672-1815`) — these become `sink.emit(ProgressEvent::SyncStage…)` calls; the `IndicatifSink` translates them back to spinners:
```rust
let sp = show_progress.then(|| spinner("Resolving git sources..."));     // → SyncStage::Reconcile
let sp = show_progress.then(|| spinner("Discovering skills..."));         // → SyncStage::Discover
let sp = show_progress.then(|| spinner("Consolidating to library..."));   // → SyncStage::Consolidate
let sp = show_progress.then(|| spinner(&format!("Distributing to {}...", name)));  // → SyncStage::Distribute
// each paired with sp.finish_and_clear()
```

**Other emit-worthy `println!` smell** (`backup.rs:85,103,108,113,171` etc.) — `BackupSnapshot` events; adopt incrementally (D-11 says thread `sync()` + `git::clone`/`backup::*` now, others later).

**Planner targets (RESEARCH Pattern 3 + Code Examples lines 285-296, 511-525):**
- `trait ProgressSink: Send + Sync { fn emit(&self, event: ProgressEvent); }` in `progress.rs`.
- `enum ProgressEvent { SyncStageStarted/Progress/Finished { stage: SyncStage, … }, GitCloneProgress, BackupSnapshot }`; `enum SyncStage { Reconcile, Discover, Consolidate, Distribute, Cleanup, Save }`. Exact members are Claude's Discretion.
- `CancelToken(Arc<AtomicBool>)` newtype (~12 lines, no tokio) co-located in `progress.rs`.
- `IndicatifSink` + `NullSink` live in `lib.rs` next to presenters (D-11). Add a `RecordingSink` test double (RESEARCH Wave 0 gap) — assert event sequence; mirror the `lint_failed_downcast_through_anyhow` test discipline.
- Thread `sink: &dyn ProgressSink, cancel: &CancelToken` through `sync()` and the `git`/`backup` long-ops.

---

### `crates/tome/src/lib.rs` — presenter decomposition (CORE-01, D-GUI-08)

**Analog:** itself. The presenter shape ALREADY EXISTS for several commands — the decomposition extends it, it does not invent it. **Do NOT create a `presenters/` module** (RESEARCH Pattern 1 + CONTEXT Deferred Idea: keep presenters inline to minimize diff and preserve `insta` snapshot bytes).

**The target presenter shape already exists** (`lib.rs:570-572`, `539-567`) — a thin `cmd_*` that calls a domain fn and formats:
```rust
pub(crate) fn cmd_status(config: &Config, paths: &TomePaths, json: bool) -> Result<()> {
    status::show(config, paths, json)        // domain (gather) is one level deeper; cmd_ is the presenter
}

pub(crate) fn cmd_sync(force, no_triage, no_install, config, paths, machine_path, machine_prefs, dry_run, no_input, verbose, quiet) -> Result<()> {
    sync(config, paths, SyncOptions { dry_run, force, no_triage: no_triage || no_input, no_input, no_install, verbose, quiet, machine_path, machine_prefs })
}
```

**The `sync()` core signature to extend** (`lib.rs:1516`) — gains `sink` + `cancel` (D-09/D-12). `SyncOptions<'_>` (the options struct destructured at `lib.rs:1517-1527`) is the natural place to thread them, or as separate args:
```rust
fn sync(config: &Config, paths: &TomePaths, opts: SyncOptions<'_>) -> Result<()> { /* ... */ }
// target: fn sync(config, paths, opts, sink: &dyn ProgressSink, cancel: &CancelToken) -> Result<()>
```

**The 25-command `run()` dispatcher** (`lib.rs:225`, full file 3,101 lines) is the giant `match cli.command` that delegates to the `cmd_*` presenters. The decomposition: where a command's logic is still inline in `run`/`cmd_*`, extract to its module as `fn collect(...) -> Result<XxxReport>` / `fn plan(...) -> Result<XxxPlan>` (mirroring `status::gather` / `remove::plan`).

**Regression gate (RESEARCH):** 130 `assert_cmd` integration tests + 8 `insta` snapshots must stay byte-for-byte identical. Anti-pattern: do NOT change any structured-type field shape "while in there" — only `SkillEntry`→`SkillOwnership` (D-08) changes shape, deliberately.

---

### `crates/tome/Cargo.toml` — `bindings` feature + optional specta (D-06)

**Analog:** the existing `[features]` block (`crates/tome/Cargo.toml:54-59`). Precedent is an *empty* feature (`test-support = []`); there is **no existing optional-dependency precedent** (`dep:` syntax) — this is new.
```toml
[features]
test-support = []        # existing precedent — empty feature, flipped on by a dev-dep self-reference
```

**Planner target (RESEARCH Standard Stack lines 130-137):**
```toml
[features]
bindings = ["dep:specta"]

[dependencies]
specta = { version = "=2.0.0-rc.25", features = ["derive"], optional = true }
```
All workspace deps currently use `.workspace = true`. The exact-pinned `specta` may go straight in the crate manifest (it's tome-desktop-coupled) or be added to `[workspace.dependencies]` and referenced — planner's call; the `=` exact pin is mandatory (specta trio moves in lockstep).

---

### `Cargo.toml` (workspace) — pick up `tome-desktop` + cargo-dist opt-out

**Analog:** the workspace manifest itself (`Cargo.toml:1-3`, `60-78`).

**Members glob already auto-includes the new crate** (`Cargo.toml:3`) — no edit needed for membership:
```toml
[workspace]
resolver = "3"
members = ["crates/*"]    # ← already picks up crates/tome-desktop
```

**The cargo-dist block to guard** (`Cargo.toml:60-78`) — `targets`/`installers`/`publish-jobs` are workspace-level; the NEW work is ensuring `tome-desktop` does NOT become a dist artifact (RESEARCH Pitfall 4):
```toml
[workspace.metadata.dist]
cargo-dist-version = "0.30.3"
targets = ["aarch64-apple-darwin", "x86_64-apple-darwin", "x86_64-unknown-linux-gnu"]
installers = ["homebrew"]
# ...
allow-dirty = ["ci"]
```
**Planner fix:** `crates/tome-desktop` sets `publish = false` and `[package.metadata.dist] dist = false` (A1 — verify the exact key against cargo-dist 0.30.3 at plan time; `cargo dist plan` is self-verifying). **Do NOT hand-edit `release.yml`** (CLAUDE.md: cargo-dist owns it; run `cargo dist init` after metadata changes). Verify `cargo dist plan` lists only the `tome` CLI artifact afterward.

---

## Shared Patterns

### Naming collision — `SkillProvenance` already exists (BLOCKING, D-08 corrected)
**Source:** `crates/tome/src/discover.rs:109-118`
**Apply to:** `manifest.rs` D-08 enum naming
`discover.rs` already defines a **struct** `SkillProvenance` (managed-source registry/version/git-SHA metadata, used at ~10 sites — `discover.rs:111,129,141,170,299,309,514,1219`, threaded through `SkillOrigin::Managed`/`ScanMode`):
```rust
/// Provenance metadata from package manager sources.
#[derive(Debug, Clone)]
pub struct SkillProvenance {
    pub registry_id: String,
    pub version: Option<String>,
    pub git_commit_sha: Option<String>,
}
```
D-08's new **enum** must therefore be named **`SkillOwnership`** (CONTEXT D-08 already locks this: "Name corrected … renamed to SkillOwnership"). Do NOT ship two `SkillProvenance` types. (The original RESEARCH body still uses the old `SkillProvenance` name in some lines — CONTEXT D-08 is authoritative; use `SkillOwnership`.)

### `anyhow::Result` + `.context()` everywhere (the chain is the value)
**Source:** every module (e.g. `manifest.rs:10`, `status.rs:3`, `lib.rs` throughout)
**Apply to:** `TomeError.context` (D-16). The flattened `err.chain().map(|c| c.to_string()).collect::<Vec<_>>()` is what preserves the `error: a: b: c` stderr chain into a GUI details view. `.context()` preserves downcastability — a sentinel survives further `.context()` wrapping (anyhow semantics, proven by `main.rs` downcast). Do NOT refactor the domain off anyhow (D-13: zero refactor, no CLI regression).

### Transparent newtypes under specta (verify in spike)
**Source:** `crates/tome/src/discover.rs:99-107` (custom validating `Deserialize` on `SkillName`), `config` `DirectoryName`, `validation.rs` `ContentHash`
**Apply to:** every cross-boundary type carrying these (`SkillEntry.content_hash`, `SkillOwnership.source: DirectoryName`, etc.)
These use `#[serde(transparent)]` + a **hand-written** `Deserialize` (specta derives off type structure, not serde impl). Spike acceptance check (RESEARCH Pitfall 6 / CONTEXT code_context): confirm `bindings.ts` emits `type SkillName = string`, not a tuple/`unknown`. Fix with `#[specta(transparent)]` if mis-rendered.
```rust
impl<'de> serde::Deserialize<'de> for SkillName {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error> where D: serde::Deserializer<'de> {
        let s = String::deserialize(deserializer)?;
        SkillName::new(s).map_err(serde::de::Error::custom)
    }
}
```

### Compile-time exhaustiveness guards (preserve when adding variants)
**Source:** `remove.rs:79-116` (`FailureKind::ALL` + `_ensure_*_exhaustive` + `const _` assert), `discover.rs:173-178` (ScanMode match sentinel)
**Apply to:** any new enum that the GUI pattern-matches (`ProgressEvent`, `SyncStage`, `ErrorCode`). The repo's convention is a compile-time drift guard so a new variant can't silently drop out of a parallel array. `ErrorCode` is explicitly additive (D-15) — new variants are non-breaking for the GUI default arm, but the in-repo style still favors an exhaustiveness check where a parallel list exists.

### "Structure at the edge" symmetry (D-17)
**Source:** the design itself — `progress.rs` (trait in core, sinks at edges) mirrors the `TomeError` boundary (anyhow in core, `TomeError` at edge).
**Apply to:** code organization. Both keep the domain ergonomic (sync, `anyhow`) and put GUI-facing structure at the boundary (`TauriEventSink`, `TomeError::from`). Read as one idea applied twice, not two separate mechanisms.

---

## No Analog Found

These have **no in-repo Rust analog** — the planner should use RESEARCH.md's verified tauri-specta example patterns (RESEARCH "Code Examples" lines 475-508, Architecture diagram lines 168-220) rather than forcing a CLI analog.

| File | Role | Data Flow | Reason / Source to use instead |
|------|------|-----------|--------------------------------|
| `crates/tome-desktop/src/main.rs` | Tauri `Builder` + `collect_commands!`/`collect_events!` + `make_builder()` + `#[cfg(debug)]` export | event-driven + request-response | No Tauri/IPC code exists in this repo. Use RESEARCH Code Examples lines 475-508 (verified against `specta-rs/tauri-specta examples/app`). Builder constructed in `main.rs` (NOT `build.rs` — Pitfall 1). |
| `crates/tome-desktop/src/bin/gen-bindings.rs` | bindings exporter bin (shares `make_builder()`) | transform | NEW per D-07 correction (build.rs cannot see `#[tauri::command]` fns). No analog; CI runs `cargo run -p tome-desktop --bin gen-bindings` then `git diff --exit-code`. |
| `crates/tome-desktop/src/commands.rs` | `#[tauri::command]` wrappers (inject `TauriEventSink`+`CancelToken`, map `anyhow`→`TomeError`) | request-response | No analog. Pattern from RESEARCH lines 484-489. Each wraps a domain fn (`status::gather`, etc.). |
| `crates/tome-desktop/src/sink.rs` | `TauriEventSink` impl `ProgressSink` over `AppHandle::emit` | event-driven | No analog (the *trait* is in `tome/src/progress.rs`; this impl is GUI-side). `AppHandle` is `Send+Sync`; emit is cross-thread (RESEARCH Pitfall 5). |
| `crates/tome-desktop/build.rs` | `tauri_build::build()` only (NOT specta export) | — | No `build.rs` exists anywhere in repo today (verified: `fd build.rs crates/` → none). Standard Tauri 2 build-dep. |
| `crates/tome-desktop/tauri.conf.json`, `capabilities/main.json`, `icons/` | Tauri config / permissions / assets | — | No analog (no JSON-config-as-app-shell precedent). Minimal capability allowlist: expose only `get_status` this phase (RESEARCH Security Domain). |
| `crates/tome-desktop/ui/**` (React + Solid + Svelte spikes) | frontend (3-way spike) | request-response | No frontend exists in this Rust-only repo. Three sibling `ui-*` dirs share one Rust backend + one `bindings.ts` (RESEARCH Open Question Q-C). Renders real `StatusReport` from the user's `tome_home`. |
| `crates/tome-desktop/ui/src/bindings.ts` | GENERATED + COMMITTED TS bindings | — | No analog. Produced by `gen-bindings`; committed; CI freshness gate via `git diff --exit-code` (D-07). |
| `.planning/research/v1.0-frontend-framework-decision.md` | ADR + scoring table (decision artifact) | — | Planning artifact, not code. Produced by the spike (D-04). |

---

## Metadata

**Analog search scope:** `crates/tome/src/` (40 module files), `Cargo.toml` (workspace), `crates/tome/Cargo.toml` (crate)
**Files scanned (read):** `main.rs`, `manifest.rs`, `status.rs`, `remove.rs`, `discover.rs`, `lint.rs`, `lib.rs` (targeted ranges), `Cargo.toml` ×2
**Confirmed absences:** no `build.rs` anywhere in `crates/`; no `thiserror` dependency yet; no `dep:`-style optional-dependency precedent; no Tauri/frontend/TS code of any kind
**Pattern extraction date:** 2026-05-25
