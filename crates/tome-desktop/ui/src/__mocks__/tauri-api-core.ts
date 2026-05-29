// Tauri API mock (axe-core/playwright a11y gate, plan 26-07 Task 3).
//
// Replaces `@tauri-apps/api/core` when `A11Y_TEST=1` (Vite alias in
// `vite.config.ts`). Returns deterministic fixture data so the React
// render tree exercises every interactive surface the a11y gate scans:
//
// - Status view: configured + library count + lockfile + machine prefs.
// - Skills view: 3 representative skills (1 managed, 2 local, 1 disabled).
// - Skill detail: realistic SKILL.md frontmatter projection.
// - Doctor report: 1 auto-fixable finding + 1 manual finding so the
//   AUTO-FIXABLE / NEEDS ATTENTION headings + FindingRow + Fix button
//   (which gates the PreviewPopover) all render.
//
// This is NOT a Tauri-runtime fidelity test — the real IPC behaviour is
// verified by the watcher integration test in plan 26-06. This mock
// only guarantees that the React render tree is well-formed for axe to
// scan.

/* eslint-disable @typescript-eslint/no-explicit-any */

// Fixture data — keep small but representative.
const STATUS_REPORT = {
  configured: true,
  library_dir: "/Users/test/.tome/skills",
  library_count: { count: 3, error: null },
  last_sync: "2026-05-29T08:00:00Z",
  directories: [
    {
      name: "claude-plugins",
      directory_type: "claude-plugins",
      role: "managed",
      role_description: "Managed (skills discovered here, read-only)",
      path: "/Users/test/.claude/plugins",
      skill_count: { count: 1, error: null },
      warnings: [],
      override_applied: false,
    },
    {
      name: "personal",
      directory_type: "directory",
      role: "synced",
      role_description:
        "Synced (skills discovered here AND distributed here)",
      path: "/Users/test/skills",
      skill_count: { count: 2, error: null },
      warnings: [],
      override_applied: false,
    },
  ],
  unowned: [],
  lockfile: { kind: "in_sync" },
  machine_prefs_summary: {
    disabled_count: 1,
    disabled_directory_count: 0,
  },
  health: { count: 2, error: null },
};

const LIST_REPORT = {
  skills: [
    {
      name: "axiom-build",
      path: "/Users/test/.claude/plugins/axiom-build",
      source_name: "claude-plugins",
      origin: {
        kind: "managed",
        registry_id: "axiom",
        version: "1.2.3",
        git_commit_sha: null,
      },
    },
    {
      name: "rust-helper",
      path: "/Users/test/skills/rust-helper",
      source_name: "personal",
      origin: { kind: "local" },
    },
    {
      name: "deprecated-skill",
      path: "/Users/test/skills/deprecated-skill",
      source_name: "personal",
      origin: { kind: "local" },
    },
  ],
  warnings: [],
};

const SKILL_DETAIL_BY_NAME: Record<string, any> = {
  "axiom-build": {
    name: "axiom-build",
    source_path: "/Users/test/.claude/plugins/axiom-build",
    content_hash:
      "abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234",
    last_sync: "2026-05-29T08:00:00Z",
    managed: true,
    disabled: false,
    frontmatter: {
      name: "axiom-build",
      description: "Build failures, Xcode, simulator, SPM",
      license: "MIT",
      compatibility: null,
      allowed_tools: null,
      metadata: "{}",
      extra: "{}",
    },
    body: "# Axiom Build\n\nHelp with iOS build issues.",
  },
};

const DOCTOR_REPORT = {
  findings: [
    {
      id: { kind: "library_broken_symlink", path: "/Users/test/.tome/skills/orphan" },
      severity: "warning",
      category: "library",
      title: "Broken library symlink",
      description:
        "The symlink at /Users/test/.tome/skills/orphan points at a directory that no longer exists.",
      repair_kind: "remove_broken_library_symlink",
      dry_run_description:
        "Remove the broken symlink at /Users/test/.tome/skills/orphan.",
    },
    {
      id: {
        kind: "skill_unparsable_frontmatter",
        skill: "deprecated-skill",
      },
      severity: "blocked",
      category: "skill",
      title: "Unparsable SKILL.md frontmatter — deprecated-skill",
      description:
        "The YAML frontmatter delimiters are missing in /Users/test/skills/deprecated-skill/SKILL.md.",
      repair_kind: null,
      dry_run_description: null,
    },
  ],
  auto_fixable_count: 1,
  manual_count: 1,
};

/** Mocked invoke — switches on command name and returns the
 *  command's wrapped `Ok` payload (matching the `typedError` wrap that
 *  bindings.ts generates). */
export async function invoke(cmd: string, _args?: any): Promise<any> {
  switch (cmd) {
    case "get_status":
      return STATUS_REPORT;
    case "list_skills":
      return LIST_REPORT;
    case "get_skill_detail": {
      const name: string = _args?.name ?? "";
      return SKILL_DETAIL_BY_NAME[name] ?? SKILL_DETAIL_BY_NAME["axiom-build"];
    }
    case "set_skill_disabled":
      return null;
    case "open_source_folder":
      return null;
    case "copy_path":
      return "/Users/test/.claude/plugins/axiom-build";
    case "get_doctor_report":
      return DOCTOR_REPORT;
    case "doctor_repair_one":
      return null;
    default:
      throw new Error(`a11y mock: unknown command '${cmd}'`);
  }
}

// Re-export the symbols `@tauri-apps/api/core` exposes that we touch.
export const transformCallback = (_cb: any) => 0;
export const Channel = class {
  onmessage: (() => void) | null = null;
};
