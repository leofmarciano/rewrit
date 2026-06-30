use std::io;
use std::path::Path;

pub fn run(template: String) -> io::Result<i32> {
    let manifest = match template.as_str() {
        "laravel-to-encore" => laravel_to_encore_manifest(),
        "django-to-rust" => django_to_rust_manifest(),
        _ => command_to_command_manifest(),
    };

    write_if_missing("rewrit.toml", manifest)?;
    std::fs::create_dir_all("contracts")?;
    std::fs::create_dir_all(".rewrit/reports")?;
    if template == "laravel-to-encore" {
        write_laravel_to_encore_template()?;
    }
    println!("created rewrit.toml using template {template}");
    Ok(0)
}

fn write_if_missing(path: impl AsRef<Path>, contents: &str) -> io::Result<()> {
    let path = path.as_ref();
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, contents)
}

fn write_laravel_to_encore_template() -> io::Result<()> {
    write_if_missing("README.rewrit.md", laravel_to_encore_readme())?;
    write_if_missing(
        "contracts/billing/invoice.create.success.json",
        laravel_to_encore_contract(),
    )?;
    write_if_missing(
        "legacy-laravel/rewrit-reference.php",
        laravel_reference_script(),
    )?;
    write_if_missing(
        "candidate-encore/rewrit-candidate.mjs",
        encore_candidate_script(),
    )
}

fn command_to_command_manifest() -> &'static str {
    r#"[project]
name = "rewrit-command-example"
reference = "reference"
candidate = "candidate"
contracts_dir = "contracts"
baselines_dir = ".rewrit/baselines"
reports_dir = ".rewrit/reports"

[runtimes.reference]
adapter = "command"
command = ["./examples/command-to-command/reference.sh"]
timeout_ms = 30000

[runtimes.candidate]
adapter = "command"
command = ["./examples/command-to-command/candidate.sh"]
timeout_ms = 30000

[[reports]]
kind = "terminal"

[[reports]]
kind = "json"
path = ".rewrit/reports/latest.json"
"#
}

fn laravel_to_encore_manifest() -> &'static str {
    r#"[project]
name = "laravel-to-encore"
reference = "legacy_laravel"
candidate = "encore_ts"
contracts_dir = "contracts"
baselines_dir = ".rewrit/baselines"
reports_dir = ".rewrit/reports"

[runtimes.legacy_laravel]
adapter = "command"
cwd = "legacy-laravel"
command = ["php", "rewrit-reference.php"]
timeout_ms = 30000

[runtimes.legacy_laravel.env]
APP_ENV = "testing"
CACHE_DRIVER = "array"
QUEUE_CONNECTION = "sync"

[runtimes.encore_ts]
adapter = "command"
cwd = "candidate-encore"
command = ["node", "rewrit-candidate.mjs"]
timeout_ms = 30000

[runtimes.encore_ts.env]
NODE_ENV = "test"

[[suites]]
id = "billing"
title = "Billing domain"
policy = "http_api_strict"
required = true

[policies.http_api_strict]
compare_stdout = false
compare_stderr = false
compare_exit_code = true
decimal_as_string = true

[policies.http_api_strict.headers]
ignore = ["date", "x-request-id", "server"]

[policies.http_api_strict.json]
ignore_paths = ["$.generated_at", "$.trace_id"]

[effects.db.maps.invoices]
target_table = "billing_invoices"

[effects.db.maps.invoices.fields]
id = "invoice_id"
customer_id = "customer_ref"
amount = "total_amount"
currency = "currency_code"
status = "state"

[[reports]]
kind = "terminal"

[[reports]]
kind = "json"
path = ".rewrit/reports/latest.json"

[[reports]]
kind = "junit"
path = ".rewrit/reports/junit.xml"
"#
}

fn laravel_to_encore_readme() -> &'static str {
    r#"# Rewrit Laravel To Encore Template

This scaffold is runnable as-is and mirrors the recommended migration shape:

```bash
rewrit run --mode mirror
rewrit capture --runtime legacy_laravel
rewrit verify --runtime encore_ts
rewrit audit
```

The generated `legacy-laravel/rewrit-reference.php` and
`candidate-encore/rewrit-candidate.mjs` files are dependency-light protocol
emitters so the template works before your real apps are wired. Replace those
commands in `rewrit.toml` with your real Pest/Vitest commands when integrating:

```toml
[runtimes.legacy_laravel]
adapter = "command"
cwd = "../legacy"
command = ["vendor/bin/pest", "--rewrit"]

[runtimes.encore_ts]
adapter = "command"
cwd = "../candidate"
command = ["npm", "run", "test:rewrit"]
```

PHP SDK usage:

```php
use Rewrit\Laravel;
use Rewrit\Rewrit;

Rewrit::case('billing.invoice.create.success', 'billing');
Laravel::observeHttpResponse($response);
Laravel::observeDbDelta('invoices', $insertedRows);
```

For Pest, call `->rewrit('billing.invoice.create.success')` on the test. For
PHPUnit, use the `Rewrit\PHPUnitCase` trait and call `$this->rewrit(...)`.

Node SDK usage:

```ts
import { test as baseTest } from 'vitest';
import { createRewritTest, observe } from '@rewrit/node/vitest-reporter';
import { encoreCase, observeHttpResponse, observeDbDelta } from '@rewrit/node/encore';

const test = createRewritTest(baseTest);

test.rewrit('billing.invoice.create.success', 'creates invoice', async () => {
  encoreCase('billing.invoice.create.success', 'billing');
  await observeHttpResponse(response);
  observeDbDelta('billing_invoices', { inserted: [row] });
  observe({ ok: true });
});
```

For Jest, import `rewrit` from `@rewrit/node/jest-reporter`. Configure Vitest or
Jest to load the Rewrit reporter when you want a runner-level doctor event.
"#
}

fn laravel_to_encore_contract() -> &'static str {
    r#"{
  "schema_version": "rewrit.contract.v1",
  "id": "billing.invoice.create.success",
  "kind": "http_case",
  "input": {
    "method": "POST",
    "path": "/api/invoices",
    "json": {
      "customer_id": "cus_123",
      "amount": "199.90",
      "currency": "BRL"
    }
  },
  "expect": {
    "status": 201,
    "json_schema": {
      "type": "object",
      "required": ["id", "amount", "currency", "status"],
      "properties": {
        "id": { "type": "string" },
        "amount": { "type": "string", "pattern": "^\\d+\\.\\d{2}$" },
        "currency": { "const": "BRL" },
        "status": { "const": "open" }
      }
    }
  },
  "policy": "http_api_strict"
}
"#
}

fn laravel_reference_script() -> &'static str {
    r#"<?php

declare(strict_types=1);

$runtimeId = getenv('REWRIT_RUNTIME_ID') ?: 'legacy_laravel';
$command = getenv('REWRIT_ADAPTER_COMMAND') ?: 'run';

function emit_rewrit_event(array $event): void
{
    $encoded = json_encode($event, JSON_THROW_ON_ERROR) . PHP_EOL;
    $eventsPath = getenv('REWRIT_EVENTS_PATH');
    if (is_string($eventsPath) && $eventsPath !== '') {
        file_put_contents($eventsPath, $encoded, FILE_APPEND | LOCK_EX);
        return;
    }
    fwrite(STDOUT, $encoded);
}

function text_value(string $value): array
{
    return ['kind' => 'string', 'value' => $value];
}

if ($command === 'doctor') {
    emit_rewrit_event([
        'schema_version' => 'rewrit.event.v1',
        'kind' => 'doctor_report',
        'runtime_id' => $runtimeId,
        'report' => ['ok' => true, 'checks' => ['php' => PHP_VERSION]],
    ]);
    exit(0);
}

$caseId = 'billing.invoice.create.success';
emit_rewrit_event([
    'schema_version' => 'rewrit.event.v1',
    'kind' => 'case_discovered',
    'runtime_id' => $runtimeId,
    'case' => [
        'id' => $caseId,
        'suite_id' => 'billing',
        'title' => 'creates invoice',
        'source_location' => null,
        'tags' => [],
        'contract_ref' => null,
        'required' => true,
    ],
]);

if ($command === 'discover') {
    exit(0);
}

emit_rewrit_event([
    'schema_version' => 'rewrit.event.v1',
    'kind' => 'observation',
    'case_id' => $caseId,
    'runtime_id' => $runtimeId,
    'status' => 'passed',
    'value' => [
        'kind' => 'object',
        'fields' => [
            'status' => ['kind' => 'integer', 'value' => '201'],
            'headers' => [
                'kind' => 'object',
                'fields' => [
                    'content-type' => text_value('application/json'),
                    'x-request-id' => text_value('legacy-request-id'),
                ],
            ],
            'body' => [
                'kind' => 'json',
                'value' => [
                    'id' => 'inv_123',
                    'amount' => '199.90',
                    'currency' => 'BRL',
                    'status' => 'open',
                ],
            ],
        ],
    ],
    'error' => null,
    'stdout' => ['text' => '', 'truncated' => false],
    'stderr' => ['text' => '', 'truncated' => false],
    'exit_code' => 0,
    'duration_ms' => 1,
    'effects' => [[
        'kind' => 'db_delta',
        'connection' => 'default',
        'table' => 'invoices',
        'inserted' => [[
            'id' => text_value('inv_123'),
            'customer_id' => text_value('cus_123'),
            'amount' => text_value('199.90'),
            'currency' => text_value('BRL'),
            'status' => text_value('open'),
        ]],
        'updated' => [],
        'deleted' => [],
    ]],
    'artifacts' => [],
    'metadata' => ['suite_id' => 'billing'],
]);
"#
}

fn encore_candidate_script() -> &'static str {
    r#"import { appendFileSync } from "node:fs";

const runtimeId = process.env.REWRIT_RUNTIME_ID || "encore_ts";
const command = process.env.REWRIT_ADAPTER_COMMAND || "run";
const caseId = "billing.invoice.create.success";

function emit(event) {
  const encoded = `${JSON.stringify(event)}\n`;
  if (process.env.REWRIT_EVENTS_PATH) {
    appendFileSync(process.env.REWRIT_EVENTS_PATH, encoded, "utf8");
    return;
  }
  process.stdout.write(encoded);
}

function text(value) {
  return { kind: "string", value };
}

if (command === "doctor") {
  emit({
    schema_version: "rewrit.event.v1",
    kind: "doctor_report",
    runtime_id: runtimeId,
    report: { ok: true, checks: { node: process.version, encore: "template" } },
  });
  process.exit(0);
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

if (command === "discover") {
  process.exit(0);
}

emit({
  schema_version: "rewrit.event.v1",
  kind: "observation",
  case_id: caseId,
  runtime_id: runtimeId,
  status: "passed",
  value: {
    kind: "object",
    fields: {
      status: { kind: "integer", value: "201" },
      headers: {
        kind: "object",
        fields: {
          "content-type": text("application/json"),
          "x-request-id": text("encore-request-id"),
        },
      },
      body: {
        kind: "json",
        value: {
          id: "inv_123",
          amount: "199.90",
          currency: "BRL",
          status: "open",
        },
      },
    },
  },
  error: null,
  stdout: { text: "", truncated: false },
  stderr: { text: "", truncated: false },
  exit_code: 0,
  duration_ms: 1,
  effects: [
    {
      kind: "db_delta",
      connection: "default",
      table: "billing_invoices",
      inserted: [
        {
          invoice_id: text("inv_123"),
          customer_ref: text("cus_123"),
          total_amount: text("199.90"),
          currency_code: text("BRL"),
          state: text("open"),
        },
      ],
      updated: [],
      deleted: [],
    },
  ],
  artifacts: [],
  metadata: { suite_id: "billing" },
});
"#
}

fn django_to_rust_manifest() -> &'static str {
    r#"[project]
name = "django-to-rust"
reference = "reference_django"
candidate = "candidate_rust"
contracts_dir = "contracts"
baselines_dir = ".rewrit/baselines"
reports_dir = ".rewrit/reports"

[runtimes.reference_django]
adapter = "command"
command = ["pytest", "--rewrit"]
timeout_ms = 30000

[runtimes.candidate_rust]
adapter = "rust:cargo_test"
command = ["cargo", "test", "--", "--nocapture"]
timeout_ms = 30000

[[reports]]
kind = "terminal"

[[reports]]
kind = "json"
path = ".rewrit/reports/latest.json"
"#
}
