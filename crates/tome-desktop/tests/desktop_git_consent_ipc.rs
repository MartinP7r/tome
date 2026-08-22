//! End-to-end Tauri IPC coverage for the Rust-owned Git-consent continuation.
//!
//! This exercises the real `#[tauri::command]` handler against throwaway Git
//! repositories. The webview only forwards the server-issued opaque ID; the
//! test proves a mismatched ID cannot consume the pending continuation, both
//! decline and accept take their respective Git paths, and every completed
//! response releases the repository lock and managed app state.

#![cfg(target_os = "macos")]

use std::path::Path;
use std::process::Command;

use serde_json::{Value, json};
use tauri::{Manager, WebviewWindowBuilder};
use tempfile::TempDir;
use tome_desktop::{commands, sync_state::SyncState};

#[tauri::command]
async fn test_start_sync(
    app: tauri::AppHandle<tauri::test::MockRuntime>,
    state: tauri::State<'_, SyncState>,
) -> Result<tome_desktop::sync_outcome_wire::DesktopSyncOutcome, tome_desktop::error::TomeError> {
    commands::start_sync_with_runtime(app, state).await
}

#[tauri::command]
async fn test_respond_sync_git_consent(
    app: tauri::AppHandle<tauri::test::MockRuntime>,
    state: tauri::State<'_, SyncState>,
    request_id: String,
    decision: tome::repo_sync::GitConsentDecision,
) -> Result<tome_desktop::sync_outcome_wire::DesktopSyncOutcome, tome_desktop::error::TomeError> {
    commands::respond_sync_git_consent_with_runtime(app, state, request_id, decision).await
}

fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn git_stdout(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run git");
    assert!(output.status.success());
    String::from_utf8(output.stdout).expect("Git output is UTF-8")
        .trim()
        .to_owned()
}

fn invoke(
    webview: &tauri::WebviewWindow<tauri::test::MockRuntime>,
    command: &str,
    body: Value,
) -> Result<Value, Value> {
    tauri::test::get_ipc_response(
        webview,
        tauri::webview::InvokeRequest {
            cmd: command.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: "tauri://localhost".parse().expect("Tauri URL"),
            body: body.into(),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.into(),
        },
    )
    .map(|body| body.deserialize::<Value>().expect("JSON IPC response"))
}

fn consent_request(response: &Value, stage: &str) -> String {
    assert_eq!(response["kind"], "git_consent_required");
    assert_eq!(response["data"]["stage"], stage);
    response["data"]["request_id"]
        .as_str()
        .expect("server-issued opaque request ID")
        .to_owned()
}

fn assert_idle(app: &tauri::App<tauri::test::MockRuntime>, pool: &Path) {
    let state = app.state::<SyncState>();
    assert!(state.cancel.lock().expect("cancel lock").is_none());
    assert!(state.consent.lock().expect("consent lock").is_none());
    assert!(state.ready_session.lock().expect("ready session lock").is_none());
    assert!(
        !pool.join(".tome-sync.lock").exists(),
        "completed continuation must release the pool lock",
    );
}

struct EnvGuard {
    home: Option<std::ffi::OsString>,
    tome_home: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set(home: &Path, tome_home: &Path) -> Self {
        let guard = Self {
            home: std::env::var_os("HOME"),
            tome_home: std::env::var_os("TOME_HOME"),
        };
        // This test is the only process-level configuration fixture in this
        // integration target. The guard restores both values before it exits.
        unsafe {
            std::env::set_var("HOME", home);
            std::env::set_var("TOME_HOME", tome_home);
        }
        guard
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            match self.home.take() {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match self.tome_home.take() {
                Some(value) => std::env::set_var("TOME_HOME", value),
                None => std::env::remove_var("TOME_HOME"),
            }
        }
    }
}

#[test]
fn tauri_ipc_resumes_only_matching_git_consent_and_releases_state() {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    let remote = root.join("remote.git");
    let seed = root.join("seed");
    let pool = root.join("pool");
    let writer = root.join("writer");
    let library = pool.join("skills");
    let source = root.join("source");
    let home = root.join("home");
    std::fs::create_dir_all(source.join("fixture-skill")).expect("source skill dir");
    std::fs::write(
        source.join("fixture-skill/SKILL.md"),
        "---\nname: fixture-skill\n---\n# Fixture skill\n",
    )
    .expect("source skill");
    std::fs::create_dir_all(home.join(".config/tome")).expect("settings dir");

    git(root, &["init", "--bare", remote.to_str().expect("remote path")]);
    git(
        root,
        &["clone", remote.to_str().expect("remote path"), seed.to_str().expect("seed path")],
    );
    git(&seed, &["config", "user.email", "test@example.com"]);
    git(&seed, &["config", "user.name", "Test User"]);
    std::fs::create_dir_all(seed.join("machines")).expect("profiles dir");
    std::fs::write(
        seed.join("tome.toml"),
        format!("library_dir = \"{}\"\n", library.display()),
    )
    .expect("pool policy");
    std::fs::write(
        seed.join("machines/desktop.toml"),
        format!(
            "[directories.source]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            source.display()
        ),
    )
    .expect("profile");
    git(&seed, &["add", "tome.toml", "machines/desktop.toml"]);
    git(&seed, &["commit", "-m", "initial pool"]);
    git(&seed, &["push", "-u", "origin", "HEAD"]);

    git(
        root,
        &["clone", remote.to_str().expect("remote path"), pool.to_str().expect("pool path")],
    );
    std::fs::create_dir_all(&library).expect("library dir");
    git(
        root,
        &["clone", remote.to_str().expect("remote path"), writer.to_str().expect("writer path")],
    );
    git(&writer, &["config", "user.email", "test@example.com"]);
    git(&writer, &["config", "user.name", "Test User"]);
    std::fs::write(writer.join("remote-change.txt"), "remote update\n").expect("remote change");
    git(&writer, &["add", "remote-change.txt"]);
    git(&writer, &["commit", "-m", "remote update"]);
    git(&writer, &["push"]);
    std::fs::write(
        home.join(".config/tome/settings.toml"),
        "profile = \"desktop\"\ngit_sync = \"ask\"\n",
    )
    .expect("local settings");
    let _env = EnvGuard::set(&home, &pool);

    let event_builder = tauri_specta::Builder::<tauri::test::MockRuntime>::new()
        .events(tauri_specta::collect_events![tome_desktop::sink::SyncProgress]);
    let app = tauri::test::mock_builder()
        .manage(SyncState::default())
        .invoke_handler(tauri::generate_handler![
            test_start_sync,
            test_respond_sync_git_consent,
        ])
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .expect("Tauri app");
    event_builder.mount_events(&app);
    let webview = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .expect("Tauri webview");

    let local_before_decline = git_stdout(&pool, &["rev-parse", "HEAD"]);
    let pre_pull = invoke(&webview, "test_start_sync", json!({})).expect("pre-pull request");
    let pre_pull_id = consent_request(&pre_pull, "pre_pull");

    let mismatch = invoke(
        &webview,
        "test_respond_sync_git_consent",
        json!({ "requestId": "git-consent-not-the-request", "decision": "decline" }),
    );
    assert!(mismatch.is_err(), "a forged ID must not consume the continuation");

    let post_sync = invoke(
        &webview,
        "test_respond_sync_git_consent",
        json!({ "requestId": pre_pull_id, "decision": "decline" }),
    )
    .expect("declining pre-pull continues locally");
    let post_sync_id = consent_request(&post_sync, "post_sync");
    assert_eq!(git_stdout(&pool, &["rev-parse", "HEAD"]), local_before_decline);

    let declined = invoke(
        &webview,
        "test_respond_sync_git_consent",
        json!({ "requestId": post_sync_id, "decision": "decline" }),
    )
    .expect("declining publish completes locally");
    assert_eq!(declined["kind"], "completed");
    assert_idle(&app, &pool);

    let pre_pull = invoke(&webview, "test_start_sync", json!({})).expect("second pre-pull request");
    let pre_pull_id = consent_request(&pre_pull, "pre_pull");
    let post_sync = invoke(
        &webview,
        "test_respond_sync_git_consent",
        json!({ "requestId": pre_pull_id, "decision": "accept" }),
    )
    .expect("accepting pre-pull continues to publish preview");
    let post_sync_id = consent_request(&post_sync, "post_sync");
    assert_ne!(git_stdout(&pool, &["rev-parse", "HEAD"]), local_before_decline);

    let accepted = invoke(
        &webview,
        "test_respond_sync_git_consent",
        json!({ "requestId": post_sync_id, "decision": "accept" }),
    )
    .expect("accepting publication completes");
    assert_eq!(accepted["kind"], "completed");
    assert_eq!(git_stdout(&pool, &["log", "-1", "--format=%s"]), "tome sync: update shared pool");
    assert_idle(&app, &pool);
}
