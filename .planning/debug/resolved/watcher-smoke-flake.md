---
status: fixing
trigger: "Watcher smoke tests fail repeatedly on M-series macOS; need 5x reliable passes parallel and serial"
created: 2026-05-29T00:00:00Z
updated: 2026-05-29T00:00:00Z
---

## Current Focus

reasoning_checkpoint:
  hypothesis: "spawn_watcher_with_sink is fire-and-forget: it spawns a thread that creates the debouncer + registers watches asynchronously, then returns Ok(()) immediately. The test's 500ms cold-start sleep races against the bg-thread's debouncer-init work. On a loaded machine, the test's atomic-write happens BEFORE the watches are registered, so FSEvents never sees it. By the time watches register, the write is already historical and FSEvents does NOT deliver retroactive events."
  confirming_evidence:
    - "probe4 (replicates spawn pattern in test) printed '[main] writing manifest' BEFORE '[bg-thread] watches registered'"
    - "probe1/2/3 with synchronous in-test debouncer + 500ms wait reliably capture all events"
    - "real spawn_watcher_with_sink test (probe3) captures ZERO events even with 3-second post-write wait"
    - "all events captured by probe3 come AFTER the watches are registered — confirming notify does NOT replay missed events"
  falsification_test: "If I synchronously register watches on the calling thread BEFORE spawn_watcher_with_sink returns, both tests should pass 5x parallel + 5x serial."
  fix_rationale: "Move new_debouncer + .watch() calls to the calling thread (synchronous). Only the rx.recv() loop runs on a background thread. spawn_watcher_with_sink returns only after watches are registered. This is the standard pattern for notify-based libraries that need 'watcher is ready' semantics."
  blind_spots: "The 500ms cold-start sleep in the test may still be needed because FSEvents itself has cold-start latency between kfsevents_register and the kernel actually plumbing events. But the budget should be a SUFFICIENT margin (1500ms per agent's original probe), not a race against init work."

## Symptoms

expected: both tests pass reliably
actual: fail 3 of 3 attempts on user's Mac
errors: assertion failures — "expected MachinePrefs event within 2000ms" / "expected Manifest event within 2000ms"
reproduction: cargo test -p tome-desktop --test watcher_smoke
started: phase 26-06 just merged

## Eliminated

- hypothesis: H1 (spurious Library event masks user write)
  evidence: probe1 + probe2 captured manifest+machine writes cleanly with library_dir watched recursively. Library watch doesn't interfere.
  timestamp: investigation-step-2

- hypothesis: H4 (canonicalization mismatch)
  evidence: probe2 used watcher.rs's exact canonicalization scheme (canon parent + rebuild file_name) and captured all events.
  timestamp: investigation-step-2

## Evidence

- timestamp: investigation-step-1
  checked: reproduced failing tests via `cargo test -p tome-desktop --test watcher_smoke`
  found: both tests fail. 3/3 attempts.
  implication: deterministic failure on this machine, not the flaky-on-CI scenario described in plan.

- timestamp: investigation-step-2
  checked: standalone debouncer probe with same paths + 500ms cold-start
  found: events arrive within 500ms of write
  implication: notify+FSEvents work fine. Problem is in spawn_watcher_with_sink integration.

- timestamp: investigation-step-3
  checked: probe3 using real spawn_watcher_with_sink + test-level sink
  found: ZERO events delivered to sink even with 3-second post-write wait
  implication: the watcher is NEVER receiving the FSEvents, not "events delivered but classified wrong"

- timestamp: investigation-step-4
  checked: probe4 replicating spawn_watcher_with_sink pattern with eprintln in bg thread
  found: print order was '[main] writing manifest' THEN '[bg-thread] watches registered' — main thread's write happened BEFORE bg thread finished registering
  implication: ROOT CAUSE — race between bg-thread debouncer init and main-thread cold-start sleep. notify does NOT deliver retroactive events; the write is lost.

## Resolution

root_cause: "spawn_watcher_with_sink is fire-and-forget: it calls thread::spawn and returns Ok(()) immediately, BEFORE the spawned thread has created the debouncer and registered watches. The 500ms cold-start sleep in the test is timing against thread::spawn scheduling latency + debouncer init + FSEvents stream creation. On a loaded machine, that's >500ms, so the write happens before watching begins. FSEvents/notify do not deliver retroactive events, so the event is silently lost."
fix: "Refactor spawn_watcher_with_sink to construct the debouncer and register watches synchronously on the caller's thread, only spawning a bg thread for the rx.recv() loop. Function returns only AFTER watches are registered. Tests use mpsc::channel + recv_timeout for prompt wake on event arrival, and a 750ms cold-start budget (sufficient margin for FSEvents kernel readiness, not a race against init)."
verification: "5x parallel + 5x serial cargo test -p tome-desktop --test watcher_smoke — all 10 runs pass. clippy -D warnings clean. Full make test passes."
files_changed:
  - crates/tome-desktop/src/watcher.rs
  - crates/tome-desktop/tests/watcher_smoke.rs
status: resolved


## Symptoms

expected: both tests pass reliably
actual: fail 3 of 3 attempts on user's Mac
errors: assertion failures — "expected MachinePrefs event within 2000ms" / "expected Manifest event within 2000ms"
reproduction: cargo test -p tome-desktop --test watcher_smoke
started: phase 26-06 just merged

## Eliminated

## Evidence

- timestamp: initial-read
  checked: watcher.rs structure
  found: TempDir layout — tome_home (config_dir) contains library/ subdir. WatcherPaths registers config_dir as NonRecursive AND library_dir as Recursive. config_dir contains .tome-manifest.json, tome.lock. library_dir contains skills. machine_dir = config/ inside tome_home — but config/ in test is a subdirectory of tome_home.
  implication: machine_dir is `tome_home/config/` — distinct from library_dir. But config_dir (where manifest lives) == tome_home, which is the PARENT of library_dir. The recursive library watch may emit events but library_dir is below config_dir, not the same. Should be fine.

## Resolution

root_cause: 
fix: 
verification: 
files_changed: []
