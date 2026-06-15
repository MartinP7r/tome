// Synthetic 2000-skill fixture generator — Phase 26 plan 26-08 (NF-01).
//
// This is a Cargo integration test that, when invoked with the env var
// `PERF_FIXTURE_OUT=/some/stable/path`, generates a 2000-skill tome library
// at that path so the Playwright FPS bench (`60fps-search.spec.ts`) has a
// deterministic large-library fixture to drive.
//
// **Why an integration test, not a binary?** `cargo test` is the canonical
// way to build + execute a small Rust program with workspace dev-deps in
// scope (rand, tempfile, the `tome` crate with the `bindings` feature on).
// Wrapping the generator as a test gives us the right toolchain hookup for
// free; the `PERF_FIXTURE_OUT` env gate ensures it skips silently during
// `make ci` so the perf bench never runs as part of the standard test
// matrix (CLAUDE.md constraint 11).
//
// **Determinism.** The RNG is seeded with a fixed constant so every run
// produces byte-identical SKILL.md bodies. Frame-time deltas in the
// downstream Playwright bench will then reflect render-pipeline changes,
// not RNG noise.
//
// **Two artefacts ship.**
//
// 1. **A real tome library** at `${PERF_FIXTURE_OUT}/` — `library/skill-NNNN/SKILL.md`
//    plus a valid `.tome-manifest.json` + `tome.toml`. Any `cargo run -p tome
//    -- status --tome-home=${PERF_FIXTURE_OUT}` invocation will see 2000
//    skills, which sanity-checks the fixture against the real tome loader.
// 2. **`${PERF_FIXTURE_OUT}/perf-skills.json`** — a flat array of
//    DiscoveredSkill-shaped JSON objects (the exact wire shape the Tauri
//    `list_skills` command returns). The Vite `__mocks__/tauri-api-core.ts`
//    module reads this file at build time when `PERF_TEST=1`, so the
//    React render tree sees the same 2000 rows the perf bench measures
//    against — no need to spin up a real Tauri runtime in CI.
//
// **macOS-only? No.** The fixture generator itself is cross-platform
// (writes plain UTF-8 files). The downstream Playwright bench runs on
// `macos-latest` only (NF-01 / Assumption A12), but the fixture step is
// a pure file-system write and is portable.

#![cfg(test)] // belt + braces — integration tests are already cfg(test).

use std::fs;
use std::path::{Path, PathBuf};

use rand::rngs::StdRng;
// rand 0.10 moved `random_range` (and friends) from `Rng` onto the `RngExt`
// extension trait; it must be in scope at the call site.
use rand::{Rng, RngExt, SeedableRng};

use tome::SkillName; // re-exported at crate root from `discover` (D-26-02).
use tome::config::DirectoryName;
use tome::manifest::{Manifest, SkillEntry};

/// Number of synthetic skills to generate.
///
/// 2000 is the NF-01 target — large enough that virtualisation matters,
/// small enough that the fixture writes in ~30s on macos-latest and the
/// JSON projection stays under ~1MB. See PLAN.md 26-08 §"Hardware".
const SKILL_COUNT: usize = 2000;

/// SKILL.md body length range. Min 100 chars guarantees a non-trivial
/// body (so the markdown renderer isn't measured on empty input); max
/// 5000 keeps total fixture size to ~5MB. See PATTERNS.md §S-9.
const BODY_LEN_MIN: usize = 100;
const BODY_LEN_MAX: usize = 5000;

/// Seed for the RNG. Any constant works; this one is just a constant for
/// the file search-and-replace. Bumping it regenerates a new
/// deterministic fixture.
const RNG_SEED: u64 = 0x_746F_6D65_5F70_6572; // ASCII "tome_per" (u64).

/// Single name used for the configured directory entry in tome.toml AND as
/// `source_name` on every DiscoveredSkill row. Keeps fuzzy-search keys
/// realistic (the field is indexed) without inflating directory count.
const SYNTHETIC_DIR_NAME: &str = "synthetic";

#[test]
fn setup_perf_fixture() -> anyhow::Result<()> {
    // Gate: only run when an explicit output path is requested. This keeps
    // the test out of the `cargo test --all` path that `make ci` runs.
    let Ok(out) = std::env::var("PERF_FIXTURE_OUT") else {
        eprintln!(
            "setup_perf_fixture: PERF_FIXTURE_OUT not set — skipping. \
             Set PERF_FIXTURE_OUT=/tmp/tome-perf-fixture to materialise."
        );
        return Ok(());
    };
    let out = PathBuf::from(out);

    eprintln!(
        "setup_perf_fixture: generating {SKILL_COUNT} skills under {}",
        out.display()
    );
    fs::create_dir_all(&out)?;
    let library = out.join("library");
    fs::create_dir_all(&library)?;

    let mut rng = StdRng::seed_from_u64(RNG_SEED);
    let dir_name = DirectoryName::new(SYNTHETIC_DIR_NAME)?;
    let mut manifest = Manifest::default();

    // The mock's wire-shape array — collected as we go so we don't walk
    // the disk twice. Each entry mirrors the `DiscoveredSkill` serialize
    // shape from `crates/tome/src/discover.rs` (specta-typed):
    //   { name, path, source_name, origin: { kind, ... } }
    let mut perf_rows: Vec<serde_json::Value> = Vec::with_capacity(SKILL_COUNT);

    for i in 0..SKILL_COUNT {
        let name_str = format!("skill-{i:04}");
        let skill_dir = library.join(&name_str);
        fs::create_dir_all(&skill_dir)?;

        let body_len: usize = rng.random_range(BODY_LEN_MIN..BODY_LEN_MAX);
        let body = generate_lorem(&mut rng, body_len);
        let content = format!(
            "---\nname: {name_str}\ndescription: Synthetic test skill {i}\n---\n\n{body}\n"
        );
        fs::write(skill_dir.join("SKILL.md"), &content)?;

        // Real content hash so the manifest round-trips through
        // `tome::manifest::load` without an epoch_zero warning.
        let content_hash = tome::hash_directory(&skill_dir)?;

        let skill_name = SkillName::new(&name_str)?;
        let entry = SkillEntry::new(
            skill_dir.clone(),
            dir_name.clone(),
            content_hash,
            /* managed */ false,
        );
        manifest.insert(skill_name, entry);

        perf_rows.push(serde_json::json!({
            "name": name_str,
            "path": skill_dir.to_string_lossy(),
            "source_name": SYNTHETIC_DIR_NAME,
            "origin": { "kind": "local" },
        }));
    }

    // Manifest at the tome-home root (matches `manifest::save`'s contract).
    tome::manifest::save(&manifest, &out)?;

    // tome.toml — one [directories.synthetic] entry of type=directory
    // role=managed. role=managed is the cheapest sync configuration (no
    // distribution targets), which matches the bench's intent: measure
    // discovery + render path on a real-shaped library, not the full sync
    // pipeline.
    let tome_toml = format!(
        "# Auto-generated by tests/perf/synthetic_skills.rs — perf bench fixture.\n\
         # DO NOT EDIT BY HAND. Regenerate by running\n\
         #   PERF_FIXTURE_OUT=<this-dir> cargo test -p tome-desktop --test synthetic_skills\n\
         \n\
         [directories.synthetic]\n\
         type = \"directory\"\n\
         role = \"managed\"\n\
         path = \"{}\"\n",
        library.display()
    );
    fs::write(out.join("tome.toml"), tome_toml)?;

    // perf-skills.json — the Vite mock module reads this on build.
    let perf_json = serde_json::json!({
        "skills": perf_rows,
        "warnings": [],
    });
    fs::write(
        out.join("perf-skills.json"),
        serde_json::to_string(&perf_json)?,
    )?;

    // Sanity self-check before returning — fail the test loudly if any
    // file count is off rather than letting the Playwright bench fail
    // with a less precise "ListBox is empty" error.
    let written = count_skill_dirs(&library)?;
    assert_eq!(
        written, SKILL_COUNT,
        "fixture write under-counted: expected {SKILL_COUNT} skill dirs, found {written}"
    );

    // Round-trip the manifest to verify tome::manifest::load is happy
    // with what we wrote. Catches any future drift in SkillEntry's
    // serde shape that would silently break the fixture.
    let loaded = tome::manifest::load(&out)?;
    assert_eq!(
        loaded.keys().count(),
        SKILL_COUNT,
        "manifest round-trip dropped entries: wrote {SKILL_COUNT}, loaded {}",
        loaded.keys().count()
    );

    eprintln!(
        "setup_perf_fixture: wrote {SKILL_COUNT} skills, manifest, tome.toml, and \
         perf-skills.json under {}",
        out.display()
    );
    Ok(())
}

/// Walks `library_dir` and returns the count of immediate-child directories
/// containing a `SKILL.md`. Used for the post-write sanity-check; mirrors
/// what `tome::discover::discover_directory_entry` will count later.
fn count_skill_dirs(library_dir: &Path) -> anyhow::Result<usize> {
    let mut count = 0;
    for entry in fs::read_dir(library_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && path.join("SKILL.md").is_file() {
            count += 1;
        }
    }
    Ok(count)
}

/// Repeat a fixed lorem-ipsum corpus to roughly `target_len` chars,
/// inserting a few random word-level perturbations so fuzzy-search keys
/// aren't trivially identical across rows.
fn generate_lorem(rng: &mut impl Rng, target_len: usize) -> String {
    const CORPUS: &str = "Lorem ipsum dolor sit amet consectetur adipiscing elit \
        sed do eiusmod tempor incididunt ut labore et dolore magna aliqua \
        Ut enim ad minim veniam quis nostrud exercitation ullamco laboris \
        nisi ut aliquip ex ea commodo consequat Duis aute irure dolor in \
        reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla \
        pariatur Excepteur sint occaecat cupidatat non proident sunt in culpa \
        qui officia deserunt mollit anim id est laborum ";

    let mut out = String::with_capacity(target_len + 32);
    // Sprinkle 0-3 perturbation tokens so adjacent skills differ on body
    // content (not just frontmatter). Keeps fuse.js indexing realistic.
    let perturbation: u32 = rng.random_range(0..4);
    for _ in 0..perturbation {
        let tag: u32 = rng.random_range(0..10_000);
        out.push_str(&format!("token-{tag:04} "));
    }
    while out.len() < target_len {
        out.push_str(CORPUS);
    }
    out.truncate(target_len);
    out
}
