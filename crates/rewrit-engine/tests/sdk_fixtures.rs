use rewrit_engine::{Engine, RunMode};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[tokio::test]
async fn laravel_to_encore_fixture_runs_end_to_end() {
    if !has_all_commands(["php", "node"]) {
        return;
    }
    assert_fixture_passes("examples/laravel-to-encore/rewrit.toml").await;
}

#[tokio::test]
async fn django_to_rust_fixture_runs_end_to_end() {
    if !has_all_commands(["python3", "cargo"]) {
        return;
    }
    assert_fixture_passes("examples/django-to-rust/rewrit.toml").await;
}

#[tokio::test]
async fn php_to_node_fixture_runs_end_to_end() {
    if !has_all_commands(["php", "node"]) {
        return;
    }
    assert_fixture_passes("examples/php-to-node-monolith/rewrit.toml").await;
}

async fn assert_fixture_passes(manifest: &str) {
    let manifest_path = workspace_root().join(manifest);
    let engine = Engine::from_manifest_path(&manifest_path).expect("engine");
    let report = engine.run(RunMode::Mirror).await.expect("run");
    assert_eq!(report.summary.exit_code, 0, "run failed for {manifest}");

    let engine = Engine::from_manifest_path(&manifest_path).expect("engine");
    let report = engine
        .verify_contracts(&["contracts/**/*.json".to_string()])
        .await
        .expect("verify contracts");
    assert_eq!(
        report.summary.exit_code, 0,
        "contract verify failed for {manifest}"
    );
}

fn has_all_commands<const N: usize>(commands: [&str; N]) -> bool {
    for command in commands {
        if !command_exists(command) {
            eprintln!("skipping SDK fixture e2e because `{command}` is not available");
            return false;
        }
    }
    true
}

fn command_exists(command: &str) -> bool {
    Command::new(command)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}
