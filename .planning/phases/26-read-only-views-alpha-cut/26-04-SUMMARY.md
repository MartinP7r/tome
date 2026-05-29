---
phase: 26-read-only-views-alpha-cut
plan: 04
subsystem: tome-desktop/ui — MarkdownBody (read-only Skills detail body)
tags: [react, react-markdown, remark-gfm, vitest, allow-list, security, VIEW-04, D-08]
requires:
  - 26-03 SkillDetail.body field (cap'd at 1 MiB Rust-side)
  - "@tauri-apps/plugin-opener already in deps (used since 26-03)"
provides:
  - "MarkdownBody.tsx — react-markdown wrapper enforcing the SC#4 12-element allow-list"
  - "MarkdownBody.module.css — UI-SPEC revision-1 typography bindings (h3 → --text-body, no raw 14px)"
  - "SkillsView DetailColumn wiring: DetailHeader (fixed) + MarkdownBody (scrolls)"
  - "REQUIREMENTS.md VIEW-04 wording aligned with ROADMAP SC#4 (UI-SPEC Open Item 4 closed)"
  - "Vitest harness for the UI workspace (used here for the snapshot test; reused by 26-05/07)"
affects:
  - .planning/REQUIREMENTS.md (VIEW-04 row)
  - crates/tome-desktop/ui/package.json (+react-markdown, remark-gfm; +dev: vitest, testing-library, jsdom)
tech-stack:
  added:
    - "react-markdown ^10.1.0 (markdown renderer; allowedElements is the security primitive)"
    - "remark-gfm ^4.0.1 (GFM extensions; emitted GFM nodes are still filtered by allowedElements)"
    - "vitest ^4.1.7 (dev — first JS test framework in the repo)"
    - "@testing-library/react ^16.3.2, @testing-library/jest-dom ^6.9.1, jsdom ^25.0.1 (dev)"
  patterns:
    - "Allow-list enforcement (NOT sanitisation) — react-markdown drops disallowed elements before render"
    - "Tauri scheme guard — onClick rejects non-http(s) schemes with console.warn, openUrl handles http(s) only"
    - "CSS Module token bindings via --text-*-size/--text-*-line (matches DetailHeader pattern)"
key-files:
  created:
    - crates/tome-desktop/ui/src/components/MarkdownBody.tsx
    - crates/tome-desktop/ui/src/components/MarkdownBody.module.css
    - crates/tome-desktop/ui/vitest.config.ts
    - crates/tome-desktop/ui/src/test-setup.ts
    - crates/tome-desktop/ui/src/components/__tests__/MarkdownBody.test.tsx
  created-snapshot:
    - crates/tome-desktop/ui/src/components/__tests__/__snapshots__/MarkdownBody.test.tsx.snap
  modified:
    - crates/tome-desktop/ui/src/views/SkillsView.tsx (DetailColumn renders MarkdownBody below DetailHeader)
    - crates/tome-desktop/ui/src/views/SkillsView.module.css (detail column → non-scrolling flex; body scrolls)
    - crates/tome-desktop/ui/package.json (deps + scripts test/test:watch)
    - .planning/REQUIREMENTS.md (VIEW-04 wording)
decisions:
  - "Use react-markdown's allowedElements (not rehype-sanitize) — the allow-list is small enough that DOM-level filtering is overkill, and rehype-sanitize would add a runtime dep without changing the threat surface."
  - "CSS uses --text-*-size + --text-*-line separately (not a single --text-body shorthand) to match DetailHeader.module.css convention; the plan's verbatim CSS sample (`font: var(--text-body)`) does NOT match the actual token shape and was adapted."
  - "Detail column converted to non-scrolling flex (overflow: hidden). DetailHeader brings its own padding (UI-SPEC §DetailHeader 20px/24px); MarkdownBody brings its own padding (20px/24px) AND its own scroll. Previously the entire detail column scrolled — now only the body does. This matches UI-SPEC §Skills 'DetailHeader (fixed) above MarkdownBody (scrolls)'."
  - "Inline `a` onClick handler is the security boundary, not the Tauri opener allowlist (we don't restrict openUrl globally because openSourceFolder and existing flows already use it). Test 3 in the snapshot suite asserts openUrl is NOT called for javascript: URLs."
metrics:
  duration: "(filled in below — see Self-Check section)"
  completed: "2026-05-29"
---

# Phase 26 Plan 04: Markdown preview pane (VIEW-04) Summary

react-markdown 10 + remark-gfm 4 rendering the SKILL.md body in the SkillsView detail column with a 12-element allow-list and a Tauri opener scheme guard; Vitest harness bootstrapped with a snapshot test that covers both allow-list directions.

## Tasks Completed

| Task | Name | Commit | Files |
| ---- | ---- | ------ | ----- |
| 1    | MarkdownBody + SkillsView wire + REQUIREMENTS.md VIEW-04 | `cd86375` | MarkdownBody.{tsx,module.css}, SkillsView.{tsx,module.css}, package.json, package-lock.json, REQUIREMENTS.md |
| —    | Draft SUMMARY (incremental save per connection-resilience rule) | `2629eba` | 26-04-SUMMARY.md |
| 2    | Vitest harness + MarkdownBody snapshot + scheme-guard tests | `2eedf7c` | vitest.config.ts, src/test-setup.ts, src/components/__tests__/MarkdownBody.test.tsx, MarkdownBody.test.tsx.snap, package.json (scripts) |

## What Shipped (Task 1)

- **`MarkdownBody.tsx`** — wraps `<ReactMarkdown allowedElements={ALLOWED} remarkPlugins={[remarkGfm]} components={{a}}>`. The `ALLOWED` tuple is the 12 elements verbatim per UI-SPEC §MarkdownBody (`h1`, `h2`, `h3`, `p`, `strong`, `em`, `code`, `ul`, `ol`, `li`, `a`, `pre`). The `<a>` override is the security boundary — `event.preventDefault()` always; `/^https?:/.test(href)` gates `openUrl(href)`; everything else `console.warn`s and silently drops.
- **`MarkdownBody.module.css`** — Token bindings: `h1` → `--text-title-size` 22px / 600; `h2` → `--text-subhead-size` 16px / 600; `h3` → `--text-body-size` 13px / 600 (no raw 14px — UI-SPEC revision-1); `p`/`li` → `--text-body-size` 13px / 400; inline `code` mono `--text-small-size` 12px on `--bg-subtle` with `--radius-xs`; fenced `pre` on `--bg-subtle` with `--radius-md`. Max readable measure 720px.
- **`SkillsView` DetailColumn** — Renders `<DetailHeader … />` immediately followed by `<MarkdownBody body={detail.body} skillName={detail.name} />` inside a fragment. The detail column CSS now uses `overflow: hidden` with `display: flex; flex-direction: column;` so the header stays pinned and the body owns its own scrollbar.
- **REQUIREMENTS.md VIEW-04** — Wording replaced; the literal `browse/markdown.rs` reference is gone; D-08 / UI-SPEC Open Item 4 is closed in the same commit as the implementation.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocker] CSS token shape mismatch in plan's verbatim sample**
- **Found during:** Task 1, when writing `MarkdownBody.module.css`.
- **Issue:** The plan's CSS snippet (lines 164-174) uses `font: var(--text-title)` / `var(--text-body)` etc., but `tokens.css` only defines `--text-*-size` and `--text-*-line` separately — there is no single shorthand custom property. Following the plan verbatim would produce silently-broken CSS (`font: var(--text-title)` evaluates to `font: ;` and gets ignored, falling back to inherited defaults).
- **Fix:** Used the existing `DetailHeader.module.css` pattern — split into `font-size: var(--text-*-size, fallback)` + `font-weight` + `line-height: var(--text-*-line, fallback)`. Token bindings still match the UI-SPEC §MarkdownBody contract (22/16/13 px sizes, 600/400 weights). No semantic deviation from the spec.
- **Files modified:** `crates/tome-desktop/ui/src/components/MarkdownBody.module.css`
- **Commit:** `cd86375`

**2. [Rule 2 - Critical accessibility addition] Focus ring on links**
- **Found during:** Task 1, CSS authoring.
- **Issue:** UI-SPEC §Colour reserves `--accent` for "focus ring on every interactive element" but the plan's link styling only specifies hover-underline. Without a `:focus-visible` outline the keyboard-only path is invisible.
- **Fix:** Added `.body a:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; border-radius: var(--radius-xs); }`. Matches React Aria's default focus-ring shape used elsewhere in the codebase.
- **Files modified:** `crates/tome-desktop/ui/src/components/MarkdownBody.module.css`
- **Commit:** `cd86375`

### Authentication Gates

None.

### Checkpoints

- **Task 0 (Package legitimacy gate)** — Resolved by orchestrator with all 6 packages verified clean: `react-markdown@10.1.0`, `remark-gfm@4.0.1`, `vitest@4.1.7`, `@testing-library/react@16.3.2`, `@testing-library/jest-dom@6.9.1`, `jsdom@29.1.1` (constraint `^25` resolves to 25.x — intentional per plan). All MIT-licensed; all on their canonical upstream repos.

## What Shipped (Task 2)

- **`vitest.config.ts`** — first Vitest config in the repo. `jsdom` env, `globals: true`, `setupFiles: ['./src/test-setup.ts']`, `css: true` (so CSS Module imports don't choke the test runner). Plugin chain reuses `@vitejs/plugin-react`.
- **`src/test-setup.ts`** — single `import '@testing-library/jest-dom'` line, registers the custom matchers.
- **`src/components/__tests__/MarkdownBody.test.tsx`** — 5 tests, 1 snapshot file:
  1. **Allow-list FORWARD** (Pitfall 3 inverse): renders a fixture exercising every allowed element (h1/h2/h3, p, strong, em, inline code, ul/ol/li, link, fenced pre+code), asserts each is in the DOM at the right tag, snapshots the `<article>` HTML.
  2. **Allow-list REVERSE** (Pitfall 3): a fixture with tables, images, blockquotes, and raw `<script>`/`<div>` HTML — parsed-disallowed nodes drop element AND descendant text; raw-HTML-disallowed nodes drop the element (the security guarantee — no XSS primitive) but survive as inert escaped text. Test asserts `container.querySelector('script') === null` and `container.querySelector('article > div') === null`.
  3. **`javascript:` link scheme guard** (T-26-04-02): clicking the rendered `<a>` does NOT call `openUrl`. (react-markdown sanitises the href at parse time; our onClick is the safety-net layer for any scheme that slips through the parser.)
  4. **`mailto:` link scheme guard** (T-26-04-02): react-markdown KEEPS this href (its default URL transform allow-lists http/https/mailto/tel/irc), and our onClick regex `/^https?:/` rejects it. Confirms the click-time guard catches schemes the parser misses.
  5. **`https://` happy path** (T-26-04-02): clicking the link calls `openUrl` exactly once with the original href.
- **`package.json` scripts** — `npm test` → `vitest run` (one-shot for CI); `npm run test:watch` → `vitest` (interactive).

## Deviations from Plan (Task 2)

### Auto-fixed Issues

**3. [Rule 1 - Bug] Test 2 (allow-list reverse) — text-absence assertion is wrong for raw-HTML cases**
- **Found during:** First Vitest run after Task 2.
- **Issue:** The plan's behaviour spec for Test 2 says "raw HTML `<script>alert(1)</script>` has the element STRIPPED — text NOT present in the rendered DOM". react-markdown without `rehype-raw` does NOT parse HTML at all — `<script>` strings survive as ESCAPED TEXT nodes, not parsed `<script>` elements. So the literal text "SCRIPT_TEXT" is in the DOM (inert), but no `<script>` element is. Asserting text absence was wrong; asserting element absence is right. Fixed both assertions and added a comment explaining why this is the correct security check.
- **Files modified:** `crates/tome-desktop/ui/src/components/__tests__/MarkdownBody.test.tsx`
- **Commit:** `2eedf7c`

**4. [Rule 1 - Bug] Test 3 (javascript: link) — getByRole("link") fails on parser-blanked href**
- **Found during:** First Vitest run.
- **Issue:** react-markdown's default URL transform sanitises `javascript:` URLs to an empty href at parse time. `@testing-library`'s `getByRole("link", { name })` requires a non-empty href to recognise the element as a link role (jsdom follows ARIA's accessible-name rules). The plan's Test 3 design pre-dated that detail.
- **Fix:** Two-layer test: (a) the original `javascript:` test now queries the `<a>` element directly via `container.querySelector("article a")` and confirms react-markdown stripped the dangerous href; (b) added a `mailto:` test that proves our onClick guard rejects schemes the parser KEEPS (defence-in-depth). Both assert `openUrl` is not called.
- **Files modified:** `crates/tome-desktop/ui/src/components/__tests__/MarkdownBody.test.tsx`
- **Commit:** `2eedf7c`

## Verification Results

- `cd crates/tome-desktop/ui && npm test` → **5 passed (5)** in 712ms (jsdom env, React 19 runtime). No console.error / no React 19 deprecation warnings.
- `cd crates/tome-desktop/ui && npx tsc --noEmit` → exit 0 (covers both production and test code).
- `cargo check -p tome-desktop` → exit 0 (no Rust-side changes; build still links).
- REQUIREMENTS.md VIEW-04 text reflects SC#4 wording — verified by `rg -n "VIEW-04" .planning/REQUIREMENTS.md`.
- Manual smoke test (cargo tauri dev) — **deferred to the orchestrator's post-merge gate**. The automated tests already pin the allow-list, scheme guard, and a11y `<article aria-label>` so a real-rich SKILL.md is a visual sanity check, not a contract test.

## Threat Surface Scan

No new threat surface beyond what the plan's `<threat_model>` already enumerates (T-26-04-01..04 + T-26-04-SC). The MarkdownBody component is the entire attack surface this plan introduces, and every threat in the register has a mitigation pinned by test or by configuration (`allowedElements` excludes `<script>`; no `rehype-raw`; onClick scheme guard).

No flags to add.

## Known Stubs

None. MarkdownBody receives its body string from the real `SkillDetail.body` field (26-03), which the file watcher (26-06) refetches on external edit. No placeholder text, no empty defaults flowing to UI.

## TDD Gate Compliance

This plan's Task 2 had `tdd="true"` and the implementation of MarkdownBody happened in Task 1 (typical Phase-26 pattern: ship the component, then pin behaviour with tests). The gate sequence is:
- `feat(26-04)` — `cd86375` (implementation)
- `test(26-04)` — `2eedf7c` (behavioural pinning)

A strict RED-first reading would order these `test` → `feat`. We deliberately deviated because the plan itself orders them this way (Task 1 ships the component before Task 2 writes the test), and the snapshot test serves as the regression contract going forward. If a future plan needs a clean RED gate it can be tracked there.

## Self-Check: PASSED

- Artifacts exist: MarkdownBody.tsx, MarkdownBody.module.css, MarkdownBody.test.tsx, MarkdownBody.test.tsx.snap, vitest.config.ts, src/test-setup.ts — all FOUND.
- Commits exist: `cd86375` (feat), `2629eba` (docs draft), `2eedf7c` (test) — all FOUND in git log.
- 5 Vitest tests pass; tsc clean; cargo check clean.
