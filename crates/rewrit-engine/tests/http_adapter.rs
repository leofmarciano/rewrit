use rewrit_engine::{Engine, RunMode};
use rewrit_model::{DivergenceKind, Severity};
use std::fs;
use std::net::TcpListener;
use std::path::Path;

#[tokio::test]
async fn http_adapter_detects_contract_response_mismatch() {
    let temp = tempfile::tempdir().expect("tempdir");
    let reference_port = free_port();
    let candidate_port = free_port();
    fs::create_dir_all(temp.path().join("contracts")).expect("contracts dir");
    fs::write(
        temp.path().join("contracts/invoice.json"),
        r#"{
  "schema_version": "rewrit.contract.v1",
  "id": "billing.invoice.create.success",
  "kind": "http_case",
  "input": {
    "method": "POST",
    "path": "/api/invoices",
    "json": { "amount": "199.90" }
  },
  "expect": { "status": 201 },
  "policy": "http_api_strict"
}"#,
    )
    .expect("contract");
    write_server(
        &temp.path().join("reference_server.py"),
        r#""amount": "199.90""#,
    );
    write_server(
        &temp.path().join("candidate_server.py"),
        r#""amount": 199.9"#,
    );
    fs::write(
        temp.path().join("rewrit.toml"),
        format!(
            r#"[project]
name = "http-test"
reference = "reference_http"
candidate = "candidate_http"
contracts_dir = "contracts"
reports_dir = ".rewrit/reports"
baselines_dir = ".rewrit/baselines"

[runtimes.reference_http]
adapter = "http"
timeout_ms = 30000

[runtimes.reference_http.server]
start = ["python3", "reference_server.py", "{reference_port}"]
healthcheck = "http://127.0.0.1:{reference_port}/health"

[runtimes.candidate_http]
adapter = "http"
timeout_ms = 30000

[runtimes.candidate_http.server]
start = ["python3", "candidate_server.py", "{candidate_port}"]
healthcheck = "http://127.0.0.1:{candidate_port}/health"
"#
        ),
    )
    .expect("manifest");

    let engine = Engine::from_manifest_path(temp.path().join("rewrit.toml")).expect("engine");
    let report = engine.run(RunMode::Mirror).await.expect("run");

    assert_eq!(report.summary.exit_code, 1);
    assert_eq!(report.summary.cases_compared, 1);
    assert!(report.divergences.iter().any(|divergence| {
        divergence.kind == DivergenceKind::TypeMismatch
            && divergence.severity == Severity::Blocking
            && divergence.path.as_deref() == Some("$.value.body.amount")
    }));
}

#[tokio::test]
async fn http_adapter_validates_contract_expectations_when_runtimes_match() {
    let temp = tempfile::tempdir().expect("tempdir");
    let reference_port = free_port();
    let candidate_port = free_port();
    fs::create_dir_all(temp.path().join("contracts")).expect("contracts dir");
    fs::write(
        temp.path().join("contracts/invoice.json"),
        r#"{
  "schema_version": "rewrit.contract.v1",
  "id": "billing.invoice.create.success",
  "kind": "http_case",
  "input": {
    "method": "POST",
    "path": "/api/invoices",
    "json": { "amount": "199.90" }
  },
  "expect": {
    "status": 201,
    "json_schema": {
      "type": "object",
      "required": ["amount"],
      "properties": {
        "amount": {
          "type": "string",
          "pattern": "^\\d+\\.\\d{2}$"
        }
      }
    }
  },
  "policy": "http_api_strict"
}"#,
    )
    .expect("contract");
    write_server_with_status(
        &temp.path().join("reference_server.py"),
        200,
        r#""amount": 199.9"#,
    );
    write_server_with_status(
        &temp.path().join("candidate_server.py"),
        200,
        r#""amount": 199.9"#,
    );
    fs::write(
        temp.path().join("rewrit.toml"),
        format!(
            r#"[project]
name = "http-contract-test"
reference = "reference_http"
candidate = "candidate_http"
contracts_dir = "contracts"
reports_dir = ".rewrit/reports"
baselines_dir = ".rewrit/baselines"

[runtimes.reference_http]
adapter = "http"
timeout_ms = 30000

[runtimes.reference_http.server]
start = ["python3", "reference_server.py", "{reference_port}"]
healthcheck = "http://127.0.0.1:{reference_port}/health"

[runtimes.candidate_http]
adapter = "http"
timeout_ms = 30000

[runtimes.candidate_http.server]
start = ["python3", "candidate_server.py", "{candidate_port}"]
healthcheck = "http://127.0.0.1:{candidate_port}/health"
"#
        ),
    )
    .expect("manifest");

    let engine = Engine::from_manifest_path(temp.path().join("rewrit.toml")).expect("engine");
    let report = engine.run(RunMode::Mirror).await.expect("run");

    assert_eq!(report.summary.exit_code, 1);
    assert_eq!(report.summary.cases_compared, 1);
    assert!(report.divergences.iter().any(|divergence| {
        divergence.kind == DivergenceKind::OutputMismatch
            && divergence.severity == Severity::Blocking
            && divergence.path.as_deref() == Some("$.value.status")
    }));
    assert!(report.divergences.iter().any(|divergence| {
        divergence.kind == DivergenceKind::SchemaMismatch
            && divergence.severity == Severity::Blocking
            && divergence.path.as_deref() == Some("$.value.body.amount")
    }));
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind")
        .local_addr()
        .expect("local addr")
        .port()
}

fn write_server(path: &Path, amount_field: &str) {
    write_server_with_status(path, 201, amount_field);
}

fn write_server_with_status(path: &Path, response_status: u16, amount_field: &str) {
    fs::write(
        path,
        format!(
            r#"from http.server import BaseHTTPRequestHandler, HTTPServer
import sys

class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/health":
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b"ok")
            return
        self.send_response(404)
        self.end_headers()

    def do_POST(self):
        if self.path == "/api/invoices":
            self.send_response({response_status})
            self.send_header("content-type", "application/json")
            self.end_headers()
            self.wfile.write(b'{{"id":"inv_123",{amount_field},"currency":"BRL","status":"open"}}')
            return
        self.send_response(404)
        self.end_headers()

    def log_message(self, *_args):
        return

HTTPServer(("127.0.0.1", int(sys.argv[1])), Handler).serve_forever()
"#
        ),
    )
    .expect("server");
}
