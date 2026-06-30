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
        divergence.kind == DivergenceKind::OutputMismatch
            && divergence.severity == Severity::Blocking
    }));
}

fn write_executable(path: &Path, contents: &str) {
    let mut file = fs::File::create(path).expect("create script");
    file.write_all(contents.as_bytes()).expect("write script");
    let mut permissions = file.metadata().expect("metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("chmod");
}
