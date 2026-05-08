//! Integration smoke tests for `tome browse`.
//!
//! The browse command is interactive (ratatui TUI) — its behaviour cannot be
//! driven from `assert_cmd::Command::write_stdin` without a PTY. The
//! interactive flows are covered by unit tests in `crates/tome/src/browse/`
//! (state transitions, fuzzy matching, theming, etc.).
//!
//! HARD-12 (Plan 15-05) lands `ratatui::TestBackend` + `insta` snapshot tests
//! for the rendering layer. This file exists today as the destination for any
//! end-to-end smoke tests that don't require an interactive TTY (e.g.
//! "browse against an empty library prints the no-skills hint and exits 0",
//! once HARD-12 wires that path).

#![allow(unused_imports)]

mod common;
use common::*;
