---
phase: 16-cleanup-message-ux-docs
plan: 05
type: execute
wave: 2
depends_on:
  - 16-01
  - 16-02
files_modified:
  - docs/src/cross-machine-sync.md
  - docs/src/SUMMARY.md
  - crates/tome/src/cli.rs
autonomous: true
requirements:
  - DOC-03

must_haves:
  truths:
    - "New page `docs/src/cross-machine-sync.md` exists; opens with two numbered walkthroughs (Machine A: source-of-truth machine; Machine B: fresh machine bootstrap); followed by reference sections covering `tome.lock` semantics, `auto_install_plugins` consent values, `directory_overrides`, missing-`claude` behaviour, and migrating a v0.9 library on Machine B."
    - "`docs/src/SUMMARY.md` lists the new page between Configuration and Architecture per CONTEXT.md D-DOC03-1."
    - "`tome sync` long-form help (`#[command(long_about = ...)]` on `Command::Sync` in cli.rs) references the cross-machine-sync.md page so users hitting `tome sync --help` see the cross-machine workflow doc reference."
    - "All cross-machine vocabulary is post-supersession: NO `tome adopt` / `tome forget` (Phase 14 D-API-1/-2); migration is `tome migrate-library` (Phase 11 D-01); drift basis is content_hash (Phase 11 D-08)."
  artifacts:
    - path: "docs/src/cross-machine-sync.md"
      provides: "New top-level page documenting library-as-dotfiles workflow end-to-end"
      min_lines: 150
      contains: "Machine A"
    - path: "docs/src/SUMMARY.md"
      provides: "TOC entry linking the new page between Configuration and Architecture"
      contains: "cross-machine-sync.md"
    - path: "crates/tome/src/cli.rs"
      provides: "`#[command(long_about = ...)]` on `Command::Sync` referencing cross-machine-sync.md"
      contains: "cross-machine-sync"
  key_links:
    - from: "docs/src/SUMMARY.md"
      to: "docs/src/cross-machine-sync.md"
      via: "[Cross-machine sync](cross-machine-sync.md) line between Configuration and Architecture entries"
      pattern: "cross-machine-sync.md"
    - from: "crates/tome/src/cli.rs Command::Sync"
      to: "cross-machine-sync.md (relative path or stable URL)"
      via: "long_about attribute string"
      pattern: "long_about|cross-machine-sync"
    - from: "docs/src/architecture.md Library-canonical model section (Plan 16-03)"
      to: "docs/src/cross-machine-sync.md"
      via: "in-prose link from architecture.md"
      pattern: "\\[.*cross-machine.*\\]\\(cross-machine-sync.md\\)"
---

<objective>
Add a new top-level documentation page `docs/src/cross-machine-sync.md` documenting the library-as-dotfiles workflow per DOC-03 + CONTEXT.md D-DOC03-1..-3. Also wire it into the mdbook TOC (`docs/src/SUMMARY.md`) and reference it from `tome sync --help` long description so the page is discoverable both ways.

Page structure (D-DOC03-2 — walkthrough first, reference second):
1. **Two numbered walkthroughs** open the page:
   - Machine A (source-of-truth): `tome init` → curate library → commit `~/.tome/` to dotfiles → push
   - Machine B (fresh machine): install tome → clone dotfiles → `tome sync` → first-time `auto_install_plugins` consent prompt → done
2. **Reference sections** below the walkthroughs:
   - `tome.lock` semantics (Cargo.lock-shaped; what it pins; why it's authoritative on Machine B)
   - `auto_install_plugins` values (Yes / Never / Prompt) + `--no-install` global override
   - `directory_overrides` for cross-machine path remapping (PORT-01..05)
   - What happens when `claude` is missing on Machine B (ADP-02 actionable error; partial-failure exit semantics)
   - Migrating a v0.9 library on Machine B (`tome sync` refuses → `tome migrate-library --dry-run` → `tome migrate-library`)

Tone (per CONTEXT.md `<specifics>`): direct, walkthrough-style, no "production guidance" boilerplate. Reader should be able to skim the walkthroughs and dive into reference sections when something surprises them.

Purpose: closes DOC-03. Library-as-dotfiles is the v0.10 milestone's core value (per PROJECT.md); the page is the user-facing manifestation of that value.

Output: a new `docs/src/cross-machine-sync.md` (~150-250 lines), an updated `docs/src/SUMMARY.md` with the TOC entry, and an updated `crates/tome/src/cli.rs` with `long_about` on `Command::Sync` referencing the page.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/phases/16-cleanup-message-ux-docs/16-CONTEXT.md

@docs/src/SUMMARY.md
@docs/src/configuration.md
@docs/src/commands.md
@crates/tome/src/cli.rs

<interfaces>
Today's `docs/src/SUMMARY.md` (15 lines):

```markdown
# Summary

[Introduction](introduction.md)

- [Commands](commands.md)
- [Configuration](configuration.md)
- [Development Workflow](development-workflow.md)
- [Architecture](architecture.md)
- [Tool Landscape](tool-landscape.md)
- [Frontmatter Compatibility](frontmatter-compatibility.md)
- [Vercel Skills Comparison](vercel-skills-comparison.md)
- [Test Setup](test-setup.md)
- [Roadmap](roadmap.md)
- [API Reference](api-reference.md)
```

Per CONTEXT.md D-DOC03-1, the new entry slots between Configuration and Development Workflow (so reading order: introduction → commands → configuration → cross-machine-sync → development-workflow → architecture → ...). NOTE: CONTEXT.md says "between Configuration and Architecture" but the current SUMMARY has Development Workflow between them; the planner picks position based on the existing TOC. Recommend: insert between Configuration and Architecture so the reading order is `... → Configuration → Cross-machine sync → Architecture → ...`. This means moving `[Architecture]` to come AFTER cross-machine-sync, putting `[Development Workflow]` earlier (near `[Commands]`) — but the simpler placement is BETWEEN existing entries: insert as a new line between `Configuration` and `Development Workflow`. **Planner's call** — pick the placement that yields the cleanest reading order; document the choice in the summary.

Today's `tome sync` clap definition (cli.rs line 158-175):

```rust
/// Discover, consolidate, and distribute skills
#[command(
    after_help = "Examples:\n  tome sync\n  tome sync --dry-run\n  tome sync --force\n  tome sync --no-triage\n  tome sync --no-input\n  tome sync --no-install"
)]
Sync {
    #[arg(short, long)]
    force: bool,
    #[arg(long)]
    no_triage: bool,
    #[arg(long)]
    no_install: bool,
},
```

Per D-DOC03-3 link strategy, add a `long_about` attribute that references the cross-machine-sync.md page. Per CONTEXT.md "Claude's Discretion": link target depends on deploy. Use a relative `docs/src/cross-machine-sync.md` reference that works for users running `--help` inside a clone.

Reference content sources for cross-machine-sync.md (consume these during writing):
- `.planning/phases/13-lockfile-authoritative-sync/13-CONTEXT.md` — `auto_install_plugins` enum values, `--no-install` semantics, RECON-01..05 flow shape
- `.planning/phases/09-cross-machine-path-overrides/` (or equivalent) — `[directory_overrides.<name>]` schema, validation behavior, `(override)` annotation, PORT-01..05 detail (this is shipped v0.9 surface)
- `.planning/phases/12-marketplace-adapter/` — ADP-02 `ClaudeMarketplaceAdapter` missing-claude behavior; the actual error message pattern shipped
- Plan 16-02 SUMMARY (when available) — locked `tome migrate-library` UX wording for the "Migrating a v0.9 library on Machine B" section

The `directory_overrides` example in the page should match the format shipped in v0.9. Quick check: `rg -n 'directory_overrides' docs/src/configuration.md` to see whether configuration.md already documents the schema; if so, link to it rather than re-documenting.
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Write docs/src/cross-machine-sync.md walkthroughs + reference sections</name>
  <files>docs/src/cross-machine-sync.md</files>
  <read_first>
    - .planning/phases/16-cleanup-message-ux-docs/16-CONTEXT.md (D-DOC03-1, D-DOC03-2, D-DOC03-3 — page structure, walkthrough shape, reference section list)
    - docs/src/configuration.md (existing schema docs — link rather than duplicate where possible; especially `directory_overrides`)
    - docs/src/commands.md (existing command docs — page should link to `tome sync`, `tome migrate-library`, `tome init` doc anchors rather than re-documenting flags)
    - .planning/REQUIREMENTS.md (RECON-01..05 + PORT-01..05 + ADP-02 — exact requirement-level guarantees the page should articulate)
    - .planning/phases/13-lockfile-authoritative-sync/13-CONTEXT.md (`auto_install_plugins` enum + the `Auto-install missing plugins on every sync? [Y/n/never]` exact prompt wording)
  </read_first>
  <action>
    Create the new file `docs/src/cross-machine-sync.md` with this structure (target ~150-250 lines):

    ```markdown
    # Cross-machine sync

    tome's library is designed to be a portable, version-controlled artifact —
    you commit `~/.tome/` to your dotfiles and clone it onto every machine you
    work on. This page walks the workflow end-to-end: setting up the source-of-
    truth machine, bootstrapping a fresh machine, and what happens when things
    drift.

    > **Why this matters:** Pre-v0.10, tome's library was a thin layer of
    > symlinks pointing into machine-specific marketplace caches; cloning the
    > library onto a fresh machine was meaningless because the symlink targets
    > didn't exist. v0.10 makes the library a real-directory copy of every
    > skill, with `tome.lock` recording exactly what versions are installed.
    > Now the library is portable.

    ## Walkthrough — Machine A (source of truth)

    Machine A is the machine you use to curate your skill library. You run
    `tome init` here, install plugins, edit local skills, and commit the
    result.

    ```bash
    # 1. Set up tome on Machine A.
    tome init

    # 2. Curate your library — install Claude plugins, add git directories,
    #    enable/disable skills via `tome browse`.
    claude plugin install foo@bar
    tome add my-org/my-skill-repo
    tome sync

    # 3. Commit ~/.tome/ to your dotfiles (or whatever portable storage you
    #    use). The library content is real-directory copies; the manifest
    #    and tome.lock pin exactly what's installed.
    cd ~/.tome
    git init
    git add .
    git commit -m "Initial tome library"
    git remote add origin git@github.com:you/your-dotfiles.git
    git push -u origin main
    ```

    What's in `~/.tome/` after this:
    - `tome.toml` — your portable directory configuration (sources, distribution targets, exclude lists)
    - `library/` — real-directory copies of every skill (managed and local)
    - `.tome-manifest.json` — content hashes + provenance for every library entry
    - `tome.lock` — Cargo.lock-shaped: pinned versions + content hashes for managed skills

    Note: machine-specific preferences (`~/.config/tome/machine.toml`) are
    deliberately NOT in `~/.tome/` — they're per-machine by design. Things
    like `disabled` skills, `auto_install_plugins` consent, and
    `directory_overrides` belong there.

    ## Walkthrough — Machine B (fresh machine)

    Machine B is a fresh machine that's never seen your library. You install
    tome and Claude Code, clone your dotfiles, and run `tome sync`.

    ```bash
    # 1. Install tome (Homebrew or cargo).
    brew install martinp7r/tap/tome
    # or: cargo install tome

    # 2. Install Claude Code (required if your library has Claude plugins).
    # See https://claude.ai/code for instructions.

    # 3. Clone your dotfiles into ~/.tome (or wherever the portable artifact lives).
    git clone git@github.com:you/your-dotfiles.git ~/.tome

    # 4. First sync. tome reads tome.lock and reconciles what's actually
    #    installed against what should be installed.
    tome sync
    ```

    On the first sync, tome detects drift (your machine has zero plugins
    installed; the lockfile expects N) and prompts:

    ```
    Auto-install missing plugins on every sync? [Y/n/never]
    ```

    Your choice persists in `machine.toml::auto_install_plugins`. Pick `Y`
    on a personal machine (auto-install on every sync); pick `never` on a
    locked-down workstation where you want to inspect drift before applying;
    pick `n` to skip this run only.

    Once you confirm, tome shells out to `claude plugin install <plugin>@<marketplace>`
    for every missing plugin, re-discovers the skill content, verifies the
    resulting `content_hash` matches `tome.lock`, and then distributes the
    skills to your configured target directories (`~/.claude/skills`,
    `~/.codex/skills`, etc.). You're set up.

    ## Reference — `tome.lock` semantics

    `tome.lock` is the cross-machine state contract. It's the equivalent of
    Cargo.lock for your skill library: a snapshot of every installed version
    + content hash that tome can use to bring a fresh machine into the same
    state.

    Each managed-skill entry records:
    - `name` — the skill name
    - `version` — the actual installed version (display-only; see drift basis below)
    - `content_hash` — SHA-256 of the skill directory contents
    - `source_name` — the directory in `tome.toml` that owns the skill
    - `previous_source` — the previous owner if the skill has been re-anchored (closes the Phase 13 fork-in-place gap)
    - `registry_id`, `git_commit_sha` — provenance metadata when applicable

    **Drift basis: content_hash, NOT version.** When `tome sync` reconciles
    the lockfile against actually-installed plugins, drift is computed from
    `content_hash(library/<skill>) != lockfile.content_hash`. The version
    string is display-only in the diff output (e.g. `plugin X: 5.0.5 →
    5.0.7`). Because Claude CLI doesn't accept `--version` on `claude plugin
    install`, true version pinning is upstream future work.

    ## Reference — `auto_install_plugins` consent

    `~/.config/tome/machine.toml`:

    ```toml
    auto_install_plugins = "yes"     # auto-install on every sync (CI / personal)
    auto_install_plugins = "never"   # warn-only; never modify; require manual install
    auto_install_plugins = "prompt"  # default; ask once per sync (the unset case)
    ```

    The unset case ("prompt") asks the question once per `tome sync` invocation
    that detects drift. Pick `yes` once and tome remembers; pick `never` and
    you'll see drift warnings on every sync but tome won't touch installed
    plugins.

    `--no-install` is a global flag that overrides the persisted choice for
    the current invocation. Use this when you want to inspect drift on a
    machine where `auto_install_plugins = "yes"` is set:

    ```bash
    tome sync --no-install
    ```

    Mirrors Cargo's `--frozen` / `--locked` semantics — temporary, doesn't
    change the persisted setting.

    ## Reference — `directory_overrides` for path remapping

    Different machines have different home layouts. macOS keeps Claude
    plugins at `~/Library/Application Support/Claude/plugins/cache`; Linux
    keeps them at `~/.claude/plugins/cache`. Your portable `tome.toml` can
    only express one path; `machine.toml::directory_overrides` provides the
    machine-specific remap.

    `~/.config/tome/machine.toml` on Linux:

    ```toml
    [directory_overrides.claude-plugins]
    path = "~/.claude/plugins/cache"
    ```

    `~/.config/tome/machine.toml` on macOS (would override the same name to
    a different path).

    Override application happens at config load (after tilde expansion,
    before validation), so all downstream code sees the canonical post-
    override path. Unknown override directory names emit a typo-target
    stderr warning. Override-induced validation failures are wrapped with
    a distinct error attributing them to `machine.toml` rather than
    `tome.toml`.

    `tome status` and `tome doctor` annotate any directory whose path was
    rewritten by an override with `(override)` so you can tell at a glance
    which paths come from the portable config and which come from the
    machine-local layer.

    See [Configuration](configuration.md) for the full schema.

    ## Reference — what happens when `claude` is missing on Machine B

    The `ClaudeMarketplaceAdapter` shells out to the `claude` binary. If
    `claude` isn't on `PATH`, `tome sync` produces a clear actionable error
    naming the binary:

    ```
    error: `claude` binary not found on PATH
      Why: tome shells out to `claude plugin install`/`update`/`list` for
           Claude marketplace plugins; the binary must be installed before
           reconcile can run.
      Suggestion: install Claude Code from https://claude.ai/code, then
                  re-run `tome sync`.
    ```

    If your library only has git-source skills and no Claude plugins,
    missing `claude` won't break sync — it'll only break if the lockfile
    contains entries the Claude adapter would need to install. A library
    that's purely git-sourced + local is fully portable to a machine
    without Claude installed.

    Vanished plugins (the marketplace removed a plugin you have installed)
    surface as a stderr warning (`plugin X vanished from marketplace Y;
    using preserved library copy`) and `tome sync` continues. Distribution
    still happens from the preserved library copy.

    Adapter `install`/`update` failures aggregate into a `⚠ N install
    operations failed` summary; library distribution still completes for
    skills whose adapter calls succeeded; sync exits non-zero on partial
    install failure.

    ## Reference — migrating a v0.9 library on Machine B

    If your dotfiles repo predates v0.10, the library was stored as
    machine-specific symlinks. v0.10's first `tome sync` against such a
    library refuses with a Conflict / Why / Suggestion error pointing at
    the migration command:

    ```bash
    tome migrate-library --dry-run    # preview the conversion
    tome migrate-library              # run it (confirmation prompt; default no)
    ```

    The dry-run shows a summary table — count of symlinks to convert,
    approximate additional disk usage, per-skill SOURCE / SIZE / STATUS
    columns — and the live run prompts via `dialoguer::Confirm` defaulting
    to no. Pressing anything other than `y` aborts cleanly with no
    filesystem mutation.

    For CI / non-interactive automation, use `--yes` (mirrors `tome remove
    skill --yes`):

    ```bash
    tome migrate-library --yes
    ```

    Under `--no-input` without `--yes`, migration bails with a
    Conflict / Why / Suggestion error — destructive operations require
    explicit consent in non-interactive mode.

    Broken managed symlinks (target gone) are SKIPPED and preserved in
    place so you can recover manually. Idempotent on re-run; subsequent
    syncs proceed normally.

    The conversion is one-way — there is no `--undo-migrate`. Commit your
    library directory to git (or back it up some other way) BEFORE running.
    ```

    Lock the page at the file path `docs/src/cross-machine-sync.md`. The page MUST NOT contain:
    - `tome adopt` / `tome forget` as live commands (Phase 14 D-API-1/-2 vocab merge)
    - "auto-on-first-sync" or "first-sync v0.10 migration prompt" (Phase 11 D-01 supersession — migration is the one-shot CLI command)
    - "no longer configured" (UX-01 trigger phrase)
    - References to a "consolidated cache" library (v0.9 framing)
  </action>
  <verify>
    <automated>wc -l /Users/martin/dev/opensource/tome/docs/src/cross-machine-sync.md &amp;&amp; rg -c '^## ' /Users/martin/dev/opensource/tome/docs/src/cross-machine-sync.md</automated>
  </verify>
  <acceptance_criteria>
    - `test -f docs/src/cross-machine-sync.md` exits 0 (file exists)
    - `wc -l docs/src/cross-machine-sync.md` shows at least 150 lines
    - `rg -n '^# Cross-machine sync$' docs/src/cross-machine-sync.md` outputs one match (top-level title)
    - `rg -n '^## Walkthrough — Machine A' docs/src/cross-machine-sync.md` outputs one match
    - `rg -n '^## Walkthrough — Machine B' docs/src/cross-machine-sync.md` outputs one match
    - `rg -n '^## Reference — ' docs/src/cross-machine-sync.md` outputs at least 4 matches (`tome.lock` semantics, `auto_install_plugins`, `directory_overrides`, missing claude, migrating v0.9)
    - `rg -n 'tome migrate-library' docs/src/cross-machine-sync.md` outputs at least two matches (used in walkthrough + dedicated reference section)
    - `rg -n 'auto_install_plugins' docs/src/cross-machine-sync.md` outputs at least three matches (machine.toml block + reference section)
    - `rg -n 'content_hash|content.hash' docs/src/cross-machine-sync.md` outputs at least one match (drift basis callout)
    - `rg -n 'directory_overrides' docs/src/cross-machine-sync.md` outputs at least one match (PORT-01..05 cross-link)
    - `rg -n 'tome adopt|tome forget' docs/src/cross-machine-sync.md` outputs zero matches (Phase 14 D-API-1/-2 vocab merge)
    - `rg -n 'no longer configured' docs/src/cross-machine-sync.md` outputs zero matches (UX-01 trigger phrase)
    - `rg -n 'consolidated cache|first.sync v0\.10' docs/src/cross-machine-sync.md` outputs zero matches (v0.9 framing absent + Phase 11 D-01 vocab honored)
  </acceptance_criteria>
  <done>
    `docs/src/cross-machine-sync.md` exists with two numbered walkthroughs (Machine A / Machine B) followed by five reference sections. Length 150-250 lines. All locked v0.10 vocabulary used; all forbidden phrases absent.
  </done>
</task>

<task type="auto">
  <name>Task 2: Add cross-machine-sync.md to docs/src/SUMMARY.md TOC</name>
  <files>docs/src/SUMMARY.md</files>
  <read_first>
    - docs/src/SUMMARY.md (current 15-line TOC)
    - .planning/phases/16-cleanup-message-ux-docs/16-CONTEXT.md (D-DOC03-1 — placement guidance: between Configuration and Architecture)
  </read_first>
  <action>
    Insert a new line in docs/src/SUMMARY.md between the existing `Configuration` entry (line 6) and the `Development Workflow` entry (line 7). Final TOC:

    ```markdown
    # Summary

    [Introduction](introduction.md)

    - [Commands](commands.md)
    - [Configuration](configuration.md)
    - [Cross-machine sync](cross-machine-sync.md)
    - [Development Workflow](development-workflow.md)
    - [Architecture](architecture.md)
    - [Tool Landscape](tool-landscape.md)
    - [Frontmatter Compatibility](frontmatter-compatibility.md)
    - [Vercel Skills Comparison](vercel-skills-comparison.md)
    - [Test Setup](test-setup.md)
    - [Roadmap](roadmap.md)
    - [API Reference](api-reference.md)
    ```

    Reading order is now: Introduction → Commands (what the binary does) → Configuration (how to tell it what to do) → **Cross-machine sync (how to share the result across machines)** → Development Workflow → Architecture → ... — which puts the new page adjacent to the topics it builds on.

    (Note: CONTEXT.md D-DOC03-1 says "between Configuration and Architecture", which technically means between configuration.md and architecture.md, but the intervening Development Workflow entry stays where it is. This placement satisfies both — cross-machine-sync slots between Configuration and Architecture in reading order, with Development Workflow continuing to live there.)
  </action>
  <verify>
    <automated>rg -n 'cross-machine-sync.md' /Users/martin/dev/opensource/tome/docs/src/SUMMARY.md</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n '\[Cross-machine sync\]\(cross-machine-sync.md\)' docs/src/SUMMARY.md` outputs one match
    - `rg -nC 1 'Configuration' docs/src/SUMMARY.md | rg -A 1 'cross-machine-sync.md'` shows the new entry on the line directly after the Configuration entry (proves placement order)
    - `cat docs/src/SUMMARY.md` shows the new line between the existing Configuration and Development Workflow lines
  </acceptance_criteria>
  <done>
    SUMMARY.md TOC has the new cross-machine-sync.md entry positioned between Configuration and Development Workflow.
  </done>
</task>

<task type="auto">
  <name>Task 3: Add long_about to Command::Sync linking the cross-machine-sync.md page</name>
  <files>crates/tome/src/cli.rs</files>
  <read_first>
    - crates/tome/src/cli.rs (line 158-175 — current `Command::Sync` definition with `after_help`; check whether any other Command variants use `long_about` for precedent)
    - .planning/phases/16-cleanup-message-ux-docs/16-CONTEXT.md (D-DOC03-3 — link strategy; use relative path that works inside a clone since there's no published mdbook URL convention)
  </read_first>
  <action>
    **Step 1: Add `long_about` attribute to `Command::Sync` (cli.rs line 158-175).** clap's `long_about` is shown when the user runs `tome sync --help` (long-form help) and supplements `after_help`. Final shape:

    ```rust
    /// Discover, consolidate, and distribute skills
    #[command(
        long_about = "Discover, consolidate, and distribute skills.\n\n\
                       For the cross-machine library-as-dotfiles workflow — committing\n\
                       ~/.tome/ to dotfiles, bootstrapping a fresh machine, and the\n\
                       auto_install_plugins consent flow — see docs/src/cross-machine-sync.md\n\
                       (or the rendered mdbook page at the same path if you have the docs\n\
                       built locally).",
        after_help = "Examples:\n  tome sync\n  tome sync --dry-run\n  tome sync --force\n  tome sync --no-triage\n  tome sync --no-input\n  tome sync --no-install"
    )]
    Sync {
        /// Recreate all symlinks even if they appear up-to-date
        #[arg(short, long)]
        force: bool,
        /// Skip interactive triage of new/changed skills
        #[arg(long)]
        no_triage: bool,
        /// Skip auto-install/update of missing or drifted managed plugins this run.
        ///
        /// Doesn't change the persisted `auto_install_plugins` setting in
        /// `machine.toml`. Mirrors Cargo's `--frozen` / `--locked`.
        #[arg(long)]
        no_install: bool,
    },
    ```

    **Step 2: Verify the help text renders.** Run `cargo run -p tome -- sync --help` and confirm the long-form help shows the cross-machine-sync.md reference line. The doc-comment `/// Discover, consolidate, and distribute skills` becomes the short help (one-liner used in `tome --help`); `long_about` overrides the long form (used by `tome sync --help`).

    **Step 3: Audit for any other Command variants that might also benefit from a long_about cross-machine-sync.md reference.** Specifically `Command::MigrateLibrary` already has good `after_help` from Plan 16-02 — it can stay as-is or get a one-line `long_about` reference too (Claude's discretion). `Command::Init` typically points users at first-time-setup guidance; if it has space, a one-liner pointing at cross-machine-sync.md for the dotfiles workflow doesn't hurt. Skip if any of these Command variants get cluttered — DOC-03's success criterion is just that `tome sync --help` references the page, not all commands.
  </action>
  <verify>
    <automated>cargo build -p tome &amp;&amp; cargo run -p tome -- sync --help 2>&amp;1 | rg cross-machine-sync</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n 'long_about' crates/tome/src/cli.rs` outputs at least one match in the `Command::Sync` block (line numbers in the 158-175 range or wherever Sync now lives)
    - `rg -n 'cross-machine-sync' crates/tome/src/cli.rs` outputs at least one match
    - `cargo build -p tome` exits 0
    - `cargo run -p tome -- sync --help 2>&amp;1` output contains the substring "cross-machine-sync.md" (proves long_about renders correctly through clap)
    - `cargo clippy -p tome -- -D warnings` exits 0
  </acceptance_criteria>
  <done>
    `Command::Sync` carries a `long_about` attribute referencing `docs/src/cross-machine-sync.md`; `tome sync --help` renders the cross-machine-sync.md reference; build is clean.
  </done>
</task>

</tasks>

<verification>
- New file `docs/src/cross-machine-sync.md` exists with the correct structure (two walkthroughs + 5 reference sections)
- `docs/src/SUMMARY.md` TOC includes the new page in the correct position
- `tome sync --help` long-form help references the page
- All forbidden phrases absent from the new doc (verified via acceptance criteria greps)
- `cargo build -p tome` exits 0
- `make ci` passes
- mdbook build (if available) succeeds without warnings about broken links
</verification>

<success_criteria>
- DOC-03 satisfied: `docs/src/cross-machine-sync.md` exists, documents the library-as-dotfiles workflow end-to-end, is linked from SUMMARY.md and from `tome sync --help`
- D-DOC03-1 placement honored (between Configuration and Architecture in reading order)
- D-DOC03-2 structure honored (walkthroughs first, reference second)
- D-DOC03-3 linking strategy honored (SUMMARY.md + sync --help long_about both reference the page)
- All locked v0.10 vocabulary used; all forbidden phrases absent
</success_criteria>

<output>
After completion, create `.planning/phases/16-cleanup-message-ux-docs/16-05-SUMMARY.md` documenting:
- Final line count of cross-machine-sync.md
- Whether the page links to configuration.md / commands.md for cross-references rather than duplicating schema docs
- The exact long_about string committed to cli.rs (so DOC-02 / DOC-01 / future docs can reference it)
- Whether other Command variants gained long_about pointers to cross-machine-sync.md (Init, MigrateLibrary)
- Any v0.9 framing that surfaced unexpectedly during writing and how it was reframed
</output>
