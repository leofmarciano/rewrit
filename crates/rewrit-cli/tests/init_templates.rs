use std::path::Path;
use std::process::{Command, Output, Stdio};

#[test]
fn laravel_to_encore_template_creates_working_scaffold() {
    let temp = tempfile::tempdir().expect("tempdir");
    init_laravel_to_encore(temp.path());

    for path in [
        "rewrit.toml",
        "README.rewrit.md",
        "contracts/billing/invoice.create.success.json",
        "legacy-laravel/rewrit-reference.php",
        "candidate-encore/rewrit-candidate.mjs",
    ] {
        assert!(temp.path().join(path).exists(), "missing {path}");
    }

    let readme = std::fs::read_to_string(temp.path().join("README.rewrit.md")).expect("readme");
    assert!(readme.contains("PHP SDK usage"));
    assert!(readme.contains("Node SDK usage"));
    assert!(readme.contains("vendor/bin/pest"));
    assert!(readme.contains("npm\", \"run\", \"test:rewrit"));

    if !has_all_commands(["php", "node"]) {
        return;
    }

    let output = rewrit(temp.path(), &["run", "--mode", "mirror"]);
    assert_success(&output, "rewrit run");
    assert_report_success(temp.path().join(".rewrit/reports/latest.json"));
    assert!(temp.path().join(".rewrit/reports/junit.xml").exists());

    let output = rewrit(temp.path(), &["capture", "--runtime", "legacy_laravel"]);
    assert_success(&output, "rewrit capture");
    assert!(temp
        .path()
        .join(".rewrit/baselines/legacy_laravel/current.jsonl")
        .exists());

    let output = rewrit(temp.path(), &["verify", "--runtime", "encore_ts"]);
    assert_success(&output, "rewrit verify");
    assert_report_success(temp.path().join(".rewrit/reports/latest.json"));
}

#[test]
fn laravel_to_encore_template_reports_missing_candidate_case() {
    if !has_all_commands(["php", "node"]) {
        return;
    }

    let temp = tempfile::tempdir().expect("tempdir");
    init_laravel_to_encore(temp.path());
    std::fs::write(
        temp.path().join("candidate-encore/rewrit-candidate.mjs"),
        missing_candidate_script(),
    )
    .expect("write missing candidate script");

    let output = rewrit(temp.path(), &["run", "--mode", "mirror"]);
    assert_exit(&output, 1, "rewrit run");
    let report = read_report(temp.path().join(".rewrit/reports/latest.json"));
    assert!(report["divergences"]
        .as_array()
        .unwrap()
        .iter()
        .any(|divergence| {
            divergence["kind"] == "missing_candidate_case"
                && divergence["severity"] == "blocking"
                && divergence["case_id"] == "billing.invoice.create.success"
        }));
}

#[test]
fn laravel_to_encore_template_reports_payload_path_and_hint() {
    if !has_all_commands(["php", "node"]) {
        return;
    }

    let temp = tempfile::tempdir().expect("tempdir");
    init_laravel_to_encore(temp.path());
    let candidate_path = temp.path().join("candidate-encore/rewrit-candidate.mjs");
    let candidate = std::fs::read_to_string(&candidate_path).expect("candidate");
    std::fs::write(
        &candidate_path,
        candidate.replacen("amount: \"199.90\"", "amount: 199.9", 1),
    )
    .expect("write candidate mismatch");

    let output = rewrit(temp.path(), &["run", "--mode", "mirror"]);
    assert_exit(&output, 1, "rewrit run");
    let report = read_report(temp.path().join(".rewrit/reports/latest.json"));
    let divergence = report["divergences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|divergence| divergence["kind"] == "type_mismatch")
        .expect("type mismatch");
    let path = divergence["path"].as_str().expect("path");
    assert_eq!(path, "$.value.body.amount");
    assert!(divergence["hint"].as_str().expect("hint").contains(path));
}

fn init_laravel_to_encore(cwd: &Path) {
    let output = rewrit(cwd, &["init", "--template", "laravel-to-encore"]);
    assert_success(&output, "rewrit init");
}

fn rewrit(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rewrit"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("run rewrit")
}

fn assert_success(output: &Output, command: &str) {
    assert_exit(output, 0, command);
}

fn assert_exit(output: &Output, expected: i32, command: &str) {
    assert!(
        output.status.code() == Some(expected),
        "{command} exited with {:?}, expected {expected}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_report_success(path: impl AsRef<Path>) {
    let report = read_report(path);
    assert_eq!(report["summary"]["exit_code"], 0);
    assert_eq!(report["summary"]["blocking"], 0);
}

fn read_report(path: impl AsRef<Path>) -> serde_json::Value {
    let report = std::fs::read_to_string(path).expect("report");
    serde_json::from_str(&report).expect("json report")
}

fn missing_candidate_script() -> &'static str {
    r#"const runtimeId = process.env.REWRIT_RUNTIME_ID || "encore_ts";
const caseId = "billing.invoice.create.success";

function emit(event) {
  process.stdout.write(`${JSON.stringify(event)}\n`);
}

emit({
  schema_version: "rewrit.event.v1",
  kind: "case_discovered",
  runtime_id: runtimeId,
  case: {
    id: caseId,
    suite_id: "billing",
    title: "creates invoice",
    source_location: null,
    tags: [],
    contract_ref: null,
    required: true,
  },
});
"#
}

fn has_all_commands<const N: usize>(commands: [&str; N]) -> bool {
    for command in commands {
        if !command_exists(command) {
            eprintln!("skipping runnable scaffold assertion because `{command}` is not available");
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
