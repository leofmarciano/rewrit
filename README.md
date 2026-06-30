# Rewrit

Rewrit is an open-source parity engine in Rust for validating that a candidate
implementation preserves the observable behavior of a reference implementation
during rewrites, migrations, refactors, and stack changes.

It does not try to deeply understand PHP, Node, Django, Laravel, Rust, Encore,
or any other runtime from inside the core. Rewrit understands a neutral
observation protocol. Framework-specific adapters translate tests, requests,
commands, jobs, or instrumented functions into canonical observations that the
Rust engine can normalize, compare, report, and turn into CI-friendly exit
codes.

> Status: WIP. This repository is under active development. The README describes
> the intended product, architecture, and protocol contract.

## Product Thesis

Rewrit validates parity inside observed, declared, and versioned contracts.

It does not promise metaphysical equivalence of an entire software system. It
answers a more useful engineering question:

```txt
For the contracts we observe and version, does the candidate behave like the
reference?
```

The core architectural decision is simple:

```txt
runtime/framework specific code
        |
        v
adapter
        |
        v
Rewrit Protocol
        |
        v
Rust engine
        |
        v
normalization
        |
        v
comparison
        |
        v
report
        |
        v
exit code for CI and agents
```

The core stays hexagonal. It does not know Laravel, Pest, Vitest, Django,
Encore, Pytest, or Cargo test. It knows only the domain model:

- case
- contract
- observation
- runtime
- policy
- divergence
- report

## Core Concepts

### Reference and Candidate

Rewrit uses `reference` and `candidate` instead of `legacy` and `new`.

- `reference`: the implementation treated as the source of truth
- `candidate`: the implementation being validated

Examples:

```txt
reference = Laravel/PHP
candidate = Encore/TypeScript

reference = Django/Python
candidate = Rust/Axum
```

This keeps the model useful for rewrites, refactors, dual-run migrations, and
comparisons between two modern implementations.

### Case

A case is one verifiable unit of behavior.

Cases can come from:

- Pest, PHPUnit, Vitest, Jest, Pytest, or Cargo tests
- HTTP requests
- CLI commands
- queue jobs
- instrumented functions
- manual contracts

The case ID is the stable link between systems.

```txt
billing.invoice.create.success
auth.login.invalid_password
orders.refund.partial
users.profile.update_email_conflict
```

### Contract

A contract declares what must be equivalent.

It can describe:

- input
- output
- expected type
- expected error
- HTTP status
- relevant headers
- database mutations
- emitted events
- queue messages
- created files
- important logs
- tolerance policy
- normalizers

Example:

```json
{
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
    },
    "effects": [
      {
        "kind": "db.insert",
        "table": "invoices",
        "fields": ["id", "customer_id", "amount", "currency", "status"]
      }
    ]
  },
  "policy": "http_api_strict"
}
```

### Observation

An observation is what a runtime produced when it executed a case.

It is richer than pass/fail. It may include canonical values, canonical errors,
stdout, stderr, exit code, duration, side effects, artifacts, and metadata.

Adapters emit observations through the Rewrit Protocol. The engine compares
reference observations against candidate observations.

### Divergence

A divergence is a classified difference.

Common divergence kinds:

- `missing_candidate_case`
- `missing_reference_case`
- `output_mismatch`
- `type_mismatch`
- `schema_mismatch`
- `error_mismatch`
- `side_effect_mismatch`
- `stdout_mismatch`
- `stderr_mismatch`
- `exit_code_mismatch`
- `timeout`
- `flaky`
- `adapter_error`
- `infra_error`
- `policy_allowed`
- `waiver_expired`

Example:

```json
{
  "kind": "type_mismatch",
  "severity": "blocking",
  "case_id": "billing.invoice.create.success",
  "path": "$.amount",
  "reference": {
    "type": "string",
    "value": "199.90"
  },
  "candidate": {
    "type": "number",
    "value": 199.9
  },
  "message": "Candidate returned number, but the contract requires decimal as string."
}
```

## Functional Flow

```txt
1. load manifest
2. validate config
3. discover cases
4. resolve bindings
5. run reference
6. run candidate
7. collect observations
8. normalize
9. validate schemas
10. compare
11. classify divergences
12. apply policies and waivers
13. write reports
14. return exit code
```

Each case moves through explicit states:

```txt
discovered
  -> bound
  -> scheduled
  -> reference_running
  -> candidate_running
  -> observed
  -> normalized
  -> compared
  -> classified
  -> reported
```

This keeps semantic mismatches, missing tests, adapter failures, timeouts, and
infrastructure errors from being flattened into a generic failure.

## Operation Modes

### Mirror Mode

Runs reference and candidate in the same execution.

```bash
rewrit run --mode mirror
```

Use it for active migrations where a stale baseline would hide risk.

### Baseline Mode

Captures a frozen reference and compares the candidate against it later.

```bash
rewrit capture --runtime reference
rewrit verify --runtime candidate
```

Use it for long migrations, faster CI, and small agent-driven iteration loops.

### Contract Mode

Runs from canonical contracts rather than existing framework tests.

```bash
rewrit verify --contracts contracts/**/*.json
```

Use it for HTTP APIs, internal services, jobs, and stable domain boundaries.

### Audit Mode

Checks whether required reference cases exist in the candidate.

```bash
rewrit audit
```

Use it to prevent silent migration gaps.

## Manifest

Rewrit projects are configured with `rewrit.toml`.

Example:

```toml
[project]
name = "billing-migration"
reference = "legacy_laravel"
candidate = "encore_ts"
contracts_dir = "contracts"
baselines_dir = ".rewrit/baselines"
reports_dir = ".rewrit/reports"

[runner]
global_timeout_ms = 120000
default_timeout_ms = 30000

[runtimes.legacy_laravel]
adapter = "command"
cwd = "../legacy"
command = ["vendor/bin/pest", "--rewrit"]
timeout_ms = 30000

[runtimes.legacy_laravel.env]
APP_ENV = "testing"
CACHE_DRIVER = "array"
QUEUE_CONNECTION = "sync"

[runtimes.encore_ts]
adapter = "command"
cwd = "../candidate"
command = ["npm", "run", "test:rewrit"]
timeout_ms = 30000

[runtimes.encore_ts.env]
NODE_ENV = "test"

[[suites]]
id = "billing"
title = "Billing domain"
source_glob = "tests/Feature/Billing/**/*.php"
policy = "http_api_strict"
required = true

[[bindings]]
case = "billing.invoice.create.success"
reference = "billing.invoice.create.success"
candidate = "billing.invoice.create.success"
required = true

[policies.http_api_strict]
compare_status = true
compare_json = true
compare_headers = true
compare_effects = true
numeric_epsilon = "0.000001"

[policies.http_api_strict.headers]
ignore = ["date", "x-request-id", "server"]

[policies.http_api_strict.json]
unordered_paths = ["$.items[*].metadata"]
ignore_paths = ["$.generated_at", "$.trace_id"]

[[normalizers]]
kind = "regex"
pattern = "\\b[0-9a-f]{32}\\b"
replacement = "<HEX32>"

[[reports]]
kind = "terminal"

[[reports]]
kind = "json"
path = ".rewrit/reports/latest.json"

[[reports]]
kind = "junit"
path = ".rewrit/reports/junit.xml"

[[reports]]
kind = "sarif"
path = ".rewrit/reports/rewrit.sarif"
```

## Adapter Protocol

The recommended adapter boundary is versioned NDJSON over stdout or files.

Why NDJSON:

- streaming
- simple in every language
- works for large monoliths
- no FFI requirement
- no long-running server requirement
- easy to inspect and debug

Example events:

```json
{"schema_version":"rewrit.event.v1","kind":"case_started","case_id":"billing.invoice.create.success","runtime_id":"reference"}
{"schema_version":"rewrit.event.v1","kind":"observation","case_id":"billing.invoice.create.success","runtime_id":"reference","status":"passed","value":{"kind":"json","value":{"status":"open","amount":"199.90"}}}
{"schema_version":"rewrit.event.v1","kind":"case_finished","case_id":"billing.invoice.create.success","runtime_id":"reference","duration_ms":82}
```

The first adapter target is intentionally generic: a command adapter that runs
any executable capable of emitting the protocol.

## Canonical Values

Type differences are a major source of migration bugs. Rewrit does not reduce
everything to raw JSON.

Examples of dangerous differences:

- PHP arrays can represent lists or maps
- JavaScript has `undefined`
- Python has `None`
- Rust has `Option`
- JSON has no native Date type
- money should not be represented as float
- HTTP headers are case-insensitive
- JSON object key order should not matter
- JSON array order usually does matter
- `null`, missing, and `undefined` are not the same thing

Rewrit's canonical value model separates concepts such as `Null`, `Absent`,
`Integer`, `Decimal`, `Float`, `DateTime`, `Bytes`, `Array`, `Object`, and raw
`Json`.

Default rule: money is not a float. Prefer canonical decimal strings such as
`"199.90"`.

## Side Effects

Parity is not only response body equality.

Rewrit models side effects such as:

- database deltas
- file changes
- outbound HTTP calls
- queue messages
- emitted events
- emails
- cache operations
- logs

Different schemas can be compared through explicit mapping, so the candidate
database does not need to be a copy of the reference database.

```toml
[effects.db.maps.invoices]
target_table = "billing_invoices"

[effects.db.maps.invoices.fields]
id = "invoice_id"
amount = "total_amount"
```

## Policies, Normalizers, and Waivers

Policies define what counts as a real difference.

Principles:

- strict by default
- tolerance must be explicit
- tolerances should be scoped by path when possible
- every applied normalizer must be visible in reports
- every waiver must have an owner, reason, and expiration date

Example waiver:

```toml
[[waivers]]
case = "billing.invoice.cancel.refund_event"
kind = "side_effect_mismatch"
reason = "Encore does not publish RefundIssued yet"
owner = "billing-platform"
expires = "2026-08-01"
issue = "BILL-4821"
```

Valid waivers do not block, but they still appear in reports. Expired waivers
block.

## Reports

Rewrit is designed for both humans and code agents.

Report targets:

- terminal
- JSON
- NDJSON
- JUnit XML
- SARIF
- HTML
- Markdown

Each blocking divergence should include enough context to locate and fix it:

- case ID
- suite
- divergence kind
- severity
- source location
- target location
- expected value
- actual value
- JSON path
- applied policy
- applied normalizers
- minimal reproduction
- suggested next action

The library does not need to use AI. It should produce clean evidence that AI
agents can act on.

## Exit Codes

The CLI should be predictable for CI:

```txt
0   success, parity reached
1   blocking divergences found
2   invalid config, manifest, or contract
3   discovery failed
4   adapter unavailable or incompatible
5   runtime execution failed
6   global timeout or cancellation
7   failed to write report or artifact
8   no cases found when cases were required
9   unsupported feature or policy
70  unexpected internal error
```

Example distinction:

```txt
candidate returned a different payload -> exit 1
PHP failed to start                   -> exit 5
rewrit.toml is invalid                -> exit 2
adapter does not speak protocol v1    -> exit 4
```

## Repository Shape

The intended workspace is organized around small crates with clear boundaries:

```txt
crates/
  rewrit-model/             canonical data model
  rewrit-core/              pure normalization, comparison, policy, validation
  rewrit-engine/            orchestration, planning, scheduling, storage
  rewrit-protocol/          versioned adapter protocol
  rewrit-report/            terminal, JSON, NDJSON, JUnit, SARIF, HTML reports
  rewrit-cli/               command-line interface
  rewrit-adapter-command/   generic command adapter
  rewrit-adapter-http/      HTTP parity adapter
  rewrit-adapter-php/       PHP/Pest/PHPUnit/Laravel integration
  rewrit-adapter-node/      Node/Vitest/Jest/Encore integration
  rewrit-adapter-python/    Python/Pytest/Django integration
  rewrit-adapter-rust/      Rust/Cargo test integration

sdks/
  php/
  node/
  python/
  rust/

docs/
  concepts/
  protocol/
  adapters/
  migrations/
  adr/

examples/
  command-to-command/
  http-to-http/
  laravel-to-encore/
  django-to-rust/
  php-to-node-monolith/
```

## Target CLI

```bash
rewrit init --template laravel-to-encore
rewrit doctor
rewrit discover
rewrit audit
rewrit capture --runtime legacy_laravel
rewrit verify --runtime encore_ts
rewrit run --mode mirror
rewrit explain billing.invoice.create.success
rewrit schema export
rewrit report open
```

## MVP Roadmap

### MVP 1: Protocol and minimal engine

- canonical model
- core comparator
- engine orchestration
- CLI
- NDJSON protocol
- command adapter
- terminal report
- JSON report
- manifest parser
- exit codes

Acceptance:

- two scripts in any language emit observations
- Rewrit compares by `case_id`
- detects pass, mismatch, missing case, and timeout
- writes JSON report
- returns the correct exit code

### MVP 2: HTTP adapter

- start and stop servers
- health checks
- request contracts
- status/header/body comparison
- header, timestamp, and ID normalization

Acceptance:

- compare a fake Laravel API with a fake Node API
- detect status differences
- detect schema differences
- detect type differences

### MVP 3: Laravel to Encore

- migration template
- minimal PHP SDK
- minimal Node SDK
- Pest integration
- Vitest integration
- required `case_id`
- missing candidate audit
- baseline mode

Acceptance:

- Laravel example generates a baseline
- Encore/TypeScript example emits equivalent observations
- Rewrit detects missing Encore tests
- Rewrit detects incompatible payloads
- Rewrit generates JUnit and JSON reports

### MVP 4: Django to Rust

- minimal Pytest plugin
- minimal Rust SDK
- basic Cargo test adapter
- HTTP-first migration guide

Acceptance:

- Django reference and Rust candidate compared through HTTP contracts
- consistent `case_id`
- reports are useful enough for agents to fix divergences

## Security Model

Rewrit executes project code. The default assumption is a trusted repository,
with guardrails:

- redact secrets in reports
- avoid leaking environment variables
- enforce timeouts
- kill process trees
- require explicit working directories
- support environment allowlists
- use per-run temporary directories
- cap stdout/stderr capture
- keep reports free of tokens and credentials

When `security.env_allowlist` is set, inherited environment variables are
cleared and only matching names are passed through. Runtime-local variables in
`[runtimes.<id>.env]` are still passed because they are explicit manifest input.
Entries may be exact names or prefix patterns ending in `*`.

```toml
[security]
env_allowlist = ["PATH", "HOME", "CI", "REWRIT_*"]
network_mode = "loopback_only"
```

Each runtime execution also receives an isolated temp directory under
`.rewrit/tmp` through `TMPDIR`, `TMP`, and `TEMP`.
Built-in HTTP runtimes enforce `network_mode = "disabled"` and
`network_mode = "loopback_only"`; command/framework adapters receive the same
mode through `REWRIT_NETWORK_MODE`.

Container sandboxing can come later. It should not block the core parity engine
MVP.

## Anti-Patterns

Avoid:

- making the core know Laravel, Django, Vitest, or any framework
- parsing human stdout as the contract
- raw string comparison as the default behavior model
- overly broad normalization
- retries that hide flakiness
- eternal waivers
- one giant adapter for all languages
- universal AST parsing
- side-effect claims without probes
- AI inside the library
- calling every problem "test failed"
- mixing infrastructure errors with semantic divergences
- unversioned protocols
- treating `null` and missing as the same thing by default
- treating money as float

## Design Summary

Rewrit is:

- Rust core
- CLI first
- protocol driven
- adapter based
- framework agnostic in the core
- explicit about contracts, policies, and waivers
- strict by default
- useful to humans, CI, and code agents

The key idea:

```txt
Do not compare frameworks. Compare canonical observations.
```

Frameworks are accents. Contracts are the language.
