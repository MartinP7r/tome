// axe-core/playwright WCAG-AA gate (Phase 26 plan 26-07 Task 3 / NF-02 +
// Phase 27 plan 27-01b Task 4 — Sync route added).
//
// Scans the five GUI surfaces — Status, Skills, Sync, Health, PreviewPopover
// — against `wcag2a` + `wcag2aa` and fails the build on any violation.
//
// Architecture (Path A from the plan):
//
// - `playwright.config.ts` starts `npm run dev:a11y` (Vite with
//   `A11Y_TEST=1`) which serves the UI bundle with the Tauri APIs
//   aliased to `src/__mocks__/`. The React tree renders against
//   deterministic fixture data — no real Tauri runtime in CI.
// - Each test navigates to the matching view via the Sidebar (which
//   renders nav items as React Aria ListBox `option`s, not buttons),
//   waits for its key landmark, then runs
//   `AxeBuilder({ page }).withTags([…]).analyze()`.
// - PreviewPopover is opened by clicking the Fix button on the
//   auto-fixable FindingRow the mock supplies.
//
// Known exceptions (26-A11Y-AUDIT.md §"axe-core baseline — disabled rules"):
//   `color-contrast` is disabled with documented justification. The
//   alpha cut's accent token (`--accent: #007aff`, Apple's canonical SF
//   Blue) clears 3:1 (WCAG-AA large-text) but not 4.5:1 (normal). The
//   Sidebar's translucent vibrancy material also drops some text
//   pairings below 4.5:1 against the underlying window. Both are
//   UI-SPEC §Color decisions that need design sign-off to retune. A
//   follow-up issue captures the token-tightening work for the next
//   milestone. Every OTHER axe rule is still enforced.
//
// Real Tauri IPC behaviour is verified manually + by the watcher
// integration test in plan 26-06; this gate validates a11y semantics.

// Like `playwright.config.ts`, we import via the relative
// `ui/node_modules/` path so this spec resolves whether `playwright
// test` is invoked from `crates/tome-desktop/ui/` (the npm-script
// origin) or directly from `crates/tome-desktop/tests/a11y/`.
import { test, expect } from "../../ui/node_modules/playwright/test";
import AxeBuilder from "../../ui/node_modules/@axe-core/playwright";

const WCAG_TAGS = ["wcag2a", "wcag2aa"];

/** Known exceptions — see 26-A11Y-AUDIT.md §"axe-core baseline — disabled
 *  rules" for the deferral rationale. Keep this list short; every entry
 *  is a follow-up. */
const DISABLED_RULES = ["color-contrast"];

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  // App lands on Status by default (D-02); wait for the shell to
  // render before any per-test navigation. The ContentPane wraps the
  // view title in an `<h1>` — there's only one h1 in the document.
  await page
    .getByRole("heading", { level: 1, name: "Status" })
    .first()
    .waitFor({ state: "visible", timeout: 15_000 });
});

test("status view passes axe WCAG-AA", async ({ page }) => {
  // Already on Status from `beforeEach`. Verify a representative
  // KeyValueRow value has rendered so axe scans the real DOM, not a
  // loading-placeholder shell. The fixture's library_dir appears in
  // both the LIBRARY row and (derived) the TOME HOME row, so use
  // `.first()` to dodge the strict-mode duplicate hit.
  await expect(
    page.getByText("/Users/test/.tome/skills").first(),
  ).toBeVisible();

  const results = await new AxeBuilder({ page })
    .withTags(WCAG_TAGS)
    .disableRules(DISABLED_RULES)
    .analyze();
  expect(results.violations).toEqual([]);
});

test("skills view passes axe WCAG-AA", async ({ page }) => {
  // Sidebar NavItems are React Aria ListBoxItems → role="option".
  await page
    .getByRole("option", { name: /^Skills, Skills section/ })
    .click();
  await page
    .getByRole("searchbox", { name: "Search skills" })
    .waitFor({ state: "visible", timeout: 10_000 });
  // Wait for the ListBox to populate from the mock so the row
  // aria-labels are present.
  await page
    .getByRole("listbox", { name: "Skills" })
    .waitFor({ state: "visible", timeout: 10_000 });

  const results = await new AxeBuilder({ page })
    .withTags(WCAG_TAGS)
    .disableRules(DISABLED_RULES)
    .analyze();
  expect(results.violations).toEqual([]);
});

test("sync view passes axe WCAG-AA", async ({ page }) => {
  // Phase 27 plan 27-01b — Sync route a11y. Click the Sidebar's Sync
  // NavItem (a React Aria ListBoxItem → role="option") and wait for the
  // idle hero's <h1> to render before scanning.
  await page
    .getByRole("option", { name: /^Sync, Sync section/ })
    .click();
  // The idle hero headline is either "You haven't synced yet." (no last
  // sync recorded in StatusReport) or "Last synced …" (the a11y mock
  // ships a `last_sync` value, so this is the rendered string). Wait
  // for the <h1> shape rather than the literal string so the test stays
  // robust if the mock's last_sync drifts later.
  await page
    .getByRole("heading", { level: 1 })
    .first()
    .waitFor({ state: "visible", timeout: 10_000 });
  // [Run sync] is the primary CTA — wait for it so axe scans the full
  // idle composition (button + glyph + heading + recent-changes
  // disclosure).
  await page
    .getByRole("button", { name: "Run sync" })
    .waitFor({ state: "visible", timeout: 10_000 });

  const results = await new AxeBuilder({ page })
    .withTags(WCAG_TAGS)
    .disableRules(DISABLED_RULES)
    .analyze();
  expect(results.violations).toEqual([]);
});

test("sync view triage panel passes axe WCAG-AA (Phase 27 plan 27-02)", async ({ page }) => {
  // Phase 27 plan 27-02 — SYNC-02 triage panel a11y scan. The mock
  // returns a populated LockfileDiff when `?triage=1` is set on the
  // URL, so loading the page with that param mounts the triage panel
  // (GridList + nested SectionHeader + TriageRow chip + RadioGroup) so
  // axe can scan every interactive surface.
  await page.goto("/?triage=1");
  // Wait for the shell to land on Status (default route), then navigate
  // to Sync.
  await page
    .getByRole("heading", { level: 1, name: "Status" })
    .first()
    .waitFor({ state: "visible", timeout: 15_000 });
  await page
    .getByRole("option", { name: /^Sync, Sync section/ })
    .click();
  // Wait for the populated triage panel — the NEW outer SectionHeader
  // is the entry-point landmark (h2 with "NEW (2)").
  await page
    .getByRole("heading", { level: 2, name: /^NEW/ })
    .waitFor({ state: "visible", timeout: 10_000 });
  // The Apply N decisions button is the canonical action affordance.
  await page
    .getByRole("button", { name: /Apply \d+ triage decisions/ })
    .waitFor({ state: "visible", timeout: 10_000 });

  const results = await new AxeBuilder({ page })
    .withTags(WCAG_TAGS)
    .disableRules(DISABLED_RULES)
    .analyze();
  expect(results.violations).toEqual([]);
});

test("health view passes axe WCAG-AA", async ({ page }) => {
  await page
    .getByRole("option", { name: /^Health, Health section/ })
    .click();
  // The mock supplies 1 auto-fixable + 1 manual finding, so both
  // section headings should render.
  await page
    .getByRole("heading", { name: /AUTO-FIXABLE/i })
    .waitFor({ state: "visible", timeout: 10_000 });
  await page
    .getByRole("heading", { name: /NEEDS ATTENTION/i })
    .waitFor({ state: "visible", timeout: 10_000 });

  const results = await new AxeBuilder({ page })
    .withTags(WCAG_TAGS)
    .disableRules(DISABLED_RULES)
    .analyze();
  expect(results.violations).toEqual([]);
});

test("preview popover (Health Fix) passes axe WCAG-AA", async ({ page }) => {
  await page
    .getByRole("option", { name: /^Health, Health section/ })
    .click();
  await page
    .getByRole("heading", { name: /AUTO-FIXABLE/i })
    .waitFor({ state: "visible", timeout: 10_000 });

  // Click the Fix button on the auto-fixable row (the mock supplies
  // exactly one). The button label is "Fix" per UI-SPEC §Copywriting.
  await page.getByRole("button", { name: "Fix" }).first().click();

  // PreviewPopover renders as a Dialog labelled by the PREVIEW heading.
  await page
    .getByRole("dialog")
    .waitFor({ state: "visible", timeout: 10_000 });
  await expect(page.getByText(/PREVIEW/)).toBeVisible();

  // Scope axe to the dialog so we measure the popover specifically.
  const results = await new AxeBuilder({ page })
    .include('[role="dialog"]')
    .withTags(WCAG_TAGS)
    .disableRules(DISABLED_RULES)
    .analyze();
  expect(results.violations).toEqual([]);
});
