# Deferred Items

- Update legacy CLI integration fixtures that invoke normal `tome sync` without a selected profile. `make ci` currently stops at `crates/tome/tests/cli_backup.rs`; this is an intentional consequence of POOL-01's explicit-selection rule and is outside Plan 27.1-01's specified files.
