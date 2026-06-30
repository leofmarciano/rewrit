use rewrit_engine::{Engine, RunMode};
use rewrit_model::{DivergenceKind, Severity};
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

#[tokio::test]
async fn command_adapter_detects_mismatch_and_missing_case() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_executable(
        &temp.path().join("reference.sh"),
        r#"#!/usr/bin/env sh
set -eu
printf '%s\n' '{"schema_version":"rewrit.event.v1","kind":"case_discovered","runtime_id":"reference","case":{"id":"billing.invoice.create.success","suite_id":"billing","title":"creates invoice","source_location":null,"tags":[],"contract_ref":null,"required":true}}'
printf '%s\n' '{"schema_version":"rewrit.event.v1","kind":"observation","case_id":"billing.invoice.create.success","runtime_id":"reference","status":"passed","value":{"kind":"json","value":{"amount":"199.90"}},"error":null,"stdout":{"text":"","truncated":false},"stderr":{"text":"","truncated":false},"exit_code":0,"duration_ms":1,"effects":[],"artifacts":[],"metadata":{}}'
printf '%s\n' '{"schema_version":"rewrit.event.v1","kind":"observation","case_id":"auth.login.invalid_password","runtime_id":"reference","status":"failed","value":null,"error":null,"stdout":{"text":"","truncated":false},"stderr":{"text":"","truncated":false},"exit_code":0,"duration_ms":1,"effects":[],"artifacts":[],"metadata":{}}'
"#,
    );
    write_executable(
        &temp.path().join("candidate.sh"),
        r#"#!/usr/bin/env sh
set -eu
printf '%s\n' '{"schema_version":"rewrit.event.v1","kind":"case_discovered","runtime_id":"candidate","case":{"id":"billing.invoice.create.success","suite_id":"billing","title":"creates invoice","source_location":null,"tags":[],"contract_ref":null,"required":true}}'
printf '%s\n' '{"schema_version":"rewrit.event.v1","kind":"observation","case_id":"billing.invoice.create.success","runtime_id":"candidate","status":"passed","value":{"kind":"json","value":{"amount":199.9}},"error":null,"stdout":{"text":"","truncated":false},"stderr":{"text":"","truncated":false},"exit_code":0,"duration_ms":1,"effects":[],"artifacts":[],"metadata":{}}'
"#,
    );
    fs::write(
        temp.path().join("rewrit.toml"),
        r#"[project]
name = "command-test"
reference = "reference"
candidate = "candidate"
reports_dir = ".rewrit/reports"
baselines_dir = ".rewrit/baselines"

[runtimes.reference]
adapter = "command"
command = ["./reference.sh"]
timeout_ms = 30000

[runtimes.candidate]
adapter = "command"
command = ["./candidate.sh"]
timeout_ms = 30000
"#,
    )
    .expect("manifest");

    let engine = Engine::from_manifest_path(temp.path().join("rewrit.toml")).expect("engine");
    let report = engine.run(RunMode::Mirror).await.expect("run");

    assert_eq!(report.summary.exit_code, 1);
    assert!(report.divergences.iter().any(|divergence| {
        divergence.kind == DivergenceKind::MissingCandidateCase
            && divergence.severity == Severity::Blocking
    }));
    assert!(report.divergences.iter().any(|divergence| {
        divergence.kind == DivergenceKind::TypeMismatch
            && divergence.severity == Severity::Blocking
            && divergence.path.as_deref() == Some("$.value.amount")
    }));
    let reproduction = report
        .divergences
        .iter()
        .find(|divergence| divergence.kind == DivergenceKind::TypeMismatch)
        .and_then(|divergence| divergence.minimal_reproduction.as_ref())
        .expect("minimal reproduction");
    assert_eq!(reproduction.command, "rewrit");
    assert_eq!(reproduction.args[0], "explain");
    assert!(reproduction
        .args
        .contains(&"billing.invoice.create.success".to_string()));
    assert_eq!(report.suites.len(), 1);
    assert_eq!(report.suites[0].suite_id, "billing");
    assert_eq!(report.suites[0].cases_compared, 1);
    assert_eq!(report.suites[0].blocking, 1);
}

#[tokio::test]
async fn command_adapter_reads_events_from_file_transport() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_executable(
        &temp.path().join("reference.sh"),
        r#"#!/usr/bin/env sh
set -eu
test "$REWRIT_PROTOCOL_INPUT" = "file"
test "$REWRIT_PROTOCOL_OUTPUT" = "file"
test -n "$REWRIT_REQUEST_PATH"
test -n "$REWRIT_EVENTS_PATH"
grep '"command":"run"' "$REWRIT_REQUEST_PATH" >/dev/null
cat > "$REWRIT_EVENTS_PATH" <<'JSON'
{"schema_version":"rewrit.event.v1","kind":"case_discovered","runtime_id":"reference","case":{"id":"billing.invoice.create.success","suite_id":"billing","title":"creates invoice","source_location":null,"tags":[],"contract_ref":null,"required":true}}
{"schema_version":"rewrit.event.v1","kind":"observation","case_id":"billing.invoice.create.success","runtime_id":"reference","status":"passed","value":{"kind":"json","value":{"amount":"199.90"}},"error":null,"stdout":{"text":"","truncated":false},"stderr":{"text":"","truncated":false},"exit_code":0,"duration_ms":1,"effects":[],"artifacts":[],"metadata":{}}
JSON
"#,
    );
    write_executable(
        &temp.path().join("candidate.sh"),
        r#"#!/usr/bin/env sh
set -eu
test "$REWRIT_PROTOCOL_INPUT" = "file"
test "$REWRIT_PROTOCOL_OUTPUT" = "file"
test -n "$REWRIT_REQUEST_PATH"
test -n "$REWRIT_EVENTS_PATH"
grep '"command":"run"' "$REWRIT_REQUEST_PATH" >/dev/null
cat > "$REWRIT_EVENTS_PATH" <<'JSON'
{"schema_version":"rewrit.event.v1","kind":"case_discovered","runtime_id":"candidate","case":{"id":"billing.invoice.create.success","suite_id":"billing","title":"creates invoice","source_location":null,"tags":[],"contract_ref":null,"required":true}}
{"schema_version":"rewrit.event.v1","kind":"observation","case_id":"billing.invoice.create.success","runtime_id":"candidate","status":"passed","value":{"kind":"json","value":{"amount":"199.90"}},"error":null,"stdout":{"text":"","truncated":false},"stderr":{"text":"","truncated":false},"exit_code":0,"duration_ms":1,"effects":[],"artifacts":[],"metadata":{}}
JSON
"#,
    );
    fs::write(
        temp.path().join("rewrit.toml"),
        r#"[project]
name = "command-file-test"
reference = "reference"
candidate = "candidate"
reports_dir = ".rewrit/reports"
baselines_dir = ".rewrit/baselines"

[runtimes.reference]
adapter = "command"
command = ["./reference.sh"]
timeout_ms = 30000

[runtimes.reference.protocol]
input = "file"
output = "file"

[runtimes.candidate]
adapter = "command"
command = ["./candidate.sh"]
timeout_ms = 30000

[runtimes.candidate.protocol]
input = "file"
output = "file"
"#,
    )
    .expect("manifest");

    let engine = Engine::from_manifest_path(temp.path().join("rewrit.toml")).expect("engine");
    let report = engine.run(RunMode::Mirror).await.expect("run");

    assert_eq!(report.summary.exit_code, 0);
    assert_eq!(report.summary.cases_compared, 1);
}

#[tokio::test]
async fn command_adapter_writes_discover_request_to_file_transport() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_executable(
        &temp.path().join("reference.sh"),
        r#"#!/usr/bin/env sh
set -eu
grep '"command":"discover"' "$REWRIT_REQUEST_PATH" >/dev/null
cat > "$REWRIT_EVENTS_PATH" <<'JSON'
{"schema_version":"rewrit.event.v1","kind":"case_discovered","runtime_id":"reference","case":{"id":"billing.invoice.create.success","suite_id":"billing","title":"creates invoice","source_location":null,"tags":[],"contract_ref":null,"required":true}}
JSON
"#,
    );
    write_executable(
        &temp.path().join("candidate.sh"),
        r#"#!/usr/bin/env sh
set -eu
cat > "$REWRIT_EVENTS_PATH" <<'JSON'
{"schema_version":"rewrit.event.v1","kind":"case_discovered","runtime_id":"candidate","case":{"id":"billing.invoice.create.success","suite_id":"billing","title":"creates invoice","source_location":null,"tags":[],"contract_ref":null,"required":true}}
JSON
"#,
    );
    fs::write(
        temp.path().join("rewrit.toml"),
        r#"[project]
name = "command-discover-file-test"
reference = "reference"
candidate = "candidate"
reports_dir = ".rewrit/reports"
baselines_dir = ".rewrit/baselines"

[runtimes.reference]
adapter = "command"
command = ["./reference.sh"]
timeout_ms = 30000

[runtimes.reference.protocol]
input = "file"
output = "file"

[runtimes.candidate]
adapter = "command"
command = ["./candidate.sh"]
timeout_ms = 30000

[runtimes.candidate.protocol]
input = "file"
output = "file"
"#,
    )
    .expect("manifest");

    let engine = Engine::from_manifest_path(temp.path().join("rewrit.toml")).expect("engine");
    let cases = engine
        .discover(Some(&rewrit_model::RuntimeId::new("reference")))
        .await
        .expect("discover");

    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0].id.as_str(), "billing.invoice.create.success");
}

#[tokio::test]
async fn command_adapter_honors_global_timeout() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_executable(
        &temp.path().join("reference.sh"),
        r#"#!/usr/bin/env sh
set -eu
sleep 2
"#,
    );
    write_executable(
        &temp.path().join("candidate.sh"),
        r#"#!/usr/bin/env sh
set -eu
sleep 2
"#,
    );
    fs::write(
        temp.path().join("rewrit.toml"),
        r#"[project]
name = "command-global-timeout-test"
reference = "reference"
candidate = "candidate"
reports_dir = ".rewrit/reports"
baselines_dir = ".rewrit/baselines"

[runner]
global_timeout_ms = 50
default_timeout_ms = 30000

[runtimes.reference]
adapter = "command"
command = ["./reference.sh"]
timeout_ms = 30000

[runtimes.candidate]
adapter = "command"
command = ["./candidate.sh"]
timeout_ms = 30000
"#,
    )
    .expect("manifest");

    let engine = Engine::from_manifest_path(temp.path().join("rewrit.toml")).expect("engine");
    let error = engine.run(RunMode::Mirror).await.expect_err("timeout");

    assert!(matches!(
        error,
        rewrit_engine::EngineError::GlobalTimeout {
            command: "run",
            timeout_ms: 50
        }
    ));
    assert_eq!(error.exit_code(), 6);
}

fn write_executable(path: &Path, contents: &str) {
    let mut file = fs::File::create(path).expect("create script");
    file.write_all(contents.as_bytes()).expect("write script");
    let mut permissions = file.metadata().expect("metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("chmod");
}
