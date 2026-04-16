# Phase 3: Import, Reassignment & Browse Polish - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-16
**Phase:** 03-import-reassignment-browse-polish
**Areas discussed:** tome add UX, tome reassign behavior, Browse TUI polish, Command flag consistency

---

## tome add UX

| Option | Description | Selected |
|--------|-------------|----------|
| Derive from URL | Auto-extract repo name, --name override | ✓ |
| Always prompt for name | Interactive naming | |
| Use full org/repo | user-repo format to avoid collisions | |

**User's choice:** Derive from URL
**Notes:** Simple, predictable. Override with --name flag.

### Auto-sync

| Option | Description | Selected |
|--------|-------------|----------|
| Config only | Just write config entry, user runs sync separately | ✓ |
| Auto-sync after add | Write config then trigger sync | |
| Ask user | Prompt after adding | |

**User's choice:** Config only

### URL scope

| Option | Description | Selected |
|--------|-------------|----------|
| Any git URL | Accept any URL that looks like a git repo | ✓ |
| GitHub only | Only github.com URLs | |
| GitHub + GitLab + Bitbucket | Big three platforms | |

**User's choice:** Any git URL

### Pinning

| Option | Description | Selected |
|--------|-------------|----------|
| Optional flags | --branch, --tag, --rev flags at add time | ✓ |
| Always track HEAD | No pin flags, manual config edit | |
| Interactive picker | Fetch remote refs and let user pick | |

**User's choice:** Optional flags

---

## tome reassign behavior

### Mechanics (first pass)

User asked for clarification: "what is this feature good for?" — explained use cases (changing skill ownership after directory reorganization, claiming orphans, switching from local to managed source).

User then suggested: "can we do something dynamic?" and "this could also work the other way round? customizing skills?"

### Dynamic + Bidirectional Model

| Option | Description | Selected |
|--------|-------------|----------|
| Dynamic detection | Skill exists in target → re-link; doesn't exist → copy there | ✓ |
| (Combined with bidirectional) | Works both directions: to managed AND to local | ✓ |

**User's choice:** Dynamic detection + bidirectional

### Fork Suppression

| Option | Description | Selected |
|--------|-------------|----------|
| Local copy wins | First-source-wins ordering handles it | ✓ |
| Remove from managed | Add to disabled list | |
| Prompt each sync | Ask user on conflict | |

**User's choice:** Local copy wins

### Command Surface

| Option | Description | Selected |
|--------|-------------|----------|
| Just reassign | One command for both directions | |
| Add tome fork alias | Dedicated command for copy-to-local flow | ✓ |

**User's choice:** Add tome fork alias

---

## Browse TUI Polish

### Theming

| Option | Description | Selected |
|--------|-------------|----------|
| Built-in themes | Ship 2-3 themes, select via flag or config | |
| Config-based colors | User-defined colors in tome.toml | |
| Terminal-adaptive | Auto-detect dark/light, adapt colors | ✓ |

**User's choice:** Terminal-adaptive

### Markdown Rendering

| Option | Description | Selected |
|--------|-------------|----------|
| Headers + emphasis | Headers, bold, italic, code spans, separators | ✓ |
| Full rendering | Plus tables, lists, code blocks | |
| Minimal highlighting | Just headers and code fences | |

**User's choice:** Headers + emphasis

### Fuzzy Highlighting

| Option | Description | Selected |
|--------|-------------|----------|
| List only | Highlight matches in skill name column | ✓ |
| List + preview | Highlight in both list and preview | |

**User's choice:** List only

### Scrollbar

| Option | Description | Selected |
|--------|-------------|----------|
| Only when needed | Appears when list exceeds viewport | ✓ |
| Always visible | Track always visible | |

**User's choice:** Only when needed

### Keyboard Shortcuts

| Option | Description | Selected |
|--------|-------------|----------|
| Keep as-is | No changes | |
| Add vim-style extras | G, gg, Ctrl+d/u, ? help overlay | ✓ |
| Something specific | Custom shortcuts | |

**User's choice:** Add vim-style extras

---

## Command Flag Consistency

### tome add confirmation

| Option | Description | Selected |
|--------|-------------|----------|
| No confirmation | Non-destructive, just write config | ✓ |
| Confirm like remove | Show preview, ask y/N | |

**User's choice:** No confirmation

### tome reassign/fork confirmation

| Option | Description | Selected |
|--------|-------------|----------|
| Confirm fork only | Fork copies files (confirm), reassign is metadata (no confirm) | ✓ |
| Confirm both | Both get confirmation | |
| No confirmation | Both run immediately | |

**User's choice:** Confirm for fork, not reassign

---

## Claude's Discretion

- URL parsing implementation
- Exact ANSI color values for adaptive themes
- Markdown parser choice
- Layout proportions and scrollbar style
- Help overlay design

## Deferred Ideas

None — discussion stayed within phase scope.
