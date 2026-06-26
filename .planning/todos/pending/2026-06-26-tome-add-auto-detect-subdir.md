---
created: 2026-06-26T13:12:05Z
title: Auto-detect --subdir in `tome add` by scanning for SKILL.md
area: general
files:
  - crates/tome/src/add.rs
---

## Problem

`tome add <url>` requires `--subdir <path>` when skills live in a subdirectory of the git
repo (e.g. `https://github.com/user/repo --subdir skills/`). The user has to know the
subdirectory path up front. If they omit it, `tome sync` discovers nothing (no SKILL.md
at repo root).

## Solution

After a shallow clone of the repo, scan the checkout for SKILL.md files. If none are
found at the repo root but one or more are found in a subdirectory:

1. Auto-set `subdir` to the common ancestor of the discovered SKILL.md files.
2. If multiple unrelated subdirs contain SKILL.md (ambiguous), surface them with a prompt
   ("Skills found in: skills/, extras/ — use which?") or accept them all as separate dirs.
3. If a single unambiguous subdir is found, apply it silently and note it in the output
   ("Detected skill subdirectory: skills/").

The existing `--subdir` flag stays; explicit always wins over auto-detected. The clone
already happens as part of `tome add` (via `git.rs`), so no extra network round-trip is
needed — just a walkdir scan over the shallow clone before writing the config entry.
