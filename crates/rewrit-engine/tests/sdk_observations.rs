use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

#[test]
fn php_sdk_emits_pest_phpunit_and_laravel_observations() {
    if !command_exists("php") {
        eprintln!("skipping PHP SDK observation test because `php` is not available");
        return;
    }

    let temp = tempfile::tempdir().expect("tempdir");
    let events_path = temp.path().join("events.ndjson");
    let script_path = temp.path().join("php-sdk.php");
    std::fs::write(&script_path, php_sdk_script()).expect("write php script");

    let output = Command::new("php")
        .arg(&script_path)
        .env("REWRIT_EVENTS_PATH", &events_path)
        .env("REWRIT_RUNTIME_ID", "reference")
        .output()
        .expect("run php");
    assert_success(&output, "php sdk script");

    let events = read_events(&events_path);
    assert_observations(
        &events,
        ["sdk.php.pest", "sdk.php.phpunit", "sdk.php.laravel"],
    );
    let laravel = observation(&events, "sdk.php.laravel");
    assert_eq!(laravel["runtime_id"], "reference");
    assert_eq!(laravel["value"]["fields"]["status"]["value"], "201");
    assert_eq!(laravel["effects"][0]["kind"], "db_delta");
}

#[test]
fn node_sdk_emits_vitest_jest_and_encore_observations() {
    if !command_exists("node") {
        eprintln!("skipping Node SDK observation test because `node` is not available");
        return;
    }

    let temp = tempfile::tempdir().expect("tempdir");
    let events_path = temp.path().join("events.ndjson");
    let script_path = temp.path().join("node-sdk.mjs");
    std::fs::write(&script_path, node_sdk_script()).expect("write node script");

    let output = Command::new("node")
        .arg(&script_path)
        .env("REWRIT_EVENTS_PATH", &events_path)
        .env("REWRIT_RUNTIME_ID", "candidate")
        .output()
        .expect("run node");
    assert_success(&output, "node sdk script");

    let events = read_events(&events_path);
    assert_observations(
        &events,
        ["sdk.node.vitest", "sdk.node.jest", "sdk.node.encore"],
    );
    let encore = observation(&events, "sdk.node.encore");
    assert_eq!(encore["runtime_id"], "candidate");
    assert_eq!(encore["value"]["fields"]["status"]["value"], "201");
    assert_eq!(encore["effects"][0]["kind"], "db_delta");
}

fn php_sdk_script() -> String {
    let sdk = workspace_root().join("sdks/php/src");
    format!(
        r#"<?php

declare(strict_types=1);

require {rewrit};
require {pest};
require {phpunit};
require {laravel};

final class PestLike
{{
    use \Rewrit\PestPlugin;
}}

$pest = new PestLike();
$pest->rewrit('sdk.php.pest', 'sdk');
\Rewrit\Rewrit::observe(['runner' => 'pest']);

final class PHPUnitLike
{{
    use \Rewrit\PHPUnitCase;

    public function runRewrit(): void
    {{
        $this->rewrit('sdk.php.phpunit', 'sdk', 'phpunit case');
        $this->observeRewrit(['runner' => 'phpunit']);
    }}
}}

(new PHPUnitLike())->runRewrit();

final class HeaderBag
{{
    public function __construct(private array $headers)
    {{
    }}

    public function all(): array
    {{
        return $this->headers;
    }}
}}

final class FakeLaravelResponse
{{
    public HeaderBag $headers;

    public function __construct(private int $status, array $headers, private string $content)
    {{
        $this->headers = new HeaderBag($headers);
    }}

    public function getStatusCode(): int
    {{
        return $this->status;
    }}

    public function getContent(): string
    {{
        return $this->content;
    }}
}}

\Rewrit\Rewrit::case('sdk.php.laravel', 'sdk', 'laravel case');
\Rewrit\Laravel::observeHttpResponse(
    new FakeLaravelResponse(
        201,
        ['content-type' => ['application/json']],
        json_encode(['ok' => true], JSON_THROW_ON_ERROR),
    ),
    null,
    [
        \Rewrit\Laravel::dbDelta('invoices', [[
            'id' => 'inv_123',
            'amount' => '199.90',
        ]]),
    ],
);
"#,
        rewrit = php_require(sdk.join("Rewrit.php")),
        pest = php_require(sdk.join("PestPlugin.php")),
        phpunit = php_require(sdk.join("PHPUnitCase.php")),
        laravel = php_require(sdk.join("Laravel.php")),
    )
}

fn node_sdk_script() -> String {
    let sdk = workspace_root().join("sdks/node/src");
    format!(
        r#"import {{ observe }} from {index};
import {{ rewrit as jestRewrit }} from {jest};
import {{ createRewritTest, observe as vitestObserve }} from {vitest};
import {{ encoreCase, observeDbDelta, observeHttpResponse }} from {encore};

const runInline = (_name, fn) => fn();
const vitest = createRewritTest(runInline);

await vitest.rewrit("sdk.node.vitest", "vitest case", () => {{
  vitestObserve({{ runner: "vitest" }});
}});

await jestRewrit("sdk.node.jest", "jest case", () => {{
  observe({{ runner: "jest" }});
}}, runInline);

encoreCase("sdk.node.encore", "sdk", "encore case");
await observeHttpResponse({{
  status: 201,
  headers: {{ "content-type": "application/json" }},
  json: () => ({{ ok: true }}),
}});
observeDbDelta("billing_invoices", {{
  inserted: [{{ invoice_id: "inv_123", total_amount: "199.90" }}],
}});
"#,
        index = js_import(sdk.join("index.ts")),
        jest = js_import(sdk.join("jest-reporter.ts")),
        vitest = js_import(sdk.join("vitest-reporter.ts")),
        encore = js_import(sdk.join("encore.ts")),
    )
}

fn php_require(path: PathBuf) -> String {
    serde_json::to_string(&path.display().to_string()).expect("php require path")
}

fn js_import(path: PathBuf) -> String {
    serde_json::to_string(&format!("file://{}", path.display())).expect("js import path")
}

fn read_events(path: &Path) -> Vec<Value> {
    std::fs::read_to_string(path)
        .expect("events")
        .lines()
        .map(|line| serde_json::from_str(line).expect("event"))
        .collect()
}

fn assert_observations<const N: usize>(events: &[Value], case_ids: [&str; N]) {
    let observed = events
        .iter()
        .filter(|event| event["kind"] == "observation")
        .filter_map(|event| event["case_id"].as_str())
        .collect::<BTreeSet<_>>();

    for case_id in case_ids {
        assert!(observed.contains(case_id), "missing observation {case_id}");
    }
}

fn observation<'a>(events: &'a [Value], case_id: &str) -> &'a Value {
    events
        .iter()
        .rev()
        .find(|event| event["kind"] == "observation" && event["case_id"] == case_id)
        .unwrap_or_else(|| panic!("missing observation {case_id}"))
}

fn assert_success(output: &Output, command: &str) {
    assert!(
        output.status.success(),
        "{command} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
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
